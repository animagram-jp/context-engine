#![no_std]
extern crate alloc;
extern crate core;

#[cfg(any(feature = "precompile", test))]
extern crate std;

pub mod debug_log;
#[doc(hidden)]
pub use alloc::{string::String, vec, vec::Vec};
pub mod required;
pub mod list;
pub mod tree;
pub mod dsl;
pub mod index;
pub mod context;
pub mod provided;

pub use index::Index;
pub use list::{List, VariableList};
pub use provided::{Context, ContextError, DslError, LoadError, StoreError, Tree};
pub use required::{SetOutcome, Store, Stores};
