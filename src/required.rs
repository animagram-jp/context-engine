// required.rs: modules required to implement

use core::primitive::{
    usize,
    str
};
use alloc::collections::BTreeMap;
use crate::provided::Tree;

/// Outcome of a `Store::set` call.
#[derive(Debug)]
pub enum SetOutcome {
    Created(usize),
    Updated,
}

/// Dispatches keyword → Store.
pub trait StoreRegistry {
    fn client_for(&self, keyword: &str) -> Option<&dyn Store>;
}

/// A store provides addressed access to values.
pub trait Store<
    Identity,      // declares what the caller is addressing within the store
    Index,         // resolves which element within the addressed set
    Schema,        // defines how indices are interpreted
    Delegate,      // store delegated to: memory reference or TCP endpoint
    Error,
    Value: ?Sized, // the value type stored
> {
    fn get<'a>(
        &self,
        identity: &Identity,
        index: &Index,
        schema: &Schema,
        delegate: &'a Delegate,
    ) -> Result<&'a Value, Error>;

    /// intern: if true, returns existing index for matching content instead of allocating a new one
    fn set(
        &mut self,
        identity: &Identity,
        index: &Index,
        schema: &mut Schema,
        delegate: &mut Delegate,
        value: &Value,
        intern: bool,
    ) -> Result<SetOutcome, Error>;

    fn delete(
        &mut self,
        identity: &Identity,
        index: &Index,
        schema: &mut Schema,
        delegate: &mut Delegate,
    ) -> Result<(), Error>;
}
