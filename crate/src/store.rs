use crate::ports::required::{InMemoryClient, KVSClient};
use crate::common::bit;
use serde_json::Value;
use std::collections::HashMap;

pub struct Store<'a> {
    in_memory: Option<&'a mut dyn InMemoryClient>,
    kvs_client: Option<&'a mut dyn KVSClient>,
}

impl<'a> Store<'a> {
    pub fn new() -> Self {
        Self {
            in_memory: None,
            kvs_client: None,
        }
    }

    pub fn with_in_memory(mut self, client: &'a mut dyn InMemoryClient) -> Self {
        self.in_memory = Some(client);
        self
    }

    pub fn with_kvs_client(mut self, client: &'a mut dyn KVSClient) -> Self {
        self.kvs_client = Some(client);
        self
    }

    /// Get value from store based on store_config
    pub fn get(&self, store_config: &HashMap<String, Value>) -> Option<Value> {
        let client = store_config.get("client")?.as_u64()?;

        match client {
            bit::CLIENT_IN_MEMORY => {
                let in_memory = self.in_memory.as_ref()?;
                let key = store_config.get("key")?.as_str()?;
                in_memory.get(key)
            }
            bit::CLIENT_KVS => {
                let kvs_client = self.kvs_client.as_ref()?;
                let key = store_config.get("key")?.as_str()?;
                let value_str = kvs_client.get(key)?;

                // deserialize
                serde_json::from_str(&value_str).ok()
            }
            _ => None,
        }
    }

    /// Set value to store based on store_config
    pub fn set(
        &mut self,
        store_config: &HashMap<String, Value>,
        value: Value,
        ttl: Option<u64>,
    ) -> Result<bool, String> {
        let client = store_config
            .get("client")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| "Store::set: 'client' not found in store config".to_string())?;

        match client {
            bit::CLIENT_IN_MEMORY => {
                let in_memory = self.in_memory.as_mut()
                    .ok_or_else(|| "Store::set: InMemoryClient not configured".to_string())?;
                let key = store_config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| "Store::set: 'key' not found in store config".to_string())?;
                in_memory.set(key, value);
                Ok(true)
            }
            bit::CLIENT_KVS => {
                let kvs_client = self.kvs_client.as_mut()
                    .ok_or_else(|| "Store::set: KVSClient not configured".to_string())?;
                let key = store_config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| "Store::set: 'key' not found in store config".to_string())?;
                let serialized = serde_json::to_string(&value)
                    .map_err(|e| format!("Store::set: JSON serialize error: {}", e))?;
                let final_ttl = ttl.or_else(|| store_config.get("ttl").and_then(|v| v.as_u64()));
                Ok(kvs_client.set(key, serialized, final_ttl))
            }
            _ => Err(format!("Store::set: unsupported client '{}'", client)),
        }
    }

    /// Delete value from store based on store_config
    pub fn delete(&mut self, store_config: &HashMap<String, Value>) -> Result<bool, String> {
        let client = store_config
            .get("client")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| "Store::delete: 'client' not found in store config".to_string())?;

        match client {
            bit::CLIENT_IN_MEMORY => {
                let in_memory = self.in_memory.as_mut()
                    .ok_or_else(|| "Store::delete: InMemoryClient not configured".to_string())?;
                let key = store_config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| "Store::delete: 'key' not found in store config".to_string())?;
                Ok(in_memory.delete(key))
            }
            bit::CLIENT_KVS => {
                let kvs_client = self.kvs_client.as_mut()
                    .ok_or_else(|| "Store::delete: KVSClient not configured".to_string())?;
                let key = store_config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| "Store::delete: 'key' not found in store config".to_string())?;
                Ok(kvs_client.delete(key))
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
