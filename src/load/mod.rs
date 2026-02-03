// Load - 自動ロード実装
//
// _load 設定に従って各種ソースからデータをロードする。

use crate::ports::required::{
    APIClient, DBClient, DBConnectionConfigConverter, ENVClient, ExpressionClient, KVSClient,
    ProcessMemoryClient,
};
use crate::common::PlaceholderResolver;
use serde_json::Value;
use std::collections::HashMap;

/// Load - 自動ロード専用
///
/// _load メタデータに従って各種clientからデータを取得する。
pub struct Load<'a> {
    db_client: Option<&'a dyn DBClient>,
    kvs_client: Option<&'a dyn KVSClient>,
    process_memory: Option<&'a dyn ProcessMemoryClient>,
    env_client: Option<&'a dyn ENVClient>,
    api_client: Option<&'a dyn APIClient>,
    expression_client: Option<&'a dyn ExpressionClient>,
    db_config_converter: Option<&'a dyn DBConnectionConfigConverter>,
    recursion_depth: usize,
    max_recursion: usize,
}

impl<'a> Load<'a> {
    /// 新しいLoadインスタンスを作成
    pub fn new() -> Self {
        Self {
            db_client: None,
            kvs_client: None,
            process_memory: None,
            env_client: None,
            api_client: None,
            expression_client: None,
            db_config_converter: None,
            recursion_depth: 0,
            max_recursion: 10,
        }
    }

    /// DBClientを設定
    pub fn with_db_client(mut self, client: &'a dyn DBClient) -> Self {
        self.db_client = Some(client);
        self
    }

    /// KVSClientを設定
    pub fn with_kvs_client(mut self, client: &'a dyn KVSClient) -> Self {
        self.kvs_client = Some(client);
        self
    }

    /// ProcessMemoryClientを設定
    pub fn with_process_memory(mut self, client: &'a dyn ProcessMemoryClient) -> Self {
        self.process_memory = Some(client);
        self
    }

    /// ENVClientを設定
    pub fn with_env_client(mut self, client: &'a dyn ENVClient) -> Self {
        self.env_client = Some(client);
        self
    }

    /// APIClientを設定
    pub fn with_api_client(mut self, client: &'a dyn APIClient) -> Self {
        self.api_client = Some(client);
        self
    }

    /// ExpressionClientを設定
    pub fn with_expression_client(mut self, client: &'a dyn ExpressionClient) -> Self {
        self.expression_client = Some(client);
        self
    }

    /// DBConnectionConfigConverterを設定
    pub fn with_db_config_converter(mut self, converter: &'a dyn DBConnectionConfigConverter) -> Self {
        self.db_config_converter = Some(converter);
        self
    }

    /// _load 設定に従ってデータをロード
    ///
    /// # Arguments
    /// * `context_key` - 現在のコンテキストキー（例: "cache.user"）
    /// * `load_config` - _load メタデータ
    /// * `state_resolver` - state.get() への callback（自己再帰用）
    ///
    /// # Returns
    /// * `Ok(Value)` - ロード成功
    /// * `Err(String)` - ロード失敗
    pub fn handle<F>(
        &mut self,
        context_key: &str,
        load_config: &HashMap<String, Value>,
        state_resolver: F,
    ) -> Result<Value, String>
    where
        F: Fn(&str) -> Option<Value>,
    {
        // 再帰深度チェック
        if self.recursion_depth >= self.max_recursion {
            return Err(format!(
                "Load::handle: max recursion depth ({}) reached",
                self.max_recursion
            ));
        }

        self.recursion_depth += 1;

        let result = self.handle_internal(context_key, load_config, &state_resolver);

        self.recursion_depth -= 1;

        result
    }

    fn handle_internal<F>(
        &mut self,
        context_key: &str,
        load_config: &HashMap<String, Value>,
        state_resolver: &F,
    ) -> Result<Value, String>
    where
        F: Fn(&str) -> Option<Value>,
    {
        use crate::common::PlaceholderResolver;
        use crate::state::parameter_builder::ParameterBuilder;

        let client = load_config
            .get("client")
            .and_then(|v| v.as_str())
            .ok_or("Load::handle: 'client' not found in _load config")?;

        // load_config 内の placeholder を型付きで解決
        let resolver = |placeholder_name: &str| {
            ParameterBuilder::resolve_placeholder(placeholder_name, context_key, state_resolver)
        };

        let resolved_config_value = Value::Mapping(
            load_config
                .iter()
                .map(|(k, v)| (Value::String(k.clone()), v.clone()))
                .collect(),
        );

        let resolved_config_value =
            PlaceholderResolver::resolve_typed(resolved_config_value, &resolver);

        let resolved_config: HashMap<String, Value> = if let Value::Mapping(m) =
            resolved_config_value
        {
            m.into_iter()
                .map(|(k, v)| {
                    if let Value::String(key) = k {
                        (key, v)
                    } else {
                        (String::new(), v) // fallback
                    }
                })
                .collect()
        } else {
            return Err("Load::handle: failed to resolve config".to_string());
        };

        match client {
            "Env" | "ENV" => self.load_from_env(&resolved_config),
            "InMemory" => self.load_from_process_memory(&resolved_config),
            "KVS" => self.load_from_kvs(&resolved_config),
            "DB" => self.load_from_db(&resolved_config),
            "API" => self.load_from_api(&resolved_config),
            "EXPRESSION" => self.load_from_expression(&resolved_config),
            _ => Err(format!("Load::handle: unsupported client '{}'", client)),
        }
    }

