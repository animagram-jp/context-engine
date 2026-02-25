use crate::ports::required::{
    DbClient, EnvClient, KVSClient,
    InMemoryClient,
};
use crate::common::bit;
use serde_json::Value;
use std::collections::HashMap;

pub struct Load<'a> {
    db_client: Option<&'a dyn DbClient>,
    kvs_client: Option<&'a dyn KVSClient>,
    in_memory: Option<&'a dyn InMemoryClient>,
    env_client: Option<&'a dyn EnvClient>,
}

impl<'a> Load<'a> {
    pub fn new() -> Self {
        Self {
            db_client: None,
            kvs_client: None,
            in_memory: None,
            env_client: None,
        }
    }

    pub fn with_db_client(mut self, client: &'a dyn DbClient) -> Self {
        self.db_client = Some(client);
        self
    }

    pub fn with_kvs_client(mut self, client: &'a dyn KVSClient) -> Self {
        self.kvs_client = Some(client);
        self
    }

    pub fn with_in_memory(mut self, client: &'a dyn InMemoryClient) -> Self {
        self.in_memory = Some(client);
        self
    }

    pub fn with_env_client(mut self, client: &'a dyn EnvClient) -> Self {
        self.env_client = Some(client);
        self
    }

    pub fn handle(&self, config: &HashMap<String, Value>) -> Result<Value, String> {
        let client = config
            .get("client")
            .and_then(|v| v.as_u64())
            .ok_or("Load::handle: 'client' not found in _load config")?;

        match client {
            bit::CLIENT_ENV       => self.load_from_env(config),
            bit::CLIENT_IN_MEMORY => self.load_from_in_memory(config),
            bit::CLIENT_KVS       => self.load_from_kvs(config),
            bit::CLIENT_DB        => self.load_from_db(config),
            _ => Err(format!("Load::handle: unsupported client '{}'", client)),
        }
    }

    fn load_from_env(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        let env_client = self
            .env_client
            .ok_or("Load::load_from_env: EnvClient not configured")?;

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

        in_memory
            .get(key)
            .ok_or_else(|| format!("Load::load_from_in_memory: key '{}' not found", key))
    }

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

        let value_str = kvs_client
            .get(key)
            .ok_or_else(|| format!("Load::load_from_kvs: key '{}' not found", key))?;

        serde_json::from_str(&value_str)
            .map_err(|e| format!("Load::load_from_kvs: JSON parse error: {}", e))
    }

    fn load_from_db(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        let db_client = self
            .db_client
            .ok_or("Load::load_from_db: DbClient not configured")?;

        let table = config
            .get("table")
            .and_then(|v| v.as_str())
            .ok_or("Load::load_from_db: 'table' not found")?;

        let where_clause = config.get("where").and_then(|v| v.as_str());

        let map = config
            .get("map")
            .and_then(|v| v.as_object())
            .ok_or("Load::load_from_db: 'map' not found")?;

        let connection = config
            .get("connection")
            .ok_or("Load::load_from_db: 'connection' not specified")?;

        let columns: Vec<&str> = map
            .values()
            .filter_map(|v| v.as_str())
            .collect();

        if columns.is_empty() {
            return Err("Load::load_from_db: no columns specified in map".to_string());
        }

        let rows = db_client
            .fetch(connection, table, &columns, where_clause)
            .ok_or_else(|| format!("Load::load_from_db: fetch failed for table '{}'", table))?;

        if rows.is_empty() {
            return Err(format!("Load::load_from_db: no data found in table '{}'", table));
        }

        let row = &rows[0];

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

    struct MockEnvClient;
    impl EnvClient for MockEnvClient {
        fn get(&self, key: &str) -> Option<String> {
            match key {
                "Db_HOST" => Some("localhost".to_string()),
                "Db_PORT" => Some("5432".to_string()),
                _ => None,
            }
        }
    }

    #[test]
    fn test_load_from_env() {
        let env_client = MockEnvClient;
        let load = Load::new().with_env_client(&env_client);

        let mut config = HashMap::new();
        config.insert("client".to_string(), Value::Number(bit::CLIENT_ENV.into()));

        let mut map = serde_json::Map::new();
        map.insert("host".to_string(), Value::String("Db_HOST".to_string()));
        map.insert("port".to_string(), Value::String("Db_PORT".to_string()));
        config.insert("map".to_string(), Value::Object(map));

        let result = load.handle(&config).unwrap();

        assert_eq!(result.get("host"), Some(&Value::String("localhost".to_string())));
        assert_eq!(result.get("port"), Some(&Value::String("5432".to_string())));
    }

}
