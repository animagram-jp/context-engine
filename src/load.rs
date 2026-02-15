use crate::ports::required::{
    DBClient, ENVClient, KVSClient,
    InMemoryClient,
};
use serde_json::Value;
use std::collections::HashMap;

/// Load - 自動ロード専用
///
/// _load メタデータに従って各種clientからデータを取得する。
/// 再帰制御は Resolver が担当。
pub struct Load<'a> {
    db_client: Option<&'a dyn DBClient>,
    kvs_client: Option<&'a dyn KVSClient>,
    in_memory: Option<&'a dyn InMemoryClient>,
    env_client: Option<&'a dyn ENVClient>,
}

impl<'a> Load<'a> {
    /// 新しいLoadインスタンスを作成
    pub fn new() -> Self {
        Self {
            db_client: None,
            kvs_client: None,
            in_memory: None,
            env_client: None,
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

    /// InMemoryClientを設定
    pub fn with_in_memory(mut self, client: &'a dyn InMemoryClient) -> Self {
        self.in_memory = Some(client);
        self
    }

    /// ENVClientを設定
    pub fn with_env_client(mut self, client: &'a dyn ENVClient) -> Self {
        self.env_client = Some(client);
        self
    }

    /// placeholder 解決済みの config でデータをロード
    ///
    /// # Arguments
    /// * `config` - 解決済み _load メタデータ
    ///
    /// # Returns
    /// * `Ok(Value)` - ロード成功
    /// * `Err(String)` - ロード失敗
    pub fn handle(&self, config: &HashMap<String, Value>) -> Result<Value, String> {
        let client = config
            .get("client")
            .and_then(|v| v.as_str())
            .ok_or("Load::handle: 'client' not found in _load config")?;

        match client {
            "Env" | "ENV" => self.load_from_env(config),
            "InMemory" => self.load_from_in_memory(config),
            "KVS" => self.load_from_kvs(config),
            "DB" => self.load_from_db(config),
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

    /// InMemoryから読み込み
    fn load_from_in_memory(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        let in_memory = self
            .in_memory
            .ok_or("Load::load_from_in_memory: InMemoryClient not configured")?;

        let key = config
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or("Load::load_from_in_memory: 'key' not found")?;

        // placeholder はすでに resolved_config で解決済み
        in_memory
            .get(key)
            .ok_or_else(|| format!("Load::load_from_in_memory: key '{}' not found", key))
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
        let value_str = kvs_client
            .get(key)
            .ok_or_else(|| format!("Load::load_from_kvs: key '{}' not found", key))?;

        // deserialize処理
        // 全ての値はJSON形式で保存されている（型情報保持）
        serde_json::from_str(&value_str)
            .map_err(|e| format!("Load::load_from_kvs: JSON parse error: {}", e))
    }

    /// DBから読み込み
    fn load_from_db(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        let db_client = self
            .db_client
            .ok_or("Load::load_from_db: DBClient not configured")?;

        let table = config
            .get("table")
            .and_then(|v| v.as_str())
            .ok_or("Load::load_from_db: 'table' not found")?;

        let where_clause = config.get("where").and_then(|v| v.as_str());

        let map = config
            .get("map")
            .and_then(|v| v.as_object())
            .ok_or("Load::load_from_db: 'map' not found")?;

        // connection 値を取得（String でも Object でも OK）
        let connection = config
            .get("connection")
            .ok_or("Load::load_from_db: 'connection' not specified")?;

        // map から SELECT カラムを抽出
        let columns: Vec<&str> = map
            .values()
            .filter_map(|v| v.as_str())
            .collect();

        if columns.is_empty() {
            return Err("Load::load_from_db: no columns specified in map".to_string());
        }

        // DB から取得（常に Vec で返る）
        let rows = db_client
            .fetch(connection, table, &columns, where_clause)
            .ok_or_else(|| format!("Load::load_from_db: fetch failed for table '{}'", table))?;

        // 空チェック
        if rows.is_empty() {
            return Err(format!("Load::load_from_db: no data found in table '{}'", table));
        }

        // 1件目を取得（現状はレコード形式のみ対応）
        let row = &rows[0];

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

    // feature function: load with API Cleint
    // fn load_from_api(
    //     &self,
    //     config: &HashMap<String, Value>,
    // ) -> Result<Value, String> {
    //     let api_client = self
    //         .api_client
    //         .ok_or("Load::load_from_api: APIClient not configured")?;

    //     let url = config
    //         .get("url")
    //         .and_then(|v| v.as_str())
    //         .ok_or("Load::load_from_api: 'url' not found")?;

    //     // placeholder はすでに resolved_config で解決済み

    //     // headers処理（optional）
    //     let headers = config.get("headers").and_then(|v| v.as_object()).map(|h| {
    //         h.iter()
    //             .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
    //             .collect::<HashMap<String, String>>()
    //     });

    //     api_client.get(url, headers.as_ref())
    // }
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
        let load = Load::new().with_env_client(&env_client);

        let mut config = HashMap::new();
        config.insert("client".to_string(), Value::String("Env".to_string()));

        let mut map = serde_json::Map::new();
        map.insert("host".to_string(), Value::String("DB_HOST".to_string()));
        map.insert("port".to_string(), Value::String("DB_PORT".to_string()));
        config.insert("map".to_string(), Value::Object(map));

        let result = load.handle(&config).unwrap();

        assert_eq!(result.get("host"), Some(&Value::String("localhost".to_string())));
        assert_eq!(result.get("port"), Some(&Value::String("5432".to_string())));
    }

    // 再帰深度テストは State が管理するため削除
    // Load は単純なデータ取得のみを担当
}
