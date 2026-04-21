use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Request-scoped context handle. Manages state per DSL definition.
pub trait Context {
    /// Returns value from instance cache → _set, triggers _get on miss.
    fn get(&mut self, key: &str) -> Result<Option<Tree>, ContextError>;

    /// Writes value to _set. Returns Ok(false) if no _set is configured.
    fn set(&mut self, key: &str, value: Tree) -> Result<bool, ContextError>;

    /// Removes value from _set.
    fn delete(&mut self, key: &str) -> Result<bool, ContextError>;

    /// Checks existence in cache or _set. Does not trigger _get.
    fn exists(&mut self, key: &str) -> Result<bool, ContextError>;
}

/// The value type used throughout context-engine's public API.
#[derive(Debug, PartialEq, Clone)]
pub enum Tree {
    Scalar(Vec<u8>),
    Sequence(Vec<Tree>),
    Mapping(Vec<(Vec<u8>, Tree)>),
    Null,
}

// ── Errors ────────────────────────────────────────────────────────────────────

/// DSL parse/compile/file errors returned by `Dsl::compile` and `Dsl::write`.
#[derive(Debug, PartialEq)]
pub enum DslError {
    FileNotFound(String),
    AmbiguousFile(String),
    ParseError(String),
    /// A compile-time limit was exceeded (path_id, word_id, store_id, or data size).
    LimitExceeded(String),
}

impl fmt::Display for DslError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DslError::FileNotFound(msg)   => write!(f, "FileNotFound: {}", msg),
            DslError::AmbiguousFile(msg)  => write!(f, "AmbiguousFile: {}", msg),
            DslError::ParseError(msg)     => write!(f, "ParseError: {}", msg),
            DslError::LimitExceeded(msg)  => write!(f, "LimitExceeded: {}", msg),
        }
    }
}

/// Errors from `_get` store resolution during `Context::get`.
#[derive(Debug, PartialEq)]
pub enum LoadError {
    /// Stores::store_for() returned None for the given store_id.
    ClientNotFound(String),
    /// A required config key is missing in the manifest.
    ConfigMissing(String),
    /// The store call succeeded but returned no data.
    NotFound(String),
    /// Parse error from store response.
    ParseError(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::ClientNotFound(msg) => write!(f, "ClientNotFound: {}", msg),
            LoadError::ConfigMissing(msg)  => write!(f, "ConfigMissing: {}", msg),
            LoadError::NotFound(msg)       => write!(f, "NotFound: {}", msg),
            LoadError::ParseError(msg)     => write!(f, "ParseError: {}", msg),
        }
    }
}

/// Errors from `_set` store operations during `Context::set` / `Context::delete`.
#[derive(Debug, PartialEq)]
pub enum StoreError {
    /// Stores::store_for() returned None for the given store_id.
    ClientNotFound(String),
    /// A required config key is missing in the manifest.
    ConfigMissing(String),
    /// Serialize error.
    SerializeError(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::ClientNotFound(msg) => write!(f, "ClientNotFound: {}", msg),
            StoreError::ConfigMissing(msg)  => write!(f, "ConfigMissing: {}", msg),
            StoreError::SerializeError(msg) => write!(f, "SerializeError: {}", msg),
        }
    }
}

/// Top-level errors returned by all `Context` methods.
#[derive(Debug, PartialEq)]
pub enum ContextError {
    ParseFailed(String),
    KeyNotFound(String),
    RecursionLimitExceeded,
    StoreFailed(StoreError),
    LoadFailed(LoadError),
}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContextError::ParseFailed(msg)       => write!(f, "ParseFailed: {}", msg),
            ContextError::KeyNotFound(msg)       => write!(f, "KeyNotFound: {}", msg),
            ContextError::RecursionLimitExceeded => write!(f, "RecursionLimitExceeded"),
            ContextError::StoreFailed(e)         => write!(f, "StoreFailed: {}", e),
            ContextError::LoadFailed(e)          => write!(f, "LoadFailed: {}", e),
        }
    }
}
