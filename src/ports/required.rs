use serde_json::Value;
use std::collections::HashMap;

/// In-process memory store. Internal mutability is the implementor's responsibility.
pub trait InMemoryClient: Send + Sync {
    fn get(&self, key: &str) -> Option<Value>;
    fn set(&self, key: &str, value: Value) -> bool;
    fn delete(&self, key: &str) -> bool;
}

/// KVS store. Serialization/deserialization is handled by the State layer.
/// Internal mutability is the implementor's responsibility.
pub trait KVSClient: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    /// `ttl` in seconds.
    fn set(&self, key: &str, value: String, ttl: Option<u64>) -> bool;
    fn delete(&self, key: &str) -> bool;
}

/// Environment / config store.
/// Internal mutability is the implementor's responsibility.
pub trait EnvClient: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: String) -> bool;
    fn delete(&self, key: &str) -> bool;
}

/// Relational DB client.
/// Do NOT call State inside DbClient — it would cause recursion.
/// `connection` is a Value::Object resolved from the manifest.
/// `columns` are extracted from the manifest `map` definition.
pub trait DbClient: Send + Sync {
    fn get(
        &self,
        connection: &Value,
        table: &str,
        columns: &[&str],
        where_clause: Option<&str>,
    ) -> Option<Vec<HashMap<String, Value>>>;
    fn set(
        &self,
        connection: &Value,
        table: &str,
        values: &HashMap<String, Value>,
        where_clause: Option<&str>,
    ) -> bool;
    fn delete(
        &self,
        connection: &Value,
        table: &str,
        where_clause: Option<&str>,
    ) -> bool;
}

/// HTTP client.
/// `headers` is an optional map of header name → value.
pub trait HttpClient: Send + Sync {
    fn get(
        &self,
        url: &str,
        headers: Option<&HashMap<String, String>>,
    ) -> Option<Value>;
    fn set(
        &self,
        url: &str,
        body: Value,
        headers: Option<&HashMap<String, String>>,
    ) -> bool;
    fn delete(
        &self,
        url: &str,
        headers: Option<&HashMap<String, String>>,
    ) -> bool;
}

/// File client. `map` drives field extraction from file contents.
pub trait FileClient: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: String) -> bool;
    fn delete(&self, key: &str) -> bool;
}
