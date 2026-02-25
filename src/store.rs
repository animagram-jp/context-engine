// Store module - handles InMemory and KVS client operations
use crate::ports::required::{InMemoryClient, KVSClient};
use crate::common::bit;
use serde_json::Value;
use std::collections::HashMap;

/// Store manages data persistence to InMemory and KVS clients
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
    ///
    /// store_config format:
    /// - client: "InMemory" or "KVS"
    /// - key: storage key
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
    ///
    /// store_config format:
    /// - client: "InMemory" or "KVS"
    /// - key: storage key
    /// - ttl: (optional) time to live in seconds (KVS only)
    pub fn set(
        &mut self,
        store_config: &HashMap<String, Value>,
        value: Value,
        ttl: Option<u64>,
    ) -> bool {
        let client = match store_config.get("client").and_then(|v| v.as_u64()) {
            Some(c) => c,
            None => return false,
        };

        match client {
            bit::CLIENT_IN_MEMORY => {
                if let Some(in_memory) = self.in_memory.as_mut() {
                    if let Some(key) = store_config.get("key").and_then(|v| v.as_str()) {
                        in_memory.set(key, value);
                        return true;
                    }
                }
                false
            }
            bit::CLIENT_KVS => {
                if let Some(kvs_client) = self.kvs_client.as_mut() {
                    if let Some(key) = store_config.get("key").and_then(|v| v.as_str()) {
                        // serialize
                        let serialized = match serde_json::to_string(&value) {
                            Ok(s) => s,
                            Err(_) => return false,
                        };

                        let final_ttl =
                            ttl.or_else(|| store_config.get("ttl").and_then(|v| v.as_u64()));
                        return kvs_client.set(key, serialized, final_ttl);
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Delete value from store based on store_config
    ///
    /// store_config format:
    /// - client: "InMemory" or "KVS"
    /// - key: storage key
    pub fn delete(&mut self, store_config: &HashMap<String, Value>) -> bool {
        let client = match store_config.get("client").and_then(|v| v.as_u64()) {
            Some(c) => c,
            None => return false,
        };

        match client {
            bit::CLIENT_IN_MEMORY => {
                if let Some(in_memory) = self.in_memory.as_mut() {
                    if let Some(key) = store_config.get("key").and_then(|v| v.as_str()) {
                        return in_memory.delete(key);
                    }
                }
                false
            }
            bit::CLIENT_KVS => {
                if let Some(kvs_client) = self.kvs_client.as_mut() {
                    if let Some(key) = store_config.get("key").and_then(|v| v.as_str()) {
                        return kvs_client.delete(key);
                    }
                }
                false
            }
            _ => false,
        }
    }
}

impl<'a> Default for Store<'a> {
    fn default() -> Self {
        Self::new()
    }
}
