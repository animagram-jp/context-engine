use crate::ports::required::{InMemoryClient, KVSClient, HttpClient, FileClient};
use crate::ports::provided::{StoreError, Value};
use crate::core::fixed_bits;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Store {
    in_memory: Option<Arc<dyn InMemoryClient>>,
    kvs: Option<Arc<dyn KVSClient>>,
    http: Option<Arc<dyn HttpClient>>,
    file: Option<Arc<dyn FileClient>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            in_memory: None,
            kvs: None,
            http: None,
            file: None,
        }
    }

    pub fn with_in_memory(mut self, client: Arc<dyn InMemoryClient>) -> Self {
        self.in_memory = Some(client);
        self
    }

    pub fn with_kvs(mut self, client: Arc<dyn KVSClient>) -> Self {
        self.kvs = Some(client);
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

    pub fn get(&self, store_config: &HashMap<String, Value>) -> Option<Value> {
        let client = client_id(store_config)?;

        match client {
            fixed_bits::CLIENT_IN_MEMORY => {
                let in_memory = self.in_memory.as_deref()?;
                let key = scalar_str(store_config, "key")?;
                in_memory.get(key)
            }
            fixed_bits::CLIENT_KVS => {
                let kvs = self.kvs.as_deref()?;
                let key = scalar_str(store_config, "key")?;
                kvs.get(key).map(|b| crate::codec_value::decode(&b).unwrap_or(Value::Scalar(b)))
            }
            fixed_bits::CLIENT_HTTP => {
                let http = self.http.as_deref()?;
                let url = scalar_str(store_config, "url")?;
                let (yaml_keys, ext_keys) = get_map_keys(store_config)?;
                let headers = headers_list(store_config);
                let values = http.get(url, &ext_keys, headers.as_deref())?;
                Some(zip_to_mapping(yaml_keys, values))
            }
            fixed_bits::CLIENT_FILE => {
                let file = self.file.as_deref()?;
                let key = scalar_str(store_config, "key")?;
                file.get(key).map(|b| crate::codec_value::decode(&b).unwrap_or(Value::Scalar(b)))
            }
            _ => None,
        }
    }

    pub fn set(
        &self,
        store_config: &HashMap<String, Value>,
        value: Value,
        ttl: Option<u64>,
    ) -> Result<bool, StoreError> {
        let client = client_id(store_config)
            .ok_or(StoreError::ConfigMissing("client".into()))?;

        match client {
            fixed_bits::CLIENT_IN_MEMORY => {
                let in_memory = self.in_memory.as_deref()
                    .ok_or(StoreError::ClientNotConfigured)?;
                let key = scalar_str(store_config, "key")
                    .ok_or(StoreError::ConfigMissing("key".into()))?;
                Ok(in_memory.set(key, value))
            }
            fixed_bits::CLIENT_KVS => {
                let kvs = self.kvs.as_deref()
                    .ok_or(StoreError::ClientNotConfigured)?;
                let key = scalar_str(store_config, "key")
                    .ok_or(StoreError::ConfigMissing("key".into()))?;
                let bytes = value_to_bytes(value);
                let final_ttl = ttl.or_else(|| scalar_u64(store_config, "ttl"));
                Ok(kvs.set(key, bytes, final_ttl))
            }
            fixed_bits::CLIENT_HTTP => {
                let http = self.http.as_deref()
                    .ok_or(StoreError::ClientNotConfigured)?;
                let url = scalar_str(store_config, "url")
                    .ok_or(StoreError::ConfigMissing("url".into()))?;
                let headers = headers_list(store_config);
                Ok(http.set(url, value, headers.as_deref()))
            }
            fixed_bits::CLIENT_FILE => {
                let file = self.file.as_deref()
                    .ok_or(StoreError::ClientNotConfigured)?;
                let key = scalar_str(store_config, "key")
                    .ok_or(StoreError::ConfigMissing("key".into()))?;
                let bytes = value_to_bytes(value);
                Ok(file.set(key, bytes))
            }
            _ => Err(StoreError::UnsupportedClient(client)),
        }
    }

    pub fn delete(&self, store_config: &HashMap<String, Value>) -> Result<bool, StoreError> {
        let client = client_id(store_config)
            .ok_or(StoreError::ConfigMissing("client".into()))?;

        match client {
            fixed_bits::CLIENT_IN_MEMORY => {
                let in_memory = self.in_memory.as_deref()
                    .ok_or(StoreError::ClientNotConfigured)?;
                let key = scalar_str(store_config, "key")
                    .ok_or(StoreError::ConfigMissing("key".into()))?;
                Ok(in_memory.delete(key))
            }
            fixed_bits::CLIENT_KVS => {
                let kvs = self.kvs.as_deref()
                    .ok_or(StoreError::ClientNotConfigured)?;
                let key = scalar_str(store_config, "key")
                    .ok_or(StoreError::ConfigMissing("key".into()))?;
                Ok(kvs.delete(key))
            }
            fixed_bits::CLIENT_HTTP => {
                let http = self.http.as_deref()
                    .ok_or(StoreError::ClientNotConfigured)?;
                let url = scalar_str(store_config, "url")
                    .ok_or(StoreError::ConfigMissing("url".into()))?;
                let headers = headers_list(store_config);
                Ok(http.delete(url, headers.as_deref()))
            }
            fixed_bits::CLIENT_FILE => {
                let file = self.file.as_deref()
                    .ok_or(StoreError::ClientNotConfigured)?;
                let key = scalar_str(store_config, "key")
                    .ok_or(StoreError::ConfigMissing("key".into()))?;
                Ok(file.delete(key))
            }
            _ => Err(StoreError::UnsupportedClient(client)),
        }
    }
}

