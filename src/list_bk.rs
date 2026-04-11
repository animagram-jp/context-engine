use core::clone::Clone;
use core::cmp::PartialEq;
use core::default::Default;
use alloc::vec::Vec;

// binary.list — a bit line whose meaning is given by a schema known to the caller.
//
// binary.bound = (origin, extent)
//   origin: position in the list; a plain u64, no structure.
//   extent: width of the element.

/// boundary: (origin, extent).
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

// ── indexed_boundary ──────────────────────────────────────────────────────────

pub fn indexed_boundary(base: u64, extent: u64, index: u64) -> Boundary {
    (base + index * extent, extent)
}

// ── ListClient ────────────────────────────────────────────────────────────────
// Single list. Operations take a pre-resolved boundary.

pub enum ListError {
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

impl<T: Clone + PartialEq + Default> ListClient<T> {
    pub fn get(&self, boundary: &Boundary) -> Result<Option<Vec<T>>, ListError> {
        let (origin, extent) = *boundary;
        if origin + extent > self.line.len() as u64 { return Err(ListError::OutOfBounds); }
        Ok(Some(self.line[origin as usize .. (origin + extent) as usize].to_vec()))
    }

    pub fn set(&mut self, boundary: &Boundary, value: &[T]) -> Result<Option<SetOutcome>, ListError> {
        let (origin, extent) = *boundary;
        if origin + extent > self.line.len() as u64 { return Err(ListError::OutOfBounds); }
        let outcome = if &self.line[origin as usize .. (origin + extent) as usize] == value {
            SetOutcome::Updated
        } else {
            SetOutcome::Created
        };
        self.line[origin as usize .. (origin + extent) as usize].clone_from_slice(value);
        Ok(Some(outcome))
    }

    pub fn delete(&mut self, boundary: &Boundary) -> Result<bool, ListError> {
        let (origin, extent) = *boundary;
        if origin + extent > self.line.len() as u64 { return Err(ListError::OutOfBounds); }
        self.line[origin as usize .. (origin + extent) as usize].fill(T::default());
        Ok(true)
    }
}

// ── ListsClient ───────────────────────────────────────────────────────────────
// Multiple lists. Resolves boundary from schema + directive via cal_bound.
//
// Usecases and required arguments:
//   fixed-length:    schema { base, extent }             directive { index }
//   variable-length: schema { base }                     directive { index, lists }
//   interning:       schema { base, extent, interning }  directive { index }
//   embedded_schema: schema { base, extent_offset }      directive { index }

pub struct ListsSchema {
    pub base:          Option<u64>,
    pub extent:        Option<u64>,
    pub extent_offset: Option<u64>,
    pub interning:     Option<bool>,
}

pub struct ListsDirective<T> {
    pub origin: Option<u64>,
    pub index:  Option<u64>,
    pub lists:  Vec<Vec<T>>,
}

pub enum ListsError {
    SchemaMissing,
    OutOfBounds,
}

impl StoreError for ListsError {}

pub struct ListsClient<T> {
    line: Vec<T>,
}

impl<T> ListsClient<T> {
    pub fn new(line: Vec<T>) -> Self {
        Self { line }
    }
}

// Derives boundary from schema + directive.
// lists empty:     fixed-length; extent is constant.
// lists non-empty: variable-length; consume one list per recursion level.
// Stops at index=0. origin=None on first call starts from base.
fn cal_bound<T: core::convert::Into<u64> + Clone>(
    origin: Option<u64>,
    base: u64,
    index: u64,
    extent: u64,
    lists: &[Vec<T>],
) -> Boundary {
    let current = origin.unwrap_or(base);
    let current_extent = match lists.first() {
        None       => extent,
        Some(list) => list[current as usize].clone().into(),
    };
    if index == 0 { return (current, current_extent); }
    cal_bound(Some(current + current_extent), base, index - 1, extent, &lists[1..])
}

impl<T: Clone + PartialEq + Default + core::convert::Into<u64>> ListsClient<T> {
    pub fn get(&mut self, schema: &ListsSchema, directive: &ListsDirective<T>) -> Result<Option<Vec<T>>, ListsError> {
        let base   = schema.base.ok_or(ListsError::SchemaMissing)?;
        let extent = schema.extent.ok_or(ListsError::SchemaMissing)?;
        let index  = directive.index.ok_or(ListsError::SchemaMissing)?;
        let (origin, extent) = cal_bound(directive.origin, base, index, extent, &directive.lists);
        if origin + extent > self.line.len() as u64 { return Err(ListsError::OutOfBounds); }
        Ok(Some(self.line[origin as usize .. (origin + extent) as usize].to_vec()))
    }

    pub fn set(&mut self, schema: &ListsSchema, directive: &ListsDirective<T>, value: &[T]) -> Result<Option<SetOutcome>, ListsError> {
        let base   = schema.base.ok_or(ListsError::SchemaMissing)?;
        let extent = schema.extent.ok_or(ListsError::SchemaMissing)?;
        let index  = directive.index.ok_or(ListsError::SchemaMissing)?;
        let (origin, extent) = cal_bound(directive.origin, base, index, extent, &directive.lists);
        if origin + extent > self.line.len() as u64 { return Err(ListsError::OutOfBounds); }
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

    pub fn delete(&mut self, schema: &ListsSchema, directive: &ListsDirective<T>) -> Result<bool, ListsError> {
        let base   = schema.base.ok_or(ListsError::SchemaMissing)?;
        let extent = schema.extent.ok_or(ListsError::SchemaMissing)?;
        let index  = directive.index.ok_or(ListsError::SchemaMissing)?;
        let (origin, extent) = cal_bound(directive.origin, base, index, extent, &directive.lists);
        if origin + extent > self.line.len() as u64 { return Err(ListsError::OutOfBounds); }
        self.line[origin as usize .. (origin + extent) as usize].fill(T::default());
        Ok(true)
    }
}
