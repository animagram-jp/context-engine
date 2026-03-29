/// InMemoryClient implementation
///
/// Implements the InMemoryClient Required Port.
/// Manages in-memory key-value storage for the current process.

use state_engine::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use state_engine::ports::required::InMemoryClient;

pub struct InMemoryAdapter {
    data: Mutex<HashMap<String, Value>>,
}

impl InMemoryAdapter {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl InMemoryClient for InMemoryAdapter {
    fn get(&self, key: &str) -> Option<Value> {
        self.data.lock().unwrap().get(key).cloned()
    }

    fn set(&self, key: &str, value: Value) -> bool {
        self.data.lock().unwrap().insert(key.to_string(), value);
        true
    }

    fn delete(&self, key: &str) -> bool {
        self.data.lock().unwrap().remove(key).is_some()
    }
}
