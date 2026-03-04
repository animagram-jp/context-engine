/// EnvClient implementation
///
/// Implements the EnvClient Required Port.
/// Provides access to environment variables.

use state_engine::ports::required::EnvClient;

pub struct EnvAdapter;

impl EnvAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl EnvClient for EnvAdapter {
    fn get(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}
