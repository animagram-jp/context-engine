use crate::ports::required::{InMemoryClient, KVSClient, HttpClient, FileClient};
use core::fixed_bits;
use serde_json::Value;
use std::collections::HashMap;

pub struct Store<'a> {
    in_memory: Option<&'a dyn InMemoryClient>,
    kvs: Option<&'a dyn KVSClient>,
    http: Option<&'a dyn HttpClient>,
    file: Option<&'a dyn FileClient>,
}

impl<'a> Store<'a> {
    pub fn new() -> Self {
        Self {
            in_memory: None,
            kvs: None,
            http: None,
            file: None,
        }
    }

    pub fn with_in_memory(mut self, client: &'a dyn InMemoryClient) -> Self {
        self.in_memory = Some(client);
        self
    }

    pub fn with_kvs(mut self, client: &'a dyn KVSClient) -> Self {
        self.kvs = Some(client);
        self
    }

    pub fn with_http(mut self, client: &'a dyn HttpClient) -> Self {
        self.http = Some(client);
        self
    }

    pub fn with_file(mut self, client: &'a dyn FileClient) -> Self {
        self.file = Some(client);
        self
    }

    pub fn get(&self, store_config: &HashMap<String, Value>) -> Option<Value> {
        let client = store_config.get("client")?.as_u64()?;

        match client {
            fixed_bits::CLIENT_IN_MEMORY => {
                let in_memory = self.in_memory.as_ref()?;
                let key = store_config.get("key")?.as_str()?;
                in_memory.get(key)
            }
            fixed_bits::CLIENT_KVS => {
                let kvs = self.kvs.as_ref()?;
                let key = store_config.get("key")?.as_str()?;
                let value_str = kvs.get(key)?;
                serde_json::from_str(&value_str).ok()
            }
            fixed_bits::CLIENT_HTTP => {
                let http = self.http.as_ref()?;
                let url = store_config.get("url")?.as_str()?;
                let headers = store_config
                    .get("headers")
                    .and_then(|v| v.as_object())
                    .map(|obj| obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect::<HashMap<String, String>>());
                http.get(url, headers.as_ref())
            }
            fixed_bits::CLIENT_FILE => {
                let file = self.file.as_ref()?;
                let key = store_config.get("key")?.as_str()?;
                let content = file.get(key)?;
                serde_json::from_str(&content).ok()
            }
            _ => None,
        }
    }

    pub fn set(
        &self,
        store_config: &HashMap<String, Value>,
        value: Value,
        ttl: Option<u64>,
    ) -> Result<bool, String> {
        let client = store_config
            .get("client")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| "Store::set: 'client' not found in store config".to_string())?;

        match client {
            fixed_bits::CLIENT_IN_MEMORY => {
                let in_memory = self.in_memory.as_ref()
                    .ok_or_else(|| "Store::set: InMemoryClient not configured".to_string())?;
                let key = store_config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| "Store::set: 'key' not found in store config".to_string())?;
                Ok(in_memory.set(key, value))
            }
            fixed_bits::CLIENT_KVS => {
                let kvs = self.kvs.as_ref()
                    .ok_or_else(|| "Store::set: KVSClient not configured".to_string())?;
                let key = store_config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| "Store::set: 'key' not found in store config".to_string())?;
                let serialized = serde_json::to_string(&value)
                    .map_err(|e| format!("Store::set: JSON serialize error: {}", e))?;
                let final_ttl = ttl.or_else(|| store_config.get("ttl").and_then(|v| v.as_u64()));
                Ok(kvs.set(key, serialized, final_ttl))
            }
            fixed_bits::CLIENT_HTTP => {
                let http = self.http.as_ref()
                    .ok_or_else(|| "Store::set: HttpClient not configured".to_string())?;
                let url = store_config.get("url").and_then(|v| v.as_str())
                    .ok_or_else(|| "Store::set: 'url' not found in store config".to_string())?;
                let headers = store_config
                    .get("headers")
                    .and_then(|v| v.as_object())
                    .map(|obj| obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect::<HashMap<String, String>>());
                Ok(http.set(url, value, headers.as_ref()))
            }
            fixed_bits::CLIENT_FILE => {
                let file = self.file.as_ref()
                    .ok_or_else(|| "Store::set: FileClient not configured".to_string())?;
                let key = store_config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| "Store::set: 'key' not found in store config".to_string())?;
                let serialized = serde_json::to_string(&value)
                    .map_err(|e| format!("Store::set: JSON serialize error: {}", e))?;
                Ok(file.set(key, serialized))
            }
            _ => Err(format!("Store::set: unsupported client '{}'", client)),
        }
    }

    pub fn delete(&self, store_config: &HashMap<String, Value>) -> Result<bool, String> {
        let client = store_config
            .get("client")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| "Store::delete: 'client' not found in store config".to_string())?;

        match client {
            fixed_bits::CLIENT_IN_MEMORY => {
                let in_memory = self.in_memory.as_ref()
                    .ok_or_else(|| "Store::delete: InMemoryClient not configured".to_string())?;
                let key = store_config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| "Store::delete: 'key' not found in store config".to_string())?;
                Ok(in_memory.delete(key))
            }
            fixed_bits::CLIENT_KVS => {
                let kvs = self.kvs.as_ref()
                    .ok_or_else(|| "Store::delete: KVSClient not configured".to_string())?;
                let key = store_config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| "Store::delete: 'key' not found in store config".to_string())?;
                Ok(kvs.delete(key))
            }
            fixed_bits::CLIENT_HTTP => {
                let http = self.http.as_ref()
                    .ok_or_else(|| "Store::delete: HttpClient not configured".to_string())?;
                let url = store_config.get("url").and_then(|v| v.as_str())
                    .ok_or_else(|| "Store::delete: 'url' not found in store config".to_string())?;
                let headers = store_config
                    .get("headers")
                    .and_then(|v| v.as_object())
                    .map(|obj| obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect::<HashMap<String, String>>());
                Ok(http.delete(url, headers.as_ref()))
            }
            fixed_bits::CLIENT_FILE => {
                let file = self.file.as_ref()
                    .ok_or_else(|| "Store::delete: FileClient not configured".to_string())?;
                let key = store_config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| "Store::delete: 'key' not found in store config".to_string())?;
                Ok(file.delete(key))
            }
            _ => Err(format!("Store::delete: unsupported client '{}'", client)),
        }
    }
}

impl<'a> Default for Store<'a> {
    fn default() -> Self {
        Self::new()
    }
}
