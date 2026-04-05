// Request-scoped context handle. Manages state per DSL definition.
pub trait Context {
    /// Returns value from instance cache → _store, triggers _load on miss.
    fn get(&mut self, key: &str) -> Result<Option<Tree>, ContextError>;

    /// Writes value to _store. Returns Ok(false) if no _store is configured.
    fn set(&mut self, key: &str, value: Tree) -> Result<bool, ContextError>;

    /// Removes value from _store.
    fn delete(&mut self, key: &str) -> Result<bool, ContextError>;

    /// Checks existence in cache or _store. Does not trigger _load.
    fn exists(&mut self, key: &str) -> Result<bool, ContextError>;
}

// The value type used throughout context-engine's public API.
#[derive(Debug, PartialEq, Clone)]
pub enum Tree {
    Scalar(Vec<u8>),
    Sequence(Vec<Tree>),
    Mapping(Vec<(Vec<u8>, Tree)>),
    Null,
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum ParseError {
    FileNotFound(String),
    AmbiguousFile(String),
    ParseError(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::FileNotFound(msg)  => write!(f, "FileNotFound: {}", msg),
            ParseError::AmbiguousFile(msg) => write!(f, "AmbiguousFile: {}", msg),
            ParseError::ParseError(msg)    => write!(f, "ParseError: {}", msg),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum LoadError {
    /// StoreRegistry::client_for() returned None for the given yaml_name.
    ClientNotFound(String),
    /// A required config key is missing in the manifest.
    ConfigMissing(String),
    /// The client call succeeded but returned no data.
    NotFound(String),
    /// Parse error from client response.
    ParseError(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::ClientNotFound(msg) => write!(f, "ClientNotFound: {}", msg),
            LoadError::ConfigMissing(msg)  => write!(f, "ConfigMissing: {}", msg),
            LoadError::NotFound(msg)       => write!(f, "NotFound: {}", msg),
            LoadError::ParseError(msg)     => write!(f, "ParseError: {}", msg),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum StoreError {
    /// StoreRegistry::client_for() returned None for the given yaml_name.
    ClientNotFound(String),
    /// A required config key is missing in the manifest.
    ConfigMissing(String),
    /// Serialize error.
    SerializeError(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::ClientNotFound(msg) => write!(f, "ClientNotFound: {}", msg),
            StoreError::ConfigMissing(msg)  => write!(f, "ConfigMissing: {}", msg),
            StoreError::SerializeError(msg) => write!(f, "SerializeError: {}", msg),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ContextError {
    ParseFailed(String),
    KeyNotFound(String),
    RecursionLimitExceeded,
    StoreFailed(StoreError),
    LoadFailed(LoadError),
}

impl std::fmt::Display for ContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextError::ParseFailed(msg)       => write!(f, "ParseFailed: {}", msg),
            ContextError::KeyNotFound(msg)       => write!(f, "KeyNotFound: {}", msg),
            ContextError::RecursionLimitExceeded => write!(f, "RecursionLimitExceeded"),
            ContextError::StoreFailed(e)         => write!(f, "StoreFailed: {}", e),
            ContextError::LoadFailed(e)          => write!(f, "LoadFailed: {}", e),
        }
    }
}
