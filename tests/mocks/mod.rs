// Mock implementations for testing
pub mod logger;
pub mod clients;

// Re-export commonly used mocks
pub use clients::{MockInMemory, MockKVS, MockENVClient};
