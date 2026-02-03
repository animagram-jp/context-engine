// state-engine library
// Conductor向けマルチストアステート管理ライブラリ

pub mod common;
pub mod manifest;
pub mod ports;
pub mod load;
pub mod state;

// Re-export main types
pub use common::{DotArrayAccessor, PlaceholderResolver};
pub use manifest::Manifest;
pub use ports::provided::State as StateTrait;
pub use state::{State, resolver::Resolver};
pub use load::Load;

// Re-export all Required Ports for app implementation
pub use ports::required::{
    APIClient, ConnectionConfig, DBClient, DBConnectionConfigConverter, ENVClient,
    ExpressionClient, KVSClient, ProcessMemoryClient,
};

// TODO: 以下のモジュールを実装
// pub mod load;
// pub mod state;
