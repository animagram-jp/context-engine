// required.rs: modules required to implement

use core::primitive::usize;

#[derive(Debug)]
pub enum SetOutcome {
    Created(usize),
    Updated,
}

/// A store provides addressed access to values.
pub trait Store<
    Error,
    Value: ?Sized,
> {
    fn get<'a>(
        &'a self,
        key: &[u8],
        map: &Vec<[u8]>,
        args: &Tree,
    ) -> Result<&'a Value, Error>;

    /// intern: if true, returns existing index for matching content instead of allocating a new one
    fn set(
        &mut self,
        key: &[u8],
        map: &Vec<[u8]>,
        args: &Tree,
        value: &Value,
        intern: bool,
    ) -> Result<SetOutcome, Error>;

    fn delete(
        &mut self,
        key: &[u8],
        map: &Vec<[u8]>,
        args: &Tree,
    ) -> Result<(), Error>;
}
