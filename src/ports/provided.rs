use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum ManifestError {
    FileNotFound(String),
    AmbiguousFile(String),
    ReadError(String),
    ParseError(String),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::FileNotFound(msg)  => write!(f, "FileNotFound: {}", msg),
            ManifestError::AmbiguousFile(msg) => write!(f, "AmbiguousFile: {}", msg),
            ManifestError::ReadError(msg)     => write!(f, "ReadError: {}", msg),
            ManifestError::ParseError(msg)    => write!(f, "ParseError: {}", msg),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum StateError {
    ManifestLoadFailed(String),
    KeyNotFound(String),
    RecursionLimitExceeded,
    StoreFailed(String),
    LoadFailed(String),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateError::ManifestLoadFailed(msg)  => write!(f, "ManifestLoadFailed: {}", msg),
            StateError::KeyNotFound(msg)          => write!(f, "KeyNotFound: {}", msg),
            StateError::RecursionLimitExceeded    => write!(f, "RecursionLimitExceeded"),
            StateError::StoreFailed(msg)          => write!(f, "StoreFailed: {}", msg),
            StateError::LoadFailed(msg)           => write!(f, "LoadFailed: {}", msg),
        }
    }
}

pub trait Manifest {
    fn get_value(&mut self, key: &str, default: Option<Value>) -> Value;
    fn get_meta(&mut self, key: &str) -> HashMap<String, Value>;
    fn load_file(&mut self, file: &str) -> Result<(), ManifestError>;
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
