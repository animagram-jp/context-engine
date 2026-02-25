use serde_json::Value;
use std::collections::HashMap;

pub trait InMemoryClient: Send + Sync {
    fn get(&self, key: &str) -> Option<Value>;
    fn set(&mut self, key: &str, value: Value);
    fn delete(&mut self, key: &str) -> bool;
}

/// `connection` is a Value::Object (connection config) resolved from manifest.
/// Do NOT call State inside DbClient — it would cause recursion.
pub trait DbClient: Send + Sync {
    /// `columns` are extracted from the manifest `map` definition.
    fn fetch(
        &self,
        connection: &Value,
        table: &str,
        columns: &[&str],
        where_clause: Option<&str>,
    ) -> Option<Vec<HashMap<String, Value>>>;
}

/// KVS stores serialized strings only. State layer handles serialize/deserialize.
pub trait KVSClient: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    /// `ttl` in seconds.
    fn set(&mut self, key: &str, value: String, ttl: Option<u64>) -> bool;
    fn delete(&mut self, key: &str) -> bool;
}

pub trait EnvClient: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
}
