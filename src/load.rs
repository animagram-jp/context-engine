use crate::ports::required::{
    DbClient, EnvClient, KVSClient,
    InMemoryClient, HttpClient, FileClient,
};
use crate::ports::provided::{LoadError, Value};
use crate::core::fixed_bits;
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

    pub fn handle(&self, config: &HashMap<String, Value>) -> Result<Value, LoadError> {
        let client = match config.get("client") {
            Some(Value::Scalar(b)) => {
                u64::from_le_bytes(b.as_slice().try_into().unwrap_or([0u8; 8]))
            }
            _ => return Err(LoadError::ConfigMissing("client".into())),
        };

        match client {
            fixed_bits::CLIENT_ENV       => self.load_from_env(config),
            fixed_bits::CLIENT_IN_MEMORY => self.load_from_in_memory(config),
            fixed_bits::CLIENT_KVS       => self.load_from_kvs(config),
            fixed_bits::CLIENT_DB        => self.load_from_db(config),
            fixed_bits::CLIENT_HTTP      => self.load_from_http(config),
            fixed_bits::CLIENT_FILE      => self.load_from_file(config),
            _ => Err(LoadError::ConfigMissing(format!("unsupported client '{}'", client))),
        }
    }

    fn load_from_env(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, LoadError> {
        let env = self.env.as_deref()
            .ok_or(LoadError::ClientNotConfigured)?;

        let (yaml_keys, ext_keys) = get_map_keys(config)?;
        let values = env.get(&ext_keys)
            .ok_or(LoadError::ClientNotConfigured)?;
        Ok(zip_to_mapping(yaml_keys, values))
    }

    fn load_from_in_memory(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, LoadError> {
        let in_memory = self.in_memory.as_deref()
            .ok_or(LoadError::ClientNotConfigured)?;

        let key = scalar_str(config, "key")?;
        in_memory
            .get(key)
            .ok_or_else(|| LoadError::NotFound(key.into()))
    }

    fn load_from_kvs(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, LoadError> {
        let kvs = self.kvs.as_deref()
            .ok_or(LoadError::ClientNotConfigured)?;

        let key = scalar_str(config, "key")?;
        let bytes = kvs
            .get(key)
            .ok_or_else(|| LoadError::NotFound(key.into()))?;
        Ok(crate::codec_value::decode(&bytes).unwrap_or(Value::Scalar(bytes)))
    }

    fn load_from_db(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, LoadError> {
        let db = self.db.as_deref()
            .ok_or(LoadError::ClientNotConfigured)?;

        let connection = config
            .get("connection")
            .ok_or(LoadError::ConfigMissing("connection".into()))?;

        let table = scalar_str(config, "table")?;

        let (yaml_keys, ext_keys) = get_map_keys(config)?;

        let where_clause = config.get("where")
            .and_then(|v| if let Value::Scalar(b) = v { Some(b.as_slice()) } else { None });

        let values = db
            .get(connection, table, &ext_keys, where_clause)
            .ok_or_else(|| LoadError::NotFound(table.into()))?;

        Ok(zip_to_mapping(yaml_keys, values))
    }

    fn load_from_file(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, LoadError> {
        let file = self.file.as_deref()
            .ok_or(LoadError::ClientNotConfigured)?;

        let key = scalar_str(config, "key")?;
        let bytes = file
            .get(key)
            .ok_or_else(|| LoadError::NotFound(key.into()))?;
        Ok(crate::codec_value::decode(&bytes).unwrap_or(Value::Scalar(bytes)))
    }

    fn load_from_http(
        &self,
        config: &HashMap<String, Value>,
    ) -> Result<Value, LoadError> {
        let http = self.http.as_deref()
            .ok_or(LoadError::ClientNotConfigured)?;

        let url = scalar_str(config, "url")?;

        let (yaml_keys, ext_keys) = get_map_keys(config)?;

        let headers = match config.get("headers") {
            Some(Value::Mapping(m)) => Some(
                m.iter()
                    .filter_map(|(k, v)| {
                        if let Value::Scalar(val) = v { Some((k.clone(), val.clone())) } else { None }
                    })
                    .collect::<Vec<_>>()
            ),
            _ => None,
        };

        let values = http.get(url, &ext_keys, headers.as_deref())
            .ok_or_else(|| LoadError::NotFound(url.into()))?;

        Ok(zip_to_mapping(yaml_keys, values))
    }
}

fn get_map_keys(config: &HashMap<String, Value>) -> Result<(Vec<Vec<u8>>, Vec<Vec<u8>>), LoadError> {
    let yaml_keys = match config.get("yaml_keys") {
        Some(Value::Sequence(s)) => s.iter().filter_map(|v| if let Value::Scalar(b) = v { Some(b.clone()) } else { None }).collect(),
        _ => return Err(LoadError::ConfigMissing("yaml_keys".into())),
    };
    let ext_keys = match config.get("ext_keys") {
        Some(Value::Sequence(s)) => s.iter().filter_map(|v| if let Value::Scalar(b) = v { Some(b.clone()) } else { None }).collect(),
        _ => return Err(LoadError::ConfigMissing("ext_keys".into())),
    };
    Ok((yaml_keys, ext_keys))
}

/// Zips yaml_keys and values into a Value::Mapping.
fn zip_to_mapping(yaml_keys: Vec<Vec<u8>>, values: Vec<Value>) -> Value {
    Value::Mapping(yaml_keys.into_iter().zip(values).collect())
}

fn scalar_str<'a>(config: &'a HashMap<String, Value>, key: &str) -> Result<&'a str, LoadError> {
    match config.get(key) {
        Some(Value::Scalar(b)) => std::str::from_utf8(b)
            .map_err(|_| LoadError::ConfigMissing(key.into())),
        _ => Err(LoadError::ConfigMissing(key.into())),
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

    fn client_config(client_id: u64) -> Value {
        Value::Scalar(client_id.to_le_bytes().to_vec())
    }

    fn map_config(pairs: &[(&str, &str)]) -> (Value, Value) {
        let yaml_keys = Value::Sequence(pairs.iter().map(|(k, _)| Value::Scalar(k.as_bytes().to_vec())).collect());
        let ext_keys  = Value::Sequence(pairs.iter().map(|(_, v)| Value::Scalar(v.as_bytes().to_vec())).collect());
        (yaml_keys, ext_keys)
    }

    // --- Env ---

    struct MockEnvClient;
    impl EnvClient for MockEnvClient {
        fn get(&self, keys: &[Vec<u8>]) -> Option<Vec<Value>> {
            let vals = keys.iter().map(|k| match k.as_slice() {
                b"DB_HOST" => Value::Scalar(b"localhost".to_vec()),
                b"DB_PORT" => Value::Scalar(b"5432".to_vec()),
                _ => Value::Null,
            }).collect();
            Some(vals)
        }
        fn set(&self, _key: &str, _value: Vec<u8>) -> bool { false }
        fn delete(&self, _key: &str) -> bool { false }
    }

    #[test]
    fn test_load_from_env() {
        let load = Load::new().with_env(Arc::new(MockEnvClient));
        let mut config = HashMap::new();
        config.insert("client".to_string(), client_config(fixed_bits::CLIENT_ENV));
        let (yaml_keys, ext_keys) = map_config(&[("host", "DB_HOST"), ("port", "DB_PORT")]);
        config.insert("yaml_keys".to_string(), yaml_keys);
        config.insert("ext_keys".to_string(), ext_keys);
        let result = load.handle(&config).unwrap();
        if let Value::Mapping(m) = result {
            let host = m.iter().find(|(k, _)| k == b"host").map(|(_, v)| v.clone());
            assert_eq!(host, Some(Value::Scalar(b"localhost".to_vec())));
        } else {
            panic!("expected Mapping");
        }
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
        let data = Value::Mapping(vec![(b"host".to_vec(), Value::Scalar(b"localhost".to_vec()))]);
        let client = Arc::new(MockInMemory::new(&[("conn", data.clone())]));
        let load = Load::new().with_in_memory(client);
        let mut config = HashMap::new();
        config.insert("client".to_string(), client_config(fixed_bits::CLIENT_IN_MEMORY));
        config.insert("key".to_string(), Value::Scalar(b"conn".to_vec()));
        assert_eq!(load.handle(&config).unwrap(), data);
    }

    #[test]
    fn test_load_from_in_memory_key_not_found() {
        let client = Arc::new(MockInMemory::new(&[]));
        let load = Load::new().with_in_memory(client);
        let mut config = HashMap::new();
        config.insert("client".to_string(), client_config(fixed_bits::CLIENT_IN_MEMORY));
        config.insert("key".to_string(), Value::Scalar(b"missing".to_vec()));
        assert!(load.handle(&config).is_err());
    }

    #[test]
    fn test_load_from_in_memory_client_not_configured() {
        let load = Load::new();
        let mut config = HashMap::new();
        config.insert("client".to_string(), client_config(fixed_bits::CLIENT_IN_MEMORY));
        config.insert("key".to_string(), Value::Scalar(b"k".to_vec()));
        assert!(load.handle(&config).is_err());
    }

    // --- KVS ---

    struct MockKVS {
        store: std::sync::Mutex<HashMap<String, Vec<u8>>>,
    }
    impl MockKVS {
        fn new(entries: &[(&str, &[u8])]) -> Self {
            Self { store: std::sync::Mutex::new(entries.iter().map(|(k, v)| (k.to_string(), v.to_vec())).collect()) }
        }
    }
    impl KVSClient for MockKVS {
        fn get(&self, key: &str) -> Option<Vec<u8>> { self.store.lock().unwrap().get(key).cloned() }
        fn set(&self, key: &str, value: Vec<u8>, _: Option<u64>) -> bool { self.store.lock().unwrap().insert(key.to_string(), value); true }
        fn delete(&self, key: &str) -> bool { self.store.lock().unwrap().remove(key).is_some() }
    }

    #[test]
    fn test_load_from_kvs() {
        let client = Arc::new(MockKVS::new(&[("sess", b"{\"user_id\":1}")]));
        let load = Load::new().with_kvs(client);
        let mut config = HashMap::new();
        config.insert("client".to_string(), client_config(fixed_bits::CLIENT_KVS));
        config.insert("key".to_string(), Value::Scalar(b"sess".to_vec()));
        assert!(matches!(load.handle(&config).unwrap(), Value::Scalar(_)));
    }

    #[test]
    fn test_load_from_kvs_key_not_found() {
        let client = Arc::new(MockKVS::new(&[]));
        let load = Load::new().with_kvs(client);
        let mut config = HashMap::new();
        config.insert("client".to_string(), client_config(fixed_bits::CLIENT_KVS));
        config.insert("key".to_string(), Value::Scalar(b"missing".to_vec()));
        assert!(load.handle(&config).is_err());
    }

    #[test]
    fn test_load_from_kvs_client_not_configured() {
        let load = Load::new();
        let mut config = HashMap::new();
        config.insert("client".to_string(), client_config(fixed_bits::CLIENT_KVS));
        config.insert("key".to_string(), Value::Scalar(b"k".to_vec()));
        assert!(load.handle(&config).is_err());
    }

    // --- DB ---

    struct MockDb {
        rows: Vec<Value>,
    }
    impl MockDb {
        fn new(rows: Vec<Value>) -> Self { Self { rows } }
    }
    impl DbClient for MockDb {
        fn get(&self, _conn: &Value, _table: &str, _keys: &[Vec<u8>], _where: Option<&[u8]>) -> Option<Vec<Value>> {
            if self.rows.is_empty() { None } else { Some(self.rows.clone()) }
        }
        fn set(&self, _: &Value, _: &str, _: &[Vec<u8>], _: Option<&[u8]>) -> bool { false }
        fn delete(&self, _: &Value, _: &str, _: Option<&[u8]>) -> bool { false }
    }

    fn db_config(table: &str, map: &[(&str, &str)]) -> HashMap<String, Value> {
        let mut config = HashMap::new();
        config.insert("client".to_string(), client_config(fixed_bits::CLIENT_DB));
        config.insert("table".to_string(), Value::Scalar(table.as_bytes().to_vec()));
        config.insert("connection".to_string(), Value::Mapping(vec![]));
        let (yaml_keys, ext_keys) = map_config(map);
        config.insert("yaml_keys".to_string(), yaml_keys);
        config.insert("ext_keys".to_string(), ext_keys);
        config
    }

    #[test]
    fn test_load_from_db() {
        // adapter returns field values in ext_keys order
        let client = Arc::new(MockDb::new(vec![Value::Scalar(b"42".to_vec())]));
        let load = Load::new().with_db(client);
        let config = db_config("users", &[("id", "id")]);
        let result = load.handle(&config).unwrap();
        // zip_to_mapping: yaml_key "id" → Value::Scalar("42")
        let expected = Value::Mapping(vec![(b"id".to_vec(), Value::Scalar(b"42".to_vec()))]);
        assert_eq!(result, expected);
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
    impl HttpClient for MockHttp {
        fn get(&self, _: &str, keys: &[Vec<u8>], _: Option<&[(Vec<u8>, Vec<u8>)]>) -> Option<Vec<Value>> {
            self.response.as_ref().map(|v| {
                keys.iter().map(|k| match v {
                    Value::Mapping(m) => m.iter()
                        .find(|(mk, _)| mk == k)
                        .map(|(_, mv)| mv.clone())
                        .unwrap_or(Value::Null),
                    _ => v.clone(),
                }).collect()
            })
        }
        fn set(&self, _: &str, _: Value, _: Option<&[(Vec<u8>, Vec<u8>)]>) -> bool { false }
        fn delete(&self, _: &str, _: Option<&[(Vec<u8>, Vec<u8>)]>) -> bool { false }
    }

    fn http_config(url: &str) -> HashMap<String, Value> {
        let mut c = HashMap::new();
        c.insert("client".to_string(), client_config(fixed_bits::CLIENT_HTTP));
        c.insert("url".to_string(), Value::Scalar(url.as_bytes().to_vec()));
        let (yaml_keys, ext_keys) = map_config(&[("status", "status")]);
        c.insert("yaml_keys".to_string(), yaml_keys);
        c.insert("ext_keys".to_string(), ext_keys);
        c
    }

    #[test]
    fn test_load_from_http() {
        let response = Value::Mapping(vec![(b"status".to_vec(), Value::Scalar(b"ok".to_vec()))]);
        let client = Arc::new(MockHttp::new(Some(response.clone())));
        let load = Load::new().with_http(client);
        let config = http_config("http://example.com/health");
        assert_eq!(load.handle(&config).unwrap(), response);
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

    // --- File ---

    struct MockFileClient {
        store: std::sync::Mutex<HashMap<String, Vec<u8>>>,
    }
    impl MockFileClient {
        fn new(entries: &[(&str, &[u8])]) -> Self {
            Self {
                store: std::sync::Mutex::new(
                    entries.iter().map(|(k, v)| (k.to_string(), v.to_vec())).collect()
                ),
            }
        }
    }
    impl FileClient for MockFileClient {
        fn get(&self, key: &str) -> Option<Vec<u8>> {
            self.store.lock().unwrap().get(key).cloned()
        }
        fn set(&self, key: &str, value: Vec<u8>) -> bool {
            self.store.lock().unwrap().insert(key.to_string(), value);
            true
        }
        fn delete(&self, key: &str) -> bool {
            self.store.lock().unwrap().remove(key).is_some()
        }
    }

    #[test]
    fn test_load_from_file() {
        let file = MockFileClient::new(&[("session_data", b"{\"user_id\":42}")]);
        let load = Load::new().with_file(Arc::new(file));
        let mut config = HashMap::new();
        config.insert("client".to_string(), client_config(fixed_bits::CLIENT_FILE));
        config.insert("key".to_string(), Value::Scalar(b"session_data".to_vec()));
        assert!(matches!(load.handle(&config).unwrap(), Value::Scalar(_)));
    }

    #[test]
    fn test_load_from_file_key_not_found() {
        let file = MockFileClient::new(&[]);
        let load = Load::new().with_file(Arc::new(file));
        let mut config = HashMap::new();
        config.insert("client".to_string(), client_config(fixed_bits::CLIENT_FILE));
        config.insert("key".to_string(), Value::Scalar(b"missing".to_vec()));
        assert!(load.handle(&config).is_err());
    }

    #[test]
    fn test_load_from_file_client_not_configured() {
        let load = Load::new();
        let mut config = HashMap::new();
        config.insert("client".to_string(), client_config(fixed_bits::CLIENT_FILE));
        config.insert("key".to_string(), Value::Scalar(b"any".to_vec()));
        assert!(load.handle(&config).is_err());
    }
}
