use crate::ports::required::{
    DbClient, EnvClient, KVSClient,
    InMemoryClient, HttpClient, FileClient,
};
use crate::core::fixed_bits;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Load {
    db: Option<Arc<dyn DbClient>>,
    kvs: Option<Arc<dyn KVSClient>>,
    in_memory: Option<Arc<dyn InMemoryClient>>,
    env: Option<Arc<dyn EnvClient>>,
    http: Option<Arc<dyn HttpClient>>,
    file: Option<Arc<dyn FileClient>>,
}

impl Load {
    pub fn new() -> Self {
        Self {
            db: None,
            kvs: None,
            in_memory: None,
            env: None,
            http: None,
            file: None,
        }
    }

    pub fn with_db(mut self, client: Arc<dyn DbClient>) -> Self {
        self.db = Some(client);
        self
    }

    pub fn with_kvs(mut self, client: Arc<dyn KVSClient>) -> Self {
        self.kvs = Some(client);
        self
    }

    pub fn with_in_memory(mut self, client: Arc<dyn InMemoryClient>) -> Self {
        self.in_memory = Some(client);
        self
    }

    pub fn with_env(mut self, client: Arc<dyn EnvClient>) -> Self {
        self.env = Some(client);
        self
    }

    pub fn with_http(mut self, client: Arc<dyn HttpClient>) -> Self {
        self.http = Some(client);
        self
    }

    pub fn with_file(mut self, client: Arc<dyn FileClient>) -> Self {
        self.file = Some(client);
        self
    }

    pub fn handle(&self, config: &HashMap<String, Value>) -> Result<Value, String> {
        let client = config
            .get("client")
            .and_then(|v| v.as_u64())
            .ok_or("Load::handle: 'client' not found in _load config")?;

        match client {
            fixed_bits::CLIENT_ENV       => self.load_from_env(config),
            fixed_bits::CLIENT_IN_MEMORY => self.load_from_in_memory(config),
            fixed_bits::CLIENT_KVS       => self.load_from_kvs(config),
            fixed_bits::CLIENT_DB        => self.load_from_db(config),
            fixed_bits::CLIENT_HTTP      => self.load_from_http(config),
            fixed_bits::CLIENT_FILE      => self.load_from_file(config),
            _ => Err(format!("Load::handle: unsupported client '{}'", client)),
        }
    }

