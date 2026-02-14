/// ENVClient implementation
///
/// Implements the ENVClient Required Port.
/// Provides access to environment variables.

use state_engine::ports::required::ENVClient;

pub struct ENVAdapter;

impl ENVAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl ENVClient for ENVAdapter {
    fn get(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}