    /// 環境変数から読み込み
    fn load_from_env(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        let env_client = self
            .env_client
            .ok_or("Load::load_from_env: ENVClient not configured")?;

        let map = config
            .get("map")
            .and_then(|v| v.as_object())
            .ok_or("Load::load_from_env: 'map' not found")?;

        let mut result = serde_json::Map::new();

        for (config_key, env_key_value) in map {
            if let Some(env_key) = env_key_value.as_str() {
                if let Some(value) = env_client.get(env_key) {
                    result.insert(config_key.clone(), Value::String(value));
                }
            }
        }

        Ok(Value::Object(result))
    }

    /// ProcessMemoryから読み込み
    fn load_from_process_memory(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        let process_memory = self
            .process_memory
            .ok_or("Load::load_from_process_memory: ProcessMemoryClient not configured")?;

        let key = config
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or("Load::load_from_process_memory: 'key' not found")?;

        // placeholder はすでに resolved_config で解決済み
        process_memory
            .get(key)
            .ok_or_else(|| format!("Load::load_from_process_memory: key '{}' not found", key))
    }

    /// KVSから読み込み
    fn load_from_kvs(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        let kvs_client = self
            .kvs_client
            .ok_or("Load::load_from_kvs: KVSClient not configured")?;

        let key = config
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or("Load::load_from_kvs: 'key' not found")?;

        // placeholder はすでに resolved_config で解決済み
        kvs_client
            .get(key)
            .ok_or_else(|| format!("Load::load_from_kvs: key '{}' not found", key))
    }

    /// DBから読み込み
    fn load_from_db(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        let db_client = self
            .db_client
            .ok_or("Load::load_from_db: DBClient not configured")?;

        let db_config_converter = self
            .db_config_converter
            .ok_or("Load::load_from_db: DBConnectionConfigConverter not configured")?;

        let table = config
            .get("table")
            .and_then(|v| v.as_str())
            .ok_or("Load::load_from_db: 'table' not found")?;

        let where_clause = config.get("where").and_then(|v| v.as_str());

        let map = config
            .get("map")
            .and_then(|v| v.as_object())
            .ok_or("Load::load_from_db: 'map' not found")?;

        // connection 解決: config.connection の値は placeholder 解決済み
        // connection が Value::Object なら直接使用、Value::String なら error
        let connection_config = if let Some(conn_value) = config.get("connection") {
            // placeholder 解決後は Object になっているはず
            if let Some(conn_map) = conn_value.as_object() {
                db_config_converter
                    .to_config(conn_map)
                    .ok_or("Load::load_from_db: failed to convert connection config")?
            } else {
                return Err(format!(
                    "Load::load_from_db: connection must be an object after resolution, got: {:?}",
                    conn_value
                ));
            }
        } else {
            return Err("Load::load_from_db: 'connection' not specified".to_string());
        };

        let row = db_client
            .fetch_one(&connection_config, table, where_clause)
            .ok_or_else(|| format!("Load::load_from_db: no data found in table '{}'", table))?;

        // mapに従ってフィールドをマッピング
        let mut result = serde_json::Map::new();
        for (config_key, db_column_value) in map {
            if let Some(db_column) = db_column_value.as_str() {
                if let Some(value) = row.get(db_column) {
                    result.insert(config_key.clone(), value.clone());
                }
            }
        }

        Ok(Value::Object(result))
    }

    /// APIから読み込み
    fn load_from_api(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        let api_client = self
            .api_client
            .ok_or("Load::load_from_api: APIClient not configured")?;

        let url = config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or("Load::load_from_api: 'url' not found")?;

        // placeholder はすでに resolved_config で解決済み

        // headers処理（optional）
        let headers = config.get("headers").and_then(|v| v.as_object()).map(|h| {
            h.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect::<HashMap<String, String>>()
        });

        api_client.get(url, headers.as_ref())
    }

    /// EXPRESSIONから読み込み
    fn load_from_expression(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        let expression_client = self
            .expression_client
            .ok_or("Load::load_from_expression: ExpressionClient not configured")?;

        let expression = config
            .get("expression")
            .and_then(|v| v.as_str())
            .ok_or("Load::load_from_expression: 'expression' not found")?;

        // placeholder はすでに resolved_config で解決済み
        expression_client.evaluate(expression)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock ENVClient
    struct MockENVClient;
    impl ENVClient for MockENVClient {
        fn get(&self, key: &str) -> Option<String> {
            match key {
                "DB_HOST" => Some("localhost".to_string()),
                "DB_PORT" => Some("5432".to_string()),
                _ => None,
            }
        }
    }

    #[test]
    fn test_load_from_env() {
        let env_client = MockENVClient;
        let mut load = Load::new().with_env_client(&env_client);

        let mut config = HashMap::new();
        config.insert("client".to_string(), Value::String("Env".to_string()));

        let mut map = serde_json::Map::new();
        map.insert("host".to_string(), Value::String("DB_HOST".to_string()));
        map.insert("port".to_string(), Value::String("DB_PORT".to_string()));
        config.insert("map".to_string(), Value::Object(map));

        let params = HashMap::new();
        let result = load.handle(&config, &params).unwrap();

        assert_eq!(result.get("host"), Some(&Value::String("localhost".to_string())));
        assert_eq!(result.get("port"), Some(&Value::String("5432".to_string())));
    }

    #[test]
    fn test_load_recursion_limit() {
        let mut load = Load::new();

        let mut config = HashMap::new();
        config.insert("client".to_string(), Value::String("Env".to_string()));

        let params = HashMap::new();

        // 再帰深度を超える
        for _ in 0..11 {
            load.recursion_depth += 1;
        }

        let result = load.handle(&config, &params);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max recursion depth"));
    }
}
