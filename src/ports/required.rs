use crate::ports::provided::Value;

/// In-process memory store. Internal mutability is the implementor's responsibility.
pub trait InMemoryClient: Send + Sync {
    fn get(&self, key: &str) -> Option<Value>;
    fn set(&self, key: &str, value: Value) -> bool;
    fn delete(&self, key: &str) -> bool;
}

/// KVS store. Serialization/deserialization is handled by the adapter.
/// Internal mutability is the implementor's responsibility.
pub trait KVSClient: Send + Sync {
    fn get(&self, key: &str) -> Option<Vec<u8>>;
    /// `ttl` in seconds.
    fn set(&self, key: &str, value: Vec<u8>, ttl: Option<u64>) -> bool;
    fn delete(&self, key: &str) -> bool;
}

/// Environment / config store.
/// `keys` is the list of external key names (map values from manifest).
/// Returns values in the same order as `keys`.
/// Internal mutability is the implementor's responsibility.
pub trait EnvClient: Send + Sync {
    fn get(&self, keys: &[Vec<u8>]) -> Option<Vec<Value>>;
    fn set(&self, key: &str, value: Vec<u8>) -> bool;
    fn delete(&self, key: &str) -> bool;
}

/// Relational DB client.
/// Do NOT call State inside DbClient — it would cause recursion.
/// `connection` is a Value::Mapping resolved from the manifest.
/// `keys` is the list of db column names (map values from manifest).
/// Returns values in the same order as `keys`.
pub trait DbClient: Send + Sync {
    fn get(
        &self,
        connection: &Value,
        table: &str,
        keys: &[Vec<u8>],
        where_clause: Option<&[u8]>,
    ) -> Option<Vec<Value>>;
    fn set(
        &self,
        connection: &Value,
        table: &str,
        keys: &[Vec<u8>],
        where_clause: Option<&[u8]>,
    ) -> bool;
    fn delete(
        &self,
        connection: &Value,
        table: &str,
        where_clause: Option<&[u8]>,
    ) -> bool;
}

/// HTTP client.
/// `keys` is the list of response field names (map values from manifest).
/// Returns values in the same order as `keys`.
/// `headers` is an optional list of (name, value) byte pairs.
pub trait HttpClient: Send + Sync {
    fn get(
        &self,
        url: &str,
        keys: &[Vec<u8>],
        headers: Option<&[(Vec<u8>, Vec<u8>)]>,
    ) -> Option<Vec<Value>>;
    fn set(
        &self,
        url: &str,
        body: Value,
        headers: Option<&[(Vec<u8>, Vec<u8>)]>,
    ) -> bool;
    fn delete(
        &self,
        url: &str,
        headers: Option<&[(Vec<u8>, Vec<u8>)]>,
    ) -> bool;
}

/// File client.
pub trait FileClient: Send + Sync {
    fn get(&self, key: &str) -> Option<Vec<u8>>;
    fn set(&self, key: &str, value: Vec<u8>) -> bool;
    fn delete(&self, key: &str) -> bool;
}
