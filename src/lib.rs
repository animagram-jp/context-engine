#![no_std]
extern crate core;
extern crate alloc;

// precompile feature requires std (file I/O, serde)
#[cfg(feature = "precompile")]
extern crate std;

pub mod log_format;
pub mod ports;
pub mod context;
pub mod tree;
pub mod dsl;
pub mod index;

pub use log_format::LogFormat;
pub use ports::provided::{
    Tree,
    ParseError, LoadError, StoreError, ContextError,
    Context,
};
pub use ports::required::{
    StoreClient,
    StoreRegistry,
    SetOutcome
};
