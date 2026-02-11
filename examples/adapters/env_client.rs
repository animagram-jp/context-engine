/// ENVClient implementation
///
/// Implements the ENVClient Required Port.
/// Provides access to environment variables.

use state_engine::ports::required::ENVClient;
use std::collections::HashMap;

pub struct ENVAdapter;

impl ENVAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Check if environment variable exists
    pub fn has(&self, key: &str) -> bool {
        std::env::var(key).is_ok()
    }

    /// Get all environment variables
    pub fn get_all(&self) -> HashMap<String, String> {
        std::env::vars().collect()
    }
}

impl ENVClient for ENVAdapter {
    fn get(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}
