#[path = "../../adapters/mod.rs"]
mod adapters_impl;

pub use adapters_impl::{
    InMemoryAdapter,
    EnvAdapter,
    KVSAdapter,
    DbAdapter,
    HttpAdapter,
};
