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
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        std::env::var(key).ok().map(|s| s.into_bytes())
    }

    fn set(&self, _key: &str, _value: Vec<u8>) -> bool { false }
    fn delete(&self, _key: &str) -> bool { false }
}
