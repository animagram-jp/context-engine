// Mock client implementations for testing
use state_engine::ports::required::{InMemoryClient, KVSClient, EnvClient};
use serde_json::Value;
use std::collections::HashMap;

// Mock InMemoryClient
pub struct MockInMemory {
    pub data: HashMap<String, Value>,
}

impl MockInMemory {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl InMemoryClient for MockInMemory {
    fn get(&self, key: &str) -> Option<Value> {
        self.data.get(key).cloned()
    }

    fn set(&mut self, key: &str, value: Value) {
        self.data.insert(key.to_string(), value);
    }

    fn delete(&mut self, key: &str) -> bool {
        self.data.remove(key).is_some()
    }
}

// Mock KVSClient
pub struct MockKVS {
    pub data: HashMap<String, String>,
}

impl MockKVS {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl KVSClient for MockKVS {
    fn get(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }

    fn set(&mut self, key: &str, value: String, _ttl: Option<u64>) -> bool {
        self.data.insert(key.to_string(), value);
        true
    }

    fn delete(&mut self, key: &str) -> bool {
        self.data.remove(key).is_some()
    }
}

// Mock EnvClient
pub struct MockEnvClient {
    pub data: HashMap<String, String>,
}

impl MockEnvClient {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl EnvClient for MockEnvClient {
    fn get(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }
}
