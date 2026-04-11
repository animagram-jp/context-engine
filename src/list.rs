// --- file global ---

use core::{
    primitive::{
        u8, u16, u32, u64, usize,
        bool,
        char,str
    },
    result::Result,
    fmt::{
        Display,
        Formatter,
        Result
    },
    clone::Clone,
    cmp::PartialEq,
    default::Default
};
use alloc::vec::Vec;

// --- StoreClient ---- 

pub trait StoreError {}
pub enum SetOutcome { 
  Created, 
  Updated
}

/// T: target    - target of operation 
/// S: schema    - structure of the list; invariant.
/// D: directive - caller-supplied instruction; usecase-dependent.
/// V: value     - element type for get/set.
pub trait StoreClient<T, S, D, V> {
    type Error: StoreError;
    fn get(&mut self, target: &T, schema: &S, directive: &D) -> Result<Option<Vec<V>>, Self::Error>;
    fn set(&mut self, target: &T, schema: &S, directive: &D, value: &[usize]) -> Result<Option<SetOutcome>, Self::Error>;
    fn delete(&mut self, target: &T, schema: &S, directive: &D) -> Result<bool, Self::Error>;
}

// --- ListClient ---

pub type Boundary = (u64, u64); // boundary: (origin, extent)

pub enum ListError {
  OutOfBounds
}

/// u64: index
/// S: schema    - structure of the list; invariant.
/// D: directive - caller-supplied instruction; usecase-dependent.
/// V: value     - [u8]/[u16]/[u32]/[u64]
pub IndexClient<u64, S, D, V> {
  type Error: ListError;
  fn get(&mut self, index: &u64, base: &u64, extent: &u64) -> Result<Option<V>, Self::Error>;
  fn set(&mut self, target: &T, schema: &S, directive: &D, value: &V) -> Result<Option<SetOutcome>, Self::Error>;
  fn delete(&mut self, target: &T, schema: &S, directive: &D) -> Result<bool, Self::Error>;
}

pub fn indexed_boundary(base: u64, extent: u64, index: u64) -> Boundary {
  (base + index * extent, extent)
}