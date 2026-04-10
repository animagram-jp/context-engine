use alloc::collections::BTreeMap;
use crate::ports::provided::Tree;

/// Outcome of a `StoreClient::set` call.
pub enum SetOutcome {
    Created,
    Updated,
}

/// Single-store adapter. Implemented by the library user per backing store.
///
/// - `key`:  the value of `_load.key` / `_store.key` from the manifest. Reserved arg.
/// - `map`:  ordered list of `(dst_path, src_column)` pairs from `map:` in the manifest.
///           Empty slice if `map:` is not defined.
/// - `args`: all other manifest args (ttl, connection, headers, etc.) as a flat map.
///           The implementor defines and reads whatever keys it needs.
/// - Thread-safety and internal mutability are the implementor's responsibility.
pub trait StoreClient: Send + Sync {
    fn get(&self, key: &str, map: &[(Tree, Tree)], args: &BTreeMap<&str, Tree>) -> Option<Tree>;
    fn set(&self, key: &str, map: &[(Tree, Tree)], args: &BTreeMap<&str, Tree>) -> Option<SetOutcome>;
    fn delete(&self, key: &str, map: &[(Tree, Tree)], args: &BTreeMap<&str, Tree>) -> bool;
}

/// Dispatches keyword → StoreClient. Implemented by the library user.
pub trait StoreRegistry {
    fn client_for(&self, keyword: &str) -> Option<&dyn StoreClient>;
}