fn client_id(config: &HashMap<String, Value>) -> Option<u64> {
    match config.get("client") {
        Some(Value::Scalar(b)) => b.as_slice().try_into().ok().map(u64::from_le_bytes),
        _ => None,
    }
}

fn scalar_str<'a>(config: &'a HashMap<String, Value>, key: &str) -> Option<&'a str> {
    match config.get(key) {
        Some(Value::Scalar(b)) => std::str::from_utf8(b).ok(),
        _ => None,
    }
}

fn scalar_u64(config: &HashMap<String, Value>, key: &str) -> Option<u64> {
    match config.get(key) {
        Some(Value::Scalar(b)) => b.as_slice().try_into().ok().map(u64::from_le_bytes),
        _ => None,
    }
}

fn headers_list(config: &HashMap<String, Value>) -> Option<Vec<(Vec<u8>, Vec<u8>)>> {
    match config.get("headers") {
        Some(Value::Mapping(m)) => Some(
            m.iter()
                .filter_map(|(k, v)| {
                    if let Value::Scalar(val) = v { Some((k.clone(), val.clone())) } else { None }
                })
                .collect()
        ),
        _ => None,
    }
}

fn get_map_keys(config: &HashMap<String, Value>) -> Option<(Vec<Vec<u8>>, Vec<Vec<u8>>)> {
    let yaml_keys = match config.get("yaml_keys") {
        Some(Value::Sequence(s)) => s.iter().filter_map(|v| if let Value::Scalar(b) = v { Some(b.clone()) } else { None }).collect(),
        _ => return None,
    };
    let ext_keys = match config.get("ext_keys") {
        Some(Value::Sequence(s)) => s.iter().filter_map(|v| if let Value::Scalar(b) = v { Some(b.clone()) } else { None }).collect(),
        _ => return None,
    };
    Some((yaml_keys, ext_keys))
}

fn zip_to_mapping(yaml_keys: Vec<Vec<u8>>, values: Vec<Value>) -> Value {
    Value::Mapping(yaml_keys.into_iter().zip(values).collect())
}

