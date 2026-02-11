/// InMemoryClient implementation
///
/// Implements the InMemoryClient Required Port.
/// Manages in-memory key-value storage for the current process.

use serde_json::Value;
use std::collections::HashMap;
use state_engine::ports::required::InMemoryClient;

pub struct InMemoryAdapter {
    data: HashMap<String, Value>,
}

impl InMemoryAdapter {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Get all keys
    pub fn keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }
}

impl InMemoryClient for InMemoryAdapter {
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