    fn load_from_env(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        let env = self.env.as_deref()
            .ok_or("Load::load_from_env: EnvClient not configured")?;

        let map = config
            .get("map")
            .and_then(|v| v.as_object())
            .ok_or("Load::load_from_env: 'map' not found")?;

        let mut result = serde_json::Map::new();
        for (config_key, env_key_value) in map {
            if let Some(env_key) = env_key_value.as_str() {
                if let Some(value) = env.get(env_key) {
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
        let in_memory = self.in_memory.as_deref()
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
        let kvs = self.kvs.as_deref()
            .ok_or("Load::load_from_kvs: KVSClient not configured")?;

        let key = config
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or("Load::load_from_kvs: 'key' not found")?;

        let value_str = kvs
            .get(key)
            .ok_or_else(|| format!("Load::load_from_kvs: key '{}' not found", key))?;

        serde_json::from_str(&value_str)
            .map_err(|e| format!("Load::load_from_kvs: JSON parse error: {}", e))
    }

    fn load_from_db(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        let db = self.db.as_deref()
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

        let columns: Vec<&str> = map.values().filter_map(|v| v.as_str()).collect();

        if columns.is_empty() {
            return Err("Load::load_from_db: no columns specified in map".to_string());
        }

        let rows = db
            .get(connection, table, &columns, where_clause)
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

    fn load_from_file(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        let file = self.file.as_deref()
            .ok_or("Load::load_from_file: FileClient not configured")?;

        let key = config
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or("Load::load_from_file: 'key' not found")?;

        let content = file
            .get(key)
            .ok_or_else(|| format!("Load::load_from_file: key '{}' not found", key))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Load::load_from_file: JSON parse error: {}", e))
    }

    fn load_from_http(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        let http = self.http.as_deref()
            .ok_or("Load::load_from_http: HttpClient not configured")?;

        let url = config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or("Load::load_from_http: 'url' not found")?;

        let headers = config
            .get("headers")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect::<HashMap<String, String>>());

        let response = http.get(url, headers.as_ref())
            .ok_or_else(|| format!("Load::load_from_http: GET '{}' failed", url))?;

        let map = config.get("map").and_then(|v| v.as_object());
        match map {
            None => Ok(response),
            Some(map) => {
                let row = match &response {
                    Value::Array(arr) => arr.first()
                        .ok_or_else(|| "Load::load_from_http: empty array response".to_string())?,
                    other => other,
                };
                let mut result = serde_json::Map::new();
                for (config_key, src_key_value) in map {
                    if let Some(src_key) = src_key_value.as_str() {
                        if let Some(value) = row.get(src_key) {
                            result.insert(config_key.clone(), value.clone());
                        }
                    }
                }
                Ok(Value::Object(result))
            }
        }
    }
}

impl Default for Load {
    fn default() -> Self {
        Self::new()
    }
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
        fn set(&self, _key: &str, _value: String) -> bool { false }
        fn delete(&self, _key: &str) -> bool { false }
    }

    struct MockFileClient {
        store: std::sync::Mutex<HashMap<String, String>>,
    }
    impl MockFileClient {
        fn new(entries: &[(&str, &str)]) -> Self {
            Self {
                store: std::sync::Mutex::new(
                    entries.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
                ),
            }
        }
    }
    impl FileClient for MockFileClient {
        fn get(&self, key: &str) -> Option<String> {
            self.store.lock().unwrap().get(key).cloned()
        }
        fn set(&self, key: &str, value: String) -> bool {
            self.store.lock().unwrap().insert(key.to_string(), value);
            true
        }
        fn delete(&self, key: &str) -> bool {
            self.store.lock().unwrap().remove(key).is_some()
        }
    }

    #[test]
    fn test_load_from_file() {
        let file = MockFileClient::new(&[("session_data", r#"{"user_id":42}"#)]);
        let load = Load::new().with_file(Arc::new(file));

        let mut config = HashMap::new();
        config.insert("client".to_string(), Value::Number(fixed_bits::CLIENT_FILE.into()));
        config.insert("key".to_string(), Value::String("session_data".to_string()));

        let result = load.handle(&config).unwrap();
        assert_eq!(result.get("user_id"), Some(&Value::Number(42.into())));
    }

    #[test]
    fn test_load_from_file_key_not_found() {
        let file = MockFileClient::new(&[]);
        let load = Load::new().with_file(Arc::new(file));

        let mut config = HashMap::new();
        config.insert("client".to_string(), Value::Number(fixed_bits::CLIENT_FILE.into()));
        config.insert("key".to_string(), Value::String("missing".to_string()));

        assert!(load.handle(&config).is_err());
    }

    #[test]
    fn test_load_from_file_client_not_configured() {
        let load = Load::new();

        let mut config = HashMap::new();
        config.insert("client".to_string(), Value::Number(fixed_bits::CLIENT_FILE.into()));
        config.insert("key".to_string(), Value::String("any".to_string()));

        assert!(load.handle(&config).is_err());
    }

    // --- InMemory ---

    struct MockInMemory {
        store: std::sync::Mutex<HashMap<String, Value>>,
    }
    impl MockInMemory {
        fn new(entries: &[(&str, Value)]) -> Self {
            Self { store: std::sync::Mutex::new(entries.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()) }
        }
    }
    impl InMemoryClient for MockInMemory {
        fn get(&self, key: &str) -> Option<Value> { self.store.lock().unwrap().get(key).cloned() }
        fn set(&self, key: &str, value: Value) -> bool { self.store.lock().unwrap().insert(key.to_string(), value); true }
        fn delete(&self, key: &str) -> bool { self.store.lock().unwrap().remove(key).is_some() }
    }

    #[test]
    fn test_load_from_in_memory() {
        let data = serde_json::json!({"host": "localhost"});
        let client = Arc::new(MockInMemory::new(&[("conn", data.clone())]));
        let load = Load::new().with_in_memory(client);
        let mut config = HashMap::new();
        config.insert("client".to_string(), Value::Number(fixed_bits::CLIENT_IN_MEMORY.into()));
        config.insert("key".to_string(), Value::String("conn".to_string()));
        assert_eq!(load.handle(&config).unwrap(), data);
    }

    #[test]
    fn test_load_from_in_memory_key_not_found() {
        let client = Arc::new(MockInMemory::new(&[]));
        let load = Load::new().with_in_memory(client);
        let mut config = HashMap::new();
        config.insert("client".to_string(), Value::Number(fixed_bits::CLIENT_IN_MEMORY.into()));
        config.insert("key".to_string(), Value::String("missing".to_string()));
        assert!(load.handle(&config).is_err());
    }

    #[test]
    fn test_load_from_in_memory_client_not_configured() {
        let load = Load::new();
        let mut config = HashMap::new();
        config.insert("client".to_string(), Value::Number(fixed_bits::CLIENT_IN_MEMORY.into()));
        config.insert("key".to_string(), Value::String("k".to_string()));
        assert!(load.handle(&config).is_err());
    }

    // --- KVS ---

    struct MockKVS {
        store: std::sync::Mutex<HashMap<String, String>>,
    }
    impl MockKVS {
        fn new(entries: &[(&str, &str)]) -> Self {
            Self { store: std::sync::Mutex::new(entries.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()) }
        }
    }
    impl KVSClient for MockKVS {
        fn get(&self, key: &str) -> Option<String> { self.store.lock().unwrap().get(key).cloned() }
        fn set(&self, key: &str, value: String, _: Option<u64>) -> bool { self.store.lock().unwrap().insert(key.to_string(), value); true }
        fn delete(&self, key: &str) -> bool { self.store.lock().unwrap().remove(key).is_some() }
    }

    #[test]
    fn test_load_from_kvs() {
        let client = Arc::new(MockKVS::new(&[("sess", r#"{"user_id":1}"#)]));
        let load = Load::new().with_kvs(client);
        let mut config = HashMap::new();
        config.insert("client".to_string(), Value::Number(fixed_bits::CLIENT_KVS.into()));
        config.insert("key".to_string(), Value::String("sess".to_string()));
        assert_eq!(load.handle(&config).unwrap().get("user_id"), Some(&Value::Number(1.into())));
    }

    #[test]
    fn test_load_from_kvs_key_not_found() {
        let client = Arc::new(MockKVS::new(&[]));
        let load = Load::new().with_kvs(client);
        let mut config = HashMap::new();
        config.insert("client".to_string(), Value::Number(fixed_bits::CLIENT_KVS.into()));
        config.insert("key".to_string(), Value::String("missing".to_string()));
        assert!(load.handle(&config).is_err());
    }

    #[test]
    fn test_load_from_kvs_client_not_configured() {
        let load = Load::new();
        let mut config = HashMap::new();
        config.insert("client".to_string(), Value::Number(fixed_bits::CLIENT_KVS.into()));
        config.insert("key".to_string(), Value::String("k".to_string()));
        assert!(load.handle(&config).is_err());
    }

    // --- DB ---

    struct MockDb {
        rows: Vec<HashMap<String, Value>>,
    }
    impl MockDb {
        fn new(rows: Vec<HashMap<String, Value>>) -> Self { Self { rows } }
    }
    impl DbClient for MockDb {
        fn get(&self, _conn: &Value, _table: &str, _cols: &[&str], _where: Option<&str>) -> Option<Vec<HashMap<String, Value>>> {
            if self.rows.is_empty() { None } else { Some(self.rows.clone()) }
        }
        fn set(&self, _: &Value, _: &str, _: &HashMap<String, Value>, _: Option<&str>) -> bool { false }
        fn delete(&self, _: &Value, _: &str, _: Option<&str>) -> bool { false }
    }

    fn db_config(table: &str, map: &[(&str, &str)]) -> HashMap<String, Value> {
        let mut config = HashMap::new();
        config.insert("client".to_string(), Value::Number(fixed_bits::CLIENT_DB.into()));
        config.insert("table".to_string(), Value::String(table.to_string()));
        config.insert("connection".to_string(), Value::Object(serde_json::Map::new()));
        let mut map_obj = serde_json::Map::new();
        for (k, v) in map { map_obj.insert(k.to_string(), Value::String(v.to_string())); }
        config.insert("map".to_string(), Value::Object(map_obj));
        config
    }

    #[test]
    fn test_load_from_db() {
        let mut row = HashMap::new();
        row.insert("id".to_string(), Value::Number(42.into()));
        let client = Arc::new(MockDb::new(vec![row]));
        let load = Load::new().with_db(client);
        let config = db_config("users", &[("id", "id")]);
        assert_eq!(load.handle(&config).unwrap().get("id"), Some(&Value::Number(42.into())));
    }

    #[test]
    fn test_load_from_db_no_rows() {
        let client = Arc::new(MockDb::new(vec![]));
        let load = Load::new().with_db(client);
        let config = db_config("users", &[("id", "id")]);
        assert!(load.handle(&config).is_err());
    }

    #[test]
    fn test_load_from_db_client_not_configured() {
        let load = Load::new();
        let config = db_config("users", &[("id", "id")]);
        assert!(load.handle(&config).is_err());
    }

    // --- HTTP ---

    struct MockHttp {
        response: Option<Value>,
    }
    impl MockHttp {
        fn new(response: Option<Value>) -> Self { Self { response } }
    }
    impl crate::ports::required::HttpClient for MockHttp {
        fn get(&self, _: &str, _: Option<&HashMap<String, String>>) -> Option<Value> { self.response.clone() }
        fn set(&self, _: &str, _: Value, _: Option<&HashMap<String, String>>) -> bool { false }
        fn delete(&self, _: &str, _: Option<&HashMap<String, String>>) -> bool { false }
    }

    fn http_config(url: &str) -> HashMap<String, Value> {
        let mut c = HashMap::new();
        c.insert("client".to_string(), Value::Number(fixed_bits::CLIENT_HTTP.into()));
        c.insert("url".to_string(), Value::String(url.to_string()));
        c
    }

    #[test]
    fn test_load_from_http_no_map() {
        let client = Arc::new(MockHttp::new(Some(serde_json::json!({"status": "ok"}))));
        let load = Load::new().with_http(client);
        let config = http_config("http://example.com/health");
        assert_eq!(load.handle(&config).unwrap(), serde_json::json!({"status": "ok"}));
    }

    #[test]
    fn test_load_from_http_with_map() {
        let client = Arc::new(MockHttp::new(Some(serde_json::json!({"status": "ok"}))));
        let load = Load::new().with_http(client);
        let mut config = http_config("http://example.com/health");
        let mut map = serde_json::Map::new();
        map.insert("health".to_string(), Value::String("status".to_string()));
        config.insert("map".to_string(), Value::Object(map));
        let result = load.handle(&config).unwrap();
        assert_eq!(result.get("health"), Some(&Value::String("ok".to_string())));
    }

    #[test]
    fn test_load_from_http_not_found() {
        let client = Arc::new(MockHttp::new(None));
        let load = Load::new().with_http(client);
        let config = http_config("http://example.com/health");
        assert!(load.handle(&config).is_err());
    }

    #[test]
    fn test_load_from_http_client_not_configured() {
        let load = Load::new();
        let config = http_config("http://example.com/health");
        assert!(load.handle(&config).is_err());
    }

    #[test]
    fn test_load_from_env() {
        let env = MockEnvClient;
        let load = Load::new().with_env(Arc::new(env));

        let mut config = HashMap::new();
        config.insert("client".to_string(), Value::Number(fixed_bits::CLIENT_ENV.into()));

        let mut map = serde_json::Map::new();
        map.insert("host".to_string(), Value::String("Db_HOST".to_string()));
        map.insert("port".to_string(), Value::String("Db_PORT".to_string()));
        config.insert("map".to_string(), Value::Object(map));

        let result = load.handle(&config).unwrap();

        assert_eq!(result.get("host"), Some(&Value::String("localhost".to_string())));
        assert_eq!(result.get("port"), Some(&Value::String("5432".to_string())));
    }
}