fn value_to_bytes(value: Value) -> Vec<u8> {
    match value {
        Value::Scalar(b) => b,
        other => crate::codec_value::encode(&other),
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fixed_bits;

    fn client_config(client_id: u64) -> Value {
        Value::Scalar(client_id.to_le_bytes().to_vec())
    }

    struct MockFileClient {
        store: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
    }
    impl MockFileClient {
        fn new() -> Self {
            Self { store: std::sync::Mutex::new(std::collections::HashMap::new()) }
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

    fn file_config(key: &str) -> HashMap<String, Value> {
        let mut config = HashMap::new();
        config.insert("client".to_string(), client_config(fixed_bits::CLIENT_FILE));
        config.insert("key".to_string(), Value::Scalar(key.as_bytes().to_vec()));
        config
    }

    #[test]
    fn test_store_file_set_and_get() {
        let file = Arc::new(MockFileClient::new());
        let store = Store::new().with_file(file);
        let config = file_config("my_key");
        let data = Value::Scalar(b"hello".to_vec());
        assert_eq!(store.set(&config, data.clone(), None).unwrap(), true);
        assert!(store.get(&config).is_some());
    }

    #[test]
    fn test_store_file_delete() {
        let file = Arc::new(MockFileClient::new());
        let store = Store::new().with_file(file);
        let config = file_config("my_key");
        store.set(&config, Value::Scalar(b"x".to_vec()), None).unwrap();
        assert_eq!(store.delete(&config).unwrap(), true);
        assert!(store.get(&config).is_none());
    }

    #[test]
    fn test_store_file_client_not_configured() {
        let store = Store::new();
        let config = file_config("my_key");
        assert!(store.set(&config, Value::Scalar(b"x".to_vec()), None).is_err());
        assert!(store.delete(&config).is_err());
    }

    // --- InMemory ---

    struct MockInMemory {
        store: std::sync::Mutex<std::collections::HashMap<String, Value>>,
    }
    impl MockInMemory {
        fn new() -> Self { Self { store: std::sync::Mutex::new(std::collections::HashMap::new()) } }
    }
    impl InMemoryClient for MockInMemory {
        fn get(&self, key: &str) -> Option<Value> { self.store.lock().unwrap().get(key).cloned() }
        fn set(&self, key: &str, value: Value) -> bool { self.store.lock().unwrap().insert(key.to_string(), value); true }
        fn delete(&self, key: &str) -> bool { self.store.lock().unwrap().remove(key).is_some() }
    }

    fn in_memory_config(key: &str) -> HashMap<String, Value> {
        let mut c = HashMap::new();
        c.insert("client".to_string(), client_config(fixed_bits::CLIENT_IN_MEMORY));
        c.insert("key".to_string(), Value::Scalar(key.as_bytes().to_vec()));
        c
    }

    #[test]
    fn test_store_in_memory_set_and_get() {
        let client = Arc::new(MockInMemory::new());
        let store = Store::new().with_in_memory(client);
        let config = in_memory_config("k");
        let data = Value::Scalar(b"42".to_vec());
        assert!(store.set(&config, data.clone(), None).unwrap());
        assert_eq!(store.get(&config).unwrap(), data);
    }

    #[test]
    fn test_store_in_memory_delete() {
        let client = Arc::new(MockInMemory::new());
        let store = Store::new().with_in_memory(client);
        let config = in_memory_config("k");
        store.set(&config, Value::Scalar(b"1".to_vec()), None).unwrap();
        assert!(store.delete(&config).unwrap());
        assert!(store.get(&config).is_none());
    }

    #[test]
    fn test_store_in_memory_client_not_configured() {
        let store = Store::new();
        let config = in_memory_config("k");
        assert!(store.set(&config, Value::Scalar(b"1".to_vec()), None).is_err());
        assert!(store.delete(&config).is_err());
    }

    // --- KVS ---

    struct MockKVS {
        store: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
    }
    impl MockKVS {
        fn new() -> Self { Self { store: std::sync::Mutex::new(std::collections::HashMap::new()) } }
    }
    impl KVSClient for MockKVS {
        fn get(&self, key: &str) -> Option<Vec<u8>> { self.store.lock().unwrap().get(key).cloned() }
        fn set(&self, key: &str, value: Vec<u8>, _ttl: Option<u64>) -> bool { self.store.lock().unwrap().insert(key.to_string(), value); true }
        fn delete(&self, key: &str) -> bool { self.store.lock().unwrap().remove(key).is_some() }
    }

    fn kvs_config(key: &str) -> HashMap<String, Value> {
        let mut c = HashMap::new();
        c.insert("client".to_string(), client_config(fixed_bits::CLIENT_KVS));
        c.insert("key".to_string(), Value::Scalar(key.as_bytes().to_vec()));
        c
    }

    #[test]
    fn test_store_kvs_set_and_get() {
        let client = Arc::new(MockKVS::new());
        let store = Store::new().with_kvs(client);
        let config = kvs_config("k");
        let data = Value::Scalar(b"hello".to_vec());
        assert!(store.set(&config, data.clone(), None).unwrap());
        assert_eq!(store.get(&config).unwrap(), Value::Scalar(b"hello".to_vec()));
    }

    #[test]
    fn test_store_kvs_set_uses_ttl_from_config() {
        let client = Arc::new(MockKVS::new());
        let store = Store::new().with_kvs(client);
        let mut config = kvs_config("k");
        config.insert("ttl".to_string(), Value::Scalar(3600u64.to_le_bytes().to_vec()));
        assert!(store.set(&config, Value::Scalar(b"1".to_vec()), None).unwrap());
    }

    #[test]
    fn test_store_kvs_delete() {
        let client = Arc::new(MockKVS::new());
        let store = Store::new().with_kvs(client);
        let config = kvs_config("k");
        store.set(&config, Value::Scalar(b"1".to_vec()), None).unwrap();
        assert!(store.delete(&config).unwrap());
        assert!(store.get(&config).is_none());
    }

    #[test]
    fn test_store_kvs_client_not_configured() {
        let store = Store::new();
        let config = kvs_config("k");
        assert!(store.set(&config, Value::Scalar(b"1".to_vec()), None).is_err());
        assert!(store.delete(&config).is_err());
    }

    // --- HTTP ---

    struct MockHttp {
        store: std::sync::Mutex<std::collections::HashMap<String, Value>>,
    }
    impl MockHttp {
        fn new() -> Self { Self { store: std::sync::Mutex::new(std::collections::HashMap::new()) } }
    }
    impl HttpClient for MockHttp {
        fn get(&self, url: &str, keys: &[Vec<u8>], _: Option<&[(Vec<u8>, Vec<u8>)]>) -> Option<Vec<Value>> {
            let stored = self.store.lock().unwrap().get(url).cloned()?;
            Some(keys.iter().map(|k| match &stored {
                Value::Mapping(m) => m.iter().find(|(mk, _)| mk == k).map(|(_, v)| v.clone()).unwrap_or(Value::Null),
                _ => stored.clone(),
            }).collect())
        }
        fn set(&self, url: &str, value: Value, _: Option<&[(Vec<u8>, Vec<u8>)]>) -> bool {
            self.store.lock().unwrap().insert(url.to_string(), value); true
        }
        fn delete(&self, url: &str, _: Option<&[(Vec<u8>, Vec<u8>)]>) -> bool {
            self.store.lock().unwrap().remove(url).is_some()
        }
    }

    fn http_config(url: &str) -> HashMap<String, Value> {
        let mut c = HashMap::new();
        c.insert("client".to_string(), client_config(fixed_bits::CLIENT_HTTP));
        c.insert("url".to_string(), Value::Scalar(url.as_bytes().to_vec()));
        c.insert("yaml_keys".to_string(), Value::Sequence(vec![Value::Scalar(b"status".to_vec())]));
        c.insert("ext_keys".to_string(),  Value::Sequence(vec![Value::Scalar(b"status".to_vec())]));
        c
    }

    #[test]
    fn test_store_http_set_and_get() {
        let client = Arc::new(MockHttp::new());
        let store = Store::new().with_http(client);
        let config = http_config("http://example.com/data");
        let data = Value::Mapping(vec![(b"status".to_vec(), Value::Scalar(b"ok".to_vec()))]);
        assert!(store.set(&config, data, None).unwrap());
        let result = store.get(&config).unwrap();
        let expected = Value::Mapping(vec![(b"status".to_vec(), Value::Scalar(b"ok".to_vec()))]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_store_http_delete() {
        let client = Arc::new(MockHttp::new());
        let store = Store::new().with_http(client);
        let config = http_config("http://example.com/data");
        store.set(&config, Value::Mapping(vec![(b"status".to_vec(), Value::Scalar(b"ok".to_vec()))]), None).unwrap();
        assert!(store.delete(&config).unwrap());
    }

    #[test]
    fn test_store_http_client_not_configured() {
        let store = Store::new();
        let config = http_config("http://example.com/data");
        assert!(store.set(&config, Value::Scalar(b"x".to_vec()), None).is_err());
        assert!(store.delete(&config).is_err());
    }
}
