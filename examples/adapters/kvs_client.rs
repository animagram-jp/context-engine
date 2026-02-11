/// KVSClient implementation using Redis
///
/// Implements the KVSClient Required Port.

use state_engine::ports::required::KVSClient;

pub struct KVSAdapter {
    client: redis::Client,
}

impl KVSAdapter {
    pub fn new() -> Result<Self, String> {
        let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string());
        let url = format!("redis://{}:{}", host, port);

        let client = redis::Client::open(url)
            .map_err(|e| format!("Failed to create Redis client: {}", e))?;

        Ok(Self { client })
    }

    /// Get TTL for a key
    pub fn ttl(&mut self, key: &str) -> Result<i64, String> {
        let mut conn = self.client.get_connection()
            .map_err(|e| format!("Failed to connect to Redis: {}", e))?;

        redis::cmd("TTL")
            .arg(key)
            .query(&mut conn)
            .map_err(|e| format!("Failed to get TTL: {}", e))
    }
}

impl KVSClient for KVSAdapter {
    fn get(&self, key: &str) -> Option<String> {
        let mut conn = self.client.get_connection().ok()?;
        redis::cmd("GET")
            .arg(key)
            .query::<Option<String>>(&mut conn)
            .ok()
            .flatten()
    }

    fn set(&mut self, key: &str, value: String, ttl: Option<u64>) -> bool {
        let mut conn = match self.client.get_connection() {
            Ok(c) => c,
            Err(_) => return false,
        };

        let result: Result<(), _> = if let Some(ttl_secs) = ttl {
            redis::cmd("SETEX")
                .arg(key)
                .arg(ttl_secs)
                .arg(value)
                .query(&mut conn)
        } else {
            redis::cmd("SET")
                .arg(key)
                .arg(value)
                .query(&mut conn)
        };

        result.is_ok()
    }

    fn delete(&mut self, key: &str) -> bool {
        let mut conn = match self.client.get_connection() {
            Ok(c) => c,
            Err(_) => return false,
        };

        let result: Result<i32, _> = redis::cmd("DEL")
            .arg(key)
            .query(&mut conn);

        result.map(|count| count > 0).unwrap_or(false)
    }
}
