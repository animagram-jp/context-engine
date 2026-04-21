// required.rs: modules required to implement

use core::primitive::usize;
use alloc::collections::BTreeMap;
use crate::provided::Tree;

#[derive(Debug)]
pub enum SetOutcome {
    Created(usize),
    Updated,
}

/// A store provides addressed access to values.
pub trait Store {
    fn get(
        &self,
        key: &[u8],
        args: &BTreeMap<&str, Tree>,
    ) -> Option<Tree>;

    fn set(
        &self,
        key: &[u8],
        args: &BTreeMap<&str, Tree>,
    ) -> Option<SetOutcome>;

    fn delete(
        &self,
        key: &[u8],
        args: &BTreeMap<&str, Tree>,
    ) -> bool;
}

/// Maps compile-time store_id (u8) to a `Store` implementation.
/// The id corresponds to the index position in the `store_ids` slice passed to `Dsl::compile`.
pub trait Stores {
    fn store_for(&self, id: u8) -> Option<&dyn Store>;
}
