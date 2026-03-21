use serde_json::Value;

#[derive(Debug, PartialEq)]
pub enum ManifestError {
    FileNotFound(String),
    AmbiguousFile(String),
    ParseError(String),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::FileNotFound(msg)  => write!(f, "FileNotFound: {}", msg),
            ManifestError::AmbiguousFile(msg) => write!(f, "AmbiguousFile: {}", msg),
            ManifestError::ParseError(msg)    => write!(f, "ParseError: {}", msg),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum LoadError {
    /// Required client (Env/KVS/DB/HTTP/File) is not configured.
    ClientNotConfigured,
    /// A required config key (key/url/table/map/connection) is missing.
    ConfigMissing(String),
    /// The client call succeeded but returned no data.
    NotFound(String),
    /// JSON parse error from client response.
    ParseError(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::ClientNotConfigured      => write!(f, "ClientNotConfigured"),
            LoadError::ConfigMissing(msg)       => write!(f, "ConfigMissing: {}", msg),
            LoadError::NotFound(msg)            => write!(f, "NotFound: {}", msg),
            LoadError::ParseError(msg)          => write!(f, "ParseError: {}", msg),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum StoreError {
    /// Required client (KVS/InMemory/HTTP/File) is not configured.
    ClientNotConfigured,
    /// A required config key (key/url/client) is missing.
    ConfigMissing(String),
    /// JSON serialize error.
    SerializeError(String),
    /// Unsupported client id in config.
    UnsupportedClient(u64),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::ClientNotConfigured    => write!(f, "ClientNotConfigured"),
            StoreError::ConfigMissing(msg)     => write!(f, "ConfigMissing: {}", msg),
            StoreError::SerializeError(msg)    => write!(f, "SerializeError: {}", msg),
            StoreError::UnsupportedClient(id)  => write!(f, "UnsupportedClient: {}", id),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum StateError {
    ManifestLoadFailed(String),
    KeyNotFound(String),
    RecursionLimitExceeded,
    StoreFailed(StoreError),
    LoadFailed(LoadError),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateError::ManifestLoadFailed(msg)  => write!(f, "ManifestLoadFailed: {}", msg),
            StateError::KeyNotFound(msg)          => write!(f, "KeyNotFound: {}", msg),
            StateError::RecursionLimitExceeded    => write!(f, "RecursionLimitExceeded"),
            StateError::StoreFailed(e)            => write!(f, "StoreFailed: {}", e),
            StateError::LoadFailed(e)             => write!(f, "LoadFailed: {}", e),
        }
    }
}

/// The primary interface for state-engine. Manages state per manifest definition.
pub trait State {
    /// Returns value from _store, or triggers _load on miss.
    fn get(&mut self, key: &str) -> Result<Option<Value>, StateError>;

    /// Writes value to _store. Returns Ok(false) if no _store is configured.
    /// `ttl` overrides manifest definition (KVS only).
    fn set(&mut self, key: &str, value: Value, ttl: Option<u64>) -> Result<bool, StateError>;

    /// Removes value from _store.
    fn delete(&mut self, key: &str) -> Result<bool, StateError>;

    /// Checks existence in cache or _store. Does not trigger _load.
    fn exists(&mut self, key: &str) -> Result<bool, StateError>;
}
