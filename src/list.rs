// --- file global ---

use alloc::vec::Vec;
use core::result::Result;

// --- StoreClient ----

pub trait StoreError {}

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

// --- List ---

pub enum ListError {
    OutOfBounds,
    NotExist,
}
pub enum SetOutcome {
    Created(usize),
    Updated,
}

mod list {
    use super::{ListError, SetOutcome};
    use alloc::vec::Vec;

    fn is_vacant(slot: &[usize]) -> bool {
        slot.iter().all(|&x| x == 0)
    }
    pub fn get(list: &[usize], index: usize, unit: usize) -> Result<&[usize], ListError> {
        let start = index * unit;
        let end = start + unit;
        let slot = list.get(start..end).ok_or(ListError::OutOfBounds)?;
        if is_vacant(slot) {
            return Err(ListError::NotExist);
        }
        Ok(slot)
    }
    pub fn set(list: &mut Vec<usize>, index: Option<usize>, unit: usize, value: &[usize], reuse_vacant: bool) -> Result<SetOutcome, ListError> {
        if value.len() != unit {
            return Err(ListError::OutOfBounds);
        }
        match index {
            Some(index) => {
                let start = index * unit;
                let end = start + unit;
                if end > list.len() {
                    return Err(ListError::OutOfBounds);
                }
                list[start..end].copy_from_slice(value);
                Ok(SetOutcome::Updated)
            }
            None => {
                let vacant = if reuse_vacant {
                    (0..list.len() / unit)
                        .find(|&i| is_vacant(&list[i * unit..(i + 1) * unit]))
                } else {
                    None
                };
                match vacant {
                    Some(i) => {
                        list[i * unit..(i + 1) * unit].copy_from_slice(value);
                        Ok(SetOutcome::Created(i))
                    }
                    None => {
                        let i = list.len() / unit;
                        list.extend_from_slice(value);
                        Ok(SetOutcome::Created(i))
                    }
                }
            }
        }
    }

    pub fn delete(list: &mut Vec<usize>, index: usize, unit: usize) -> Result<(), ListError> {
        let start = index * unit;
        let end = start + unit;
        if end > list.len() {
            return Err(ListError::OutOfBounds);
        }
        if is_vacant(&list[start..end]) {
            return Err(ListError::NotExist);
        }
        list[start..end].fill(0);
        Ok(())
    }
}

// --- Variable List  ---

mod variable_list {
    fn is_vacant(slot: &[usize]) -> bool {
        slot.iter().all(|&x| x == 0)
    }

    pub fn get<'a>(index: &[usize], data: &'a [usize], i: usize) -> Result<&'a [usize], ListError> {
        let idx_start = i * 2;
        let idx_end = idx_start + 2;
        let idx_slot = index.get(idx_start..idx_end).ok_or(ListError::OutOfBounds)?;
        if is_vacant(idx_slot) {
            return Err(ListError::NotExist);
        }
        let start = idx_slot[0];
        let end = idx_slot[1];
        data.get(start..end).ok_or(ListError::OutOfBounds)
    }

    pub fn set(index: &mut Vec<usize>, data: &mut Vec<usize>, i: Option<usize>, value: &[usize], intern: bool) -> Result<SetOutcome, ListError> {
        match i {
            Some(i) => {
                let idx_start = i * 2;
                let idx_end = idx_start + 2;
                if idx_end > index.len() {
                    return Err(ListError::OutOfBounds);
                }
                let start = data.len();
                let end = start + value.len();
                data.extend_from_slice(value);
                index[idx_start..idx_end].copy_from_slice(&[start, end]);
                Ok(SetOutcome::Updated)
            }
            None => {
                if intern {
                    let count = index.len() / 2;
                    for i in 0..count {
                        let idx_start = i * 2;
                        let start = index[idx_start];
                        let end = index[idx_start + 1];
                        if !is_vacant(&index[idx_start..idx_start + 2]) && &data[start..end] == value {
                            return Ok(SetOutcome::Created(i));
                        }
                    }
                }
                let start = data.len();
                let end = start + value.len();
                data.extend_from_slice(value);
                let entry = [start, end];
                let i = list::set(index, None, 2, &entry, false)?;
                Ok(i)
            }
        }
    }

    pub fn delete(index: &mut Vec<usize>, i: usize) -> Result<(), ListError> {
        let idx_start = i * 2;
        let idx_end = idx_start + 2;
        if idx_end > index.len() {
            return Err(ListError::OutOfBounds);
        }
        if is_vacant(&index[idx_start..idx_end]) {
            return Err(ListError::NotExist);
        }
        index[idx_start..idx_end].fill(0);
        Ok(())
    }

    pub fn compact(index: &mut Vec<usize>, data: &mut Vec<usize>) {
        let mut new_data = Vec::new();
        let count = index.len() / 2;
        for i in 0..count {
            let idx_start = i * 2;
            let start = index[idx_start];
            let end = index[idx_start + 1];
            if start == 0 && end == 0 {
                continue;
            }
            let new_start = new_data.len();
            new_data.extend_from_slice(&data[start..end]);
            let new_end = new_data.len();
            index[idx_start] = new_start;
            index[idx_start + 1] = new_end;
        }
        *data = new_data;
    }
}
