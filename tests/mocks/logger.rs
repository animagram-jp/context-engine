// MockLogger - Simple logger for testing
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct MockLogger {
    logs: Arc<Mutex<Vec<String>>>,
}

#[allow(dead_code)]
impl MockLogger {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn info(&self, message: &str) {
        self.logs.lock().unwrap().push(format!("INFO: {}", message));
    }

    pub fn error(&self, message: &str) {
        self.logs.lock().unwrap().push(format!("ERROR: {}", message));
    }

    pub fn get_logs(&self) -> Vec<String> {
        self.logs.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.logs.lock().unwrap().clear();
    }
}

impl Default for MockLogger {
    fn default() -> Self {
        Self::new()
    }
}
