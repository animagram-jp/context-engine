use core::{
    primitive::{
        usize,
        bool
    },
    result::Result
};
use alloc::vec::Vec;
use crate::required::{
    Store, 
    SetOutcome
}

#[derive(Debug)]
pub enum ListError {
    OutOfBounds,
    NotExist,
}
#[derive(Debug)]
pub enum VariableListError {
    List(ListError),
    Compact,
}

fn is_null(slot: &[usize]) -> bool {
    slot.iter().all(|&x| x == 0)
}

/// A list provides usize width store.
///
/// Index:    usize      - 1-based integer (0 is the null sentinel)
/// Schema:   usize      - width unit
/// Delegate: Vec<usize> - flat data line
/// Error:    ListError
/// Value:    [usize]
pub struct List;

impl Store<(), usize, usize, Vec<usize>, ListError, [usize]> for List {

    fn get<'a>(
        &self,
        _identity: &(),
        index: &usize,
        schema: &usize,
        delegate: &'a Vec<usize>,
    ) -> Result<&'a [usize], ListError> {
        let start = index * schema;
        let end = start + schema;
        let slot = delegate.get(start..end).ok_or(ListError::OutOfBounds)?;
        if is_null(slot) {
            return Err(ListError::NotExist);
        }
        Ok(slot)
    }

    /// intern: if true and index=0, return first match value index(i)
    fn set(
        &mut self,
        _identity: &(),
        index: &usize,
        schema: &mut usize,
        delegate: &mut Vec<usize>,
        value: &[usize],
        reuse_vacant: bool,
    ) -> Result<SetOutcome, ListError> {
        if value.len() != *schema {
            return Err(ListError::OutOfBounds);
        }
        let unit = *schema;
        if *index != 0 {
            let start = index * unit;
            let end = start + unit;
            if end > delegate.len() {
                return Err(ListError::OutOfBounds);
            }
            if is_null(&delegate[start..end]) {
                return Err(ListError::NotExist);
            }
            delegate[start..end].copy_from_slice(value);
            Ok(SetOutcome::Updated)
        } else {
            // Ensure idx=0 sentinel slot exists
            if delegate.is_empty() {
                delegate.extend(core::iter::repeat(0).take(unit));
            }
            let vacant = if reuse_vacant {
                (1..delegate.len() / unit)
                    .find(|&i| is_null(&delegate[i * unit..(i + 1) * unit]))
            } else {
                None
            };
            match vacant {
                Some(i) => {
                    delegate[i * unit..(i + 1) * unit].copy_from_slice(value);
                    Ok(SetOutcome::Created(i))
                }
                None => {
                    let i = delegate.len() / unit;
                    delegate.extend_from_slice(value);
                    Ok(SetOutcome::Created(i))
                }
            }
        }
    }

    fn delete(
        &mut self,
        _identity: &(),
        index: &usize,
        schema: &mut usize,
        delegate: &mut Vec<usize>,
    ) -> Result<(), ListError> {
        let unit = *schema;
        let start = index * unit;
        let end = start + unit;
        if end > delegate.len() {
            return Err(ListError::OutOfBounds);
        }
        delegate[start..end].fill(0);
        Ok(())
    }
}

/// A variable list provides variable size store.
///
/// Index:    usize      - 1-based integer (0 is the null sentinel). 0 on set appends
/// Schema:   Vec<usize> - the range index line
/// Delegate: [usize]    - flat data line
/// Schema:   Vec<usize> — the range index line
/// Delegate: [usize]    — the data line
pub struct VariableList;

impl Store<(), usize, Vec<usize>, Vec<usize>, ListError, [usize]> for VariableList {

