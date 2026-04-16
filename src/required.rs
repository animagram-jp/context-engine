// required.rs: modules required to implement

use core::primitive::usize;

/// Outcome of a `Store::set` call.
#[derive(Debug)]
pub enum SetOutcome {
    Created(usize),
    Updated,
}

/// A store provides addressed access to values.
pub trait Store<
    Identity,      // declares what the caller is addressing within the store
    Index,         // resolves which element within the addressed set
    Schema,        // defines how indices are interpreted
    // Delegate,      // store delegated to: memory reference or TCP endpoint
    Error,
    Value: ?Sized, // the value type stored
> {
    fn get<'a>(
        &'a self,
        identity: &Identity,
        index: &Index,
        schema: &Schema,
    ) -> Result<&'a Value, Error>;

    /// intern: if true, returns existing index for matching content instead of allocating a new one
    fn set(
        &mut self,
        identity: &Identity,
        index: &Index,
        schema: &mut Schema,
        value: &Value,
        intern: bool,
    ) -> Result<SetOutcome, Error>;

    fn delete(
        &mut self,
        identity: &Identity,
        index: &Index,
        schema: &mut Schema,
    ) -> Result<(), Error>;
}
