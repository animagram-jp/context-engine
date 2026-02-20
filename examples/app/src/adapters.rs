/// Re-export adapters from examples/adapters/
///
/// This allows the app to use the shared adapter implementations.

// Include the adapters module from parent directory
#[path = "../../adapters/mod.rs"]
mod adapters_impl;

pub use adapters_impl::{
    InMemoryAdapter,
    EnvAdapter,
    KVSAdapter,
    DBAdapter,
};