    /// example:
    /// ```
    /// use crate::list::VariableList;
    /// use crate::required::Store;
    /// let mut schema   = vec![0, 0, 0, 3, 3, 6];
    /// let delegate = vec![1, 2, 3, 4, 5, 6];
    /// let s = VariableList;
    /// assert_eq!(s.get(&(), &1, &schema, &delegate).unwrap(), &[1, 2, 3]);
    /// assert_eq!(s.get(&(), &2, &schema, &delegate).unwrap(), &[4, 5, 6]);
    /// ```
    fn get<'a>(
        &self,
        _identity: &(),
        index: &usize,
        schema: &Vec<usize>,
        delegate: &'a Vec<usize>,
    ) -> Result<&'a [usize], ListError> {
        let idx_start = index * 2;
        let idx_end = idx_start + 2;
        let idx_slot = schema.get(idx_start..idx_end).ok_or(ListError::OutOfBounds)?;
        if is_null(idx_slot) {
            return Err(ListError::NotExist);
        }
        let start = idx_slot[0];
        let end = idx_slot[1];
        delegate.get(start..end).ok_or(ListError::OutOfBounds)
    }

    /// intern: if true and index=0, return first match value index(i)
    ///
    /// note: update tries in-place if value fits the existing slot; otherwise
    ///       appends to delegate and rewrites the schema range (old bytes become unreachable
    ///       until compact is called).
    ///
    /// example:
    /// ```
    /// use crate::list::VariableList;
    /// use crate::required::{Store, SetOutcome};
    /// let mut schema   = vec![];
    /// let mut delegate = vec![];
    /// let mut s = VariableList;
    ///
    /// // append: first real entry is idx=1 (idx=0 is the null sentinel)
    /// let r = s.set(&(), &0, &mut schema, &mut delegate, &[1, 2, 3], false).unwrap();
    /// assert!(matches!(r, SetOutcome::Created(1)));
    /// assert_eq!(s.get(&(), &1, &schema, &delegate).unwrap(), &[1, 2, 3]);
    ///
    /// // update in-place (same length)
    /// let r = s.set(&(), &1, &mut schema, &mut delegate, &[7, 8, 9], false).unwrap();
    /// assert!(matches!(r, SetOutcome::Updated));
    /// assert_eq!(s.get(&(), &1, &schema, &delegate).unwrap(), &[7, 8, 9]);
    ///
    /// // intern: same value returns existing idx
    /// let r = s.set(&(), &0, &mut schema, &mut delegate, &[7, 8, 9], true).unwrap();
    /// assert!(matches!(r, SetOutcome::Created(1)));
    /// ```
    fn set(
        &mut self,
        _identity: &(),
        index: &usize,
        schema: &mut Vec<usize>,
        delegate: &mut Vec<usize>,
        value: &[usize],
        intern: bool,
    ) -> Result<SetOutcome, ListError> {
        if *index != 0 {
            if *index == 0 {
                return Err(ListError::NotExist);
            }
            let idx_start = index * 2;
            let idx_end = idx_start + 2;
            if idx_end > schema.len() {
                return Err(ListError::OutOfBounds);
            }
            if is_null(&schema[idx_start..idx_end]) {
                return Err(ListError::NotExist);
            }
            let old_start = schema[idx_start];
            let old_end   = schema[idx_start + 1];
            let old_len   = old_end - old_start;
            if value.len() <= old_len {
                // in-place: value fits within the existing slot
                delegate[old_start..old_start + value.len()].copy_from_slice(value);
                schema[idx_start + 1] = old_start + value.len();
            } else {
                // append: value does not fit; old bytes are unreachable until compact
                let start = delegate.len();
                let end = start + value.len();
                delegate.extend_from_slice(value);
                schema[idx_start..idx_end].copy_from_slice(&[start, end]);
            }
            Ok(SetOutcome::Updated)
        } else {
            if intern {
                let count = schema.len() / 2;
                for i in 1..count {
                    let idx_start = i * 2;
                    let start = schema[idx_start];
                    let end = schema[idx_start + 1];
                    if !is_null(&schema[idx_start..idx_start + 2]) && &delegate[start..end] == value {
                        return Ok(SetOutcome::Created(i));
                    }
                }
            }
            let start = delegate.len();
            let end = start + value.len();
            delegate.extend_from_slice(value);
            // use list::set to append a [start, end] entry to schema
            let entry = [start, end];
            use super::list::List;
            let mut ls = List;
            let i = ls.set(&(), &0, &mut 2usize, schema, &entry, false)?;
            Ok(i)
        }
    }

    /// example:
    /// ```
    /// use context_engine::list::variable_list::VariableList;
    /// use context_engine::list::{Store, ListError};
    /// // idx=0 sentinel, idx=1 -> [1,2,3], idx=2 -> [4,5,6]
    /// let mut schema   = vec![0, 0, 0, 3, 3, 6];
    /// let mut delegate = vec![1, 2, 3, 4, 5, 6];
    /// let mut s = VariableList;
    /// s.delete(&(), &1, &mut schema, &mut delegate).unwrap();
    /// assert!(matches!(s.get(&(), &1, &schema, &delegate), Err(ListError::NotExist)));
    /// ```
    fn delete(
        &mut self,
        _identity: &(),
        index: &usize,
        schema: &mut Vec<usize>,
        _delegate: &mut Vec<usize>,
    ) -> Result<(), ListError> {
        if *index == 0 {
            return Err(ListError::NotExist);
        }
        let idx_start = index * 2;
        let idx_end = idx_start + 2;
        if idx_end > schema.len() {
            return Err(ListError::OutOfBounds);
        }
        schema[idx_start..idx_end].fill(0);
        Ok(())
    }
}

impl VariableList {
    /// Rebuilds both schema and delegate from scratch:
    /// - vacant slots are removed from schema (schema shrinks)
    /// - update-leaked bytes in delegate are reclaimed
    /// - surviving entries are re-assigned sequential idx values starting at 1
    /// Returns a mapping of old idx -> new idx for callers that hold external references.
    ///
    /// example:
    /// ```
    /// use context_engine::list::variable_list::VariableList;
    /// // idx=0 sentinel, idx=1 -> [1,2,3], idx=2 is vacant, idx=3 -> [4,5,6]
    /// let mut schema   = vec![0, 0, 0, 3, 0, 0, 3, 6];
    /// let mut delegate = vec![1, 2, 3, 4, 5, 6];
    /// let remap = VariableList::compact(&mut schema, &mut delegate).unwrap();
    /// // vacant idx=2 removed; survivors re-assigned to idx=1 and idx=2
    /// assert_eq!(remap[&1], 1);
    /// assert_eq!(remap[&3], 2);
    /// ```
    pub fn compact(schema: &mut Vec<usize>, delegate: &mut Vec<usize>) -> Result<alloc::collections::BTreeMap<usize, usize>, VariableListError> {
        let mut new_schema   = vec![0, 0]; // idx=0 sentinel
        let mut new_delegate = Vec::new();
        let mut remap        = alloc::collections::BTreeMap::new();
        let count = schema.len() / 2;
        // skip i=0 (sentinel)
        for i in 1..count {
            let idx_start = i * 2;
            if is_null(&schema[idx_start..idx_start + 2]) {
                continue;
            }
            let start = schema[idx_start];
            let end   = schema[idx_start + 1];
            let slice = delegate.get(start..end).ok_or(VariableListError::Compact)?;
            let new_start = new_delegate.len();
            new_delegate.extend_from_slice(slice);
            let new_end = new_delegate.len();
            let new_idx = new_schema.len() / 2;
            new_schema.push(new_start);
            new_schema.push(new_end);
            remap.insert(i, new_idx);
        }
        *schema   = new_schema;
        *delegate = new_delegate;
        Ok(remap)
    }
}
