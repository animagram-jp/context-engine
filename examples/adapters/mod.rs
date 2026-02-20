/// Sample adapters for state-engine
///
/// These implementations demonstrate how to implement the Required Ports.

pub mod in_memory;
pub mod env_client;
pub mod kvs_client;
pub mod db_client;

pub use in_memory::InMemoryAdapter;
pub use env_client::EnvAdapter;
pub use kvs_client::KVSAdapter;
pub use db_client::DBAdapter;
