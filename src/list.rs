// binary.list — a bit line whose meaning is given by a schema known to the caller.
//
// binary.bound = (origin, extent)
//   origin: position in the list; a plain u64, no structure.
//           compile-time: written into the list; caller reads it.
//           runtime:      caller computes it from the schema.
//   extent: width of the element; derived by cal_bound.
//           fixed:    constant; determined at compile time.
//           variable: read from a separate index list at runtime.

/// boundary: (origin, extent). Derived by cal_bound; consumed by get/set/delete.
pub type Boundary = (u64, u64);

// ── StoreClient ───────────────────────────────────────────────────────────────

pub trait StoreError {}

/// S: schema    — structure of the list; invariant.
/// D: directive — caller-supplied instruction; usecase-dependent.
/// V: value     — element type for get/set.
pub trait StoreClient<S, D, V> {
    type Error: StoreError;

    fn get(&mut self, schema: &S, directive: &D) -> Result<Option<Vec<V>>, Self::Error>;

    fn set(&mut self, schema: &S, directive: &D, value: &[V]) -> Result<Option<SetOutcome>, Self::Error>;

    fn delete(&mut self, schema: &S, directive: &D) -> Result<bool, Self::Error>;
}

pub enum SetOutcome { Created, Updated }

// ── ListClient ────────────────────────────────────────────────────────────────
// Holds the bit line at instantiation. Implements StoreClient<ListSchema, ListDirective<T>, T>.
//
// Usecases and required arguments:
//   fixed-length:    schema { base, extent }                directive { index }
//   variable-length: schema { base }                        directive { index, list }
//   interning:       schema { base, extent, interning }     directive { index }
//   embedded_schema: schema { base, extent_offset }         directive { index }

pub struct ListSchema {
    pub base:          Option<u64>,
    pub extent:        Option<u64>,
    pub extent_offset: Option<u64>,
    pub interning:     Option<bool>,
}

pub struct ListDirective<T> {
    pub origin: Option<u64>,
    pub index:  Option<u64>,
    pub list:   Option<Vec<T>>,
}

pub enum ListError {
    SchemaMissing,
    OutOfBounds,
}

impl StoreError for ListError {}

pub struct ListClient<T> {
    line: Vec<T>,
}

impl<T> ListClient<T> {
    pub fn new(line: Vec<T>) -> Self {
        Self { line }
    }
}

// Derives boundary from schema + directive.
// list=None: fixed-length; extent is constant.
// list=Some: variable-length; extent is read from the list at current origin.
// Stops at index=0. origin=None on first call starts from base.
fn cal_bound<T: Into<u64> + Clone>(origin: Option<u64>, base: u64, index: u64, extent: u64, list: Option<&Vec<T>>) -> Boundary {
    let current = origin.unwrap_or(base);
    let current_extent = match list {
        None       => extent,
        Some(list) => list[current as usize].clone().into(),
    };
    if index == 0 { return (current, current_extent); }
    cal_bound(Some(current + current_extent), base, index - 1, extent, list)
}

impl<T: Clone + PartialEq + Default + Into<u64>> StoreClient<ListSchema, ListDirective<T>, T> for ListClient<T> {
    type Error = ListError;

    fn get(&mut self, schema: &ListSchema, directive: &ListDirective<T>) -> Result<Option<Vec<T>>, ListError> {
        let base   = schema.base.ok_or(ListError::SchemaMissing)?;
        let extent = schema.extent.ok_or(ListError::SchemaMissing)?;
        let index  = directive.index.ok_or(ListError::SchemaMissing)?;
        let (origin, extent) = cal_bound(directive.origin, base, index, extent, directive.list.as_ref());
        if origin + extent > self.line.len() as u64 { return Err(ListError::OutOfBounds); }
        Ok(Some(self.line[origin as usize .. (origin + extent) as usize].to_vec()))
    }

    fn set(&mut self, schema: &ListSchema, directive: &ListDirective<T>, value: &[T]) -> Result<Option<SetOutcome>, ListError> {
        let base   = schema.base.ok_or(ListError::SchemaMissing)?;
        let extent = schema.extent.ok_or(ListError::SchemaMissing)?;
        let index  = directive.index.ok_or(ListError::SchemaMissing)?;
        let bound @ (origin, extent) = cal_bound(directive.origin, base, index, extent, directive.list.as_ref());
        if origin + extent > self.line.len() as u64 { return Err(ListError::OutOfBounds); }
        if schema.interning.unwrap_or(false) {
            // interning: dedup; fold on first hit. multiple hits = list corruption (out of scope).
            let mut pos = 0u64;
            while pos + extent <= self.line.len() as u64 {
                if &self.line[pos as usize .. (pos + extent) as usize] == value {
                    return Ok(Some(SetOutcome::Updated));
                }
                pos += extent;
            }
            self.line.extend_from_slice(value);
            return Ok(Some(SetOutcome::Created));
        }
        let outcome = if &self.line[origin as usize .. (origin + extent) as usize] == value {
            SetOutcome::Updated
        } else {
            SetOutcome::Created
        };
        self.line[origin as usize .. (origin + extent) as usize].clone_from_slice(value);
        Ok(Some(outcome))
    }

    fn delete(&mut self, schema: &ListSchema, directive: &ListDirective<T>) -> Result<bool, ListError> {
        let base   = schema.base.ok_or(ListError::SchemaMissing)?;
        let extent = schema.extent.ok_or(ListError::SchemaMissing)?;
        let index  = directive.index.ok_or(ListError::SchemaMissing)?;
        let (origin, extent) = cal_bound(directive.origin, base, index, extent, directive.list.as_ref());
        if origin + extent > self.line.len() as u64 { return Err(ListError::OutOfBounds); }
        self.line[origin as usize .. (origin + extent) as usize].fill(T::default());
        Ok(true)
    }
}
