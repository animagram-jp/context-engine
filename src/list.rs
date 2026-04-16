use core::{
    primitive::{
        usize,
        bool
    },
    result::Result
};
use alloc::vec::Vec;
use alloc::vec;
use crate::required::{
    Store,
    SetOutcome
};

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

fn is_null<T: Default + PartialEq>(unit: &[T]) -> bool {
    unit.iter().all(|x| *x == T::default())
}

/// A list provides fixed-width unit store.
///
/// Index:  usize - 1-based integer (0 is the null sentinel)
/// Schema: usize - unit size in T
/// Error:  ListError
/// Value:  [T]
///
/// ```
/// use context_engine::list::List;
/// use context_engine::required::{Store, SetOutcome};
///
/// let mut list: List<u32> = List::new(2);
///
/// // append: first real entry is idx=1
/// let r = list.set(&(), &0, &mut 2, &[10u32, 20], false).unwrap();
/// assert!(matches!(r, SetOutcome::Created(1)));
/// assert_eq!(list.get(&(), &1, &2).unwrap(), &[10u32, 20]);
///
/// // update
/// let r = list.set(&(), &1, &mut 2, &[30u32, 40], false).unwrap();
/// assert!(matches!(r, SetOutcome::Updated));
/// assert_eq!(list.get(&(), &1, &2).unwrap(), &[30u32, 40]);
///
/// // delete then reuse_vacant
/// list.delete(&(), &1, &mut 2).unwrap();
/// assert!(list.get(&(), &1, &2).is_err());
/// let r = list.set(&(), &0, &mut 2, &[50u32, 60], true).unwrap();
/// assert!(matches!(r, SetOutcome::Created(1)));
/// ```
pub struct List<T> {
    pub data: Vec<T>,
}

impl<T: Copy + Default + PartialEq> List<T> {
    pub fn new(width: usize) -> Self {
        Self {
            data: vec![T::default(); width],
        }
    }
}

impl<T: Copy + Default + PartialEq> Store<(), usize, usize, ListError, [T]> for List<T> {

    fn get<'a>(
        &'a self,
        _identity: &(),
        index: &usize,
        schema: &usize,
    ) -> Result<&'a [T], ListError> {
        let start = index * schema;
        let end = start + schema;
        let unit = self.data.get(start..end).ok_or(ListError::OutOfBounds)?;
        if is_null(unit) {
            return Err(ListError::NotExist);
        }
        Ok(unit)
    }

    /// intern: if true and index=0, return first match value index(i)
    fn set(
        &mut self,
        _identity: &(),
        index: &usize,
        schema: &mut usize,
        value: &[T],
        reuse_vacant: bool,
    ) -> Result<SetOutcome, ListError> {
        if value.len() != *schema {
            return Err(ListError::OutOfBounds);
        }
        let unit = *schema;
        if *index != 0 {
            let start = index * unit;
            let end = start + unit;
            if end > self.data.len() {
                return Err(ListError::OutOfBounds);
            }
            if is_null(&self.data[start..end]) {
                return Err(ListError::NotExist);
            }
            self.data[start..end].copy_from_slice(value);
            Ok(SetOutcome::Updated)
        } else {
            // Ensure idx=0 sentinel exists
            if self.data.is_empty() {
                self.data.extend(core::iter::repeat(T::default()).take(unit));
            }
            let vacant = if reuse_vacant {
                (1..self.data.len() / unit)
                    .find(|&i| is_null(&self.data[i * unit..(i + 1) * unit]))
            } else {
                None
            };
            match vacant {
                Some(i) => {
                    self.data[i * unit..(i + 1) * unit].copy_from_slice(value);
                    Ok(SetOutcome::Created(i))
                }
                None => {
                    let i = self.data.len() / unit;
                    self.data.extend_from_slice(value);
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
    ) -> Result<(), ListError> {
        let unit = *schema;
        let start = index * unit;
        let end = start + unit;
        if end > self.data.len() {
            return Err(ListError::OutOfBounds);
        }
        self.data[start..end].fill(T::default());
        Ok(())
    }
}

/// A variable list provides variable-length unit store.
///
/// Index:  usize - 1-based integer (0 is the null sentinel). 0 on set appends
/// Error:  ListError
/// Value:  [T]
///
/// ```
/// use context_engine::list::VariableList;
/// use context_engine::required::{Store, SetOutcome};
///
/// let mut vl: VariableList<u32> = VariableList::new();
///
/// // append: first real entry is idx=1
/// let r = vl.set(&(), &0, &mut (), &[1u32, 2, 3], false).unwrap();
/// assert!(matches!(r, SetOutcome::Created(1)));
/// assert_eq!(vl.get(&(), &1, &()).unwrap(), &[1u32, 2, 3]);
///
/// // intern: same value returns existing idx
/// let r = vl.set(&(), &0, &mut (), &[1u32, 2, 3], true).unwrap();
/// assert!(matches!(r, SetOutcome::Created(1)));
///
/// // update in-place (value fits)
/// let r = vl.set(&(), &1, &mut (), &[9u32, 8], false).unwrap();
/// assert!(matches!(r, SetOutcome::Updated));
/// assert_eq!(vl.get(&(), &1, &()).unwrap(), &[9u32, 8]);
///
/// // delete
/// vl.delete(&(), &1, &mut ()).unwrap();
/// assert!(vl.get(&(), &1, &()).is_err());
/// ```
pub struct VariableList<T> {
    pub index: Vec<usize>,
    pub data: Vec<T>,
}

impl<T: Copy + Default + PartialEq> VariableList<T> {
    pub fn new() -> Self {
        Self {
            index: vec![0, 0], // idx=0 sentinel
            data: Vec::new(),
        }
    }
}

impl<T: Copy + Default + PartialEq> Store<(), usize, (), ListError, [T]> for VariableList<T> {

    fn get<'a>(
        &'a self,
        _identity: &(),
        index: &usize,
        _schema: &(),
    ) -> Result<&'a [T], ListError> {
        let index_start = index * 2;
        let index_end = index_start + 2;
        let index_range = self.index.get(index_start..index_end).ok_or(ListError::OutOfBounds)?;
        if is_null(index_range) {
            return Err(ListError::NotExist);
        }
        let start = index_range[0];
        let end = index_range[1];
        self.data.get(start..end).ok_or(ListError::OutOfBounds)
    }

    /// intern: if true and index=0, return first match value index(i)
    ///
    /// note: update tries in-place if value fits the existing range; otherwise
    ///       appends to data and rewrites the index range (old bytes become unreachable
    ///       until compact is called).
    fn set(
        &mut self,
        _identity: &(),
        index: &usize,
        _schema: &mut (),
        value: &[T],
        intern: bool,
    ) -> Result<SetOutcome, ListError> {
        if *index != 0 {
            let index_start = index * 2;
            let index_end = index_start + 2;
            if index_end > self.index.len() {
                return Err(ListError::OutOfBounds);
            }
            if is_null(&self.index[index_start..index_end]) {
                return Err(ListError::NotExist);
            }
            let old_start = self.index[index_start];
            let old_end   = self.index[index_start + 1];
            let old_len   = old_end - old_start;
            if value.len() <= old_len {
                // in-place: value fits within the existing range
                self.data[old_start..old_start + value.len()].copy_from_slice(value);
                self.index[index_start + 1] = old_start + value.len();
            } else {
                // append: value does not fit; old bytes are unreachable until compact
                let start = self.data.len();
                let end = start + value.len();
                self.data.extend_from_slice(value);
                self.index[index_start..index_end].copy_from_slice(&[start, end]);
            }
            Ok(SetOutcome::Updated)
        } else {
            if intern {
                let count = self.index.len() / 2;
                for i in 1..count {
                    let index_start = i * 2;
                    let start = self.index[index_start];
                    let end = self.index[index_start + 1];
                    if !is_null(&self.index[index_start..index_start + 2]) && &self.data[start..end] == value {
                        return Ok(SetOutcome::Created(i));
                    }
                }
            }
            let start = self.data.len();
            let end = start + value.len();
            self.data.extend_from_slice(value);
            // append [start, end] entry to index line via List<usize>
            let entry = [start, end];
            let mut ls: List<usize> = List {
                data: core::mem::take(&mut self.index),
            };
            let outcome = ls.set(&(), &0, &mut 2usize, &entry, false)
                .map_err(|_| ListError::OutOfBounds)?;
            self.index = ls.data;
            Ok(outcome)
        }
    }

    fn delete(
        &mut self,
        _identity: &(),
        index: &usize,
        _schema: &mut (),
    ) -> Result<(), ListError> {
        if *index == 0 {
            return Err(ListError::NotExist);
        }
        let index_start = index * 2;
        let index_end = index_start + 2;
        if index_end > self.index.len() {
            return Err(ListError::OutOfBounds);
        }
        self.index[index_start..index_end].fill(0);
        Ok(())
    }
}

impl<T: Copy + Default + PartialEq> VariableList<T> {
    /// Rebuilds both index and data from scratch:
    /// - vacant entries are removed from index (index shrinks)
    /// - update-leaked bytes in data are reclaimed
    /// - surviving entries are re-assigned sequential idx values starting at 1
    /// Returns a mapping of old idx -> new idx for callers that hold external references.
    ///
    /// ```
    /// use context_engine::list::VariableList;
    /// use context_engine::required::{Store, SetOutcome};
    ///
    /// let mut vl: VariableList<u32> = VariableList::new();
    /// vl.set(&(), &0, &mut (), &[1u32, 2, 3], false).unwrap(); // idx=1
    /// vl.set(&(), &0, &mut (), &[4u32, 5, 6], false).unwrap(); // idx=2
    /// vl.delete(&(), &1, &mut ()).unwrap();                     // idx=1 vacant
    ///
    /// let remap = vl.compact().unwrap();
    /// assert_eq!(remap[&2], 1); // old idx=2 -> new idx=1
    /// assert_eq!(vl.get(&(), &1, &()).unwrap(), &[4u32, 5, 6]);
    /// ```
    pub fn compact(&mut self) -> Result<alloc::collections::BTreeMap<usize, usize>, VariableListError> {
        let mut new_index    = vec![0, 0]; // idx=0 sentinel
        let mut new_data: Vec<T> = Vec::new();
        let mut remap        = alloc::collections::BTreeMap::new();
        let count = self.index.len() / 2;
        // skip i=0 (sentinel)
        for i in 1..count {
            let index_start = i * 2;
            if is_null(&self.index[index_start..index_start + 2]) {
                continue;
            }
            let start = self.index[index_start];
            let end   = self.index[index_start + 1];
            let slice = self.data.get(start..end).ok_or(VariableListError::Compact)?;
            let new_start = new_data.len();
            new_data.extend_from_slice(slice);
            let new_end = new_data.len();
            let new_idx = new_index.len() / 2;
            new_index.push(new_start);
            new_index.push(new_end);
            remap.insert(i, new_idx);
        }
        self.index = new_index;
        self.data  = new_data;
        Ok(remap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::required::{Store, SetOutcome};

    // ── List ─────────────────────────────────────────────────────────────────

    #[test]
    fn list_set_update_append_when_value_too_large() {
        // VariableList::set update path: value exceeds existing range → append
        let mut vl: VariableList<u32> = VariableList::new();
        vl.set(&(), &0, &mut (), &[1u32, 2], false).unwrap(); // idx=1, len=2
        // update with larger value: must append instead of in-place
        let r = vl.set(&(), &1, &mut (), &[10u32, 20, 30], false).unwrap();
        assert!(matches!(r, SetOutcome::Updated));
        assert_eq!(vl.get(&(), &1, &()).unwrap(), &[10u32, 20, 30]);
    }

    #[test]
    fn list_set_wrong_width_returns_out_of_bounds() {
        let mut list: List<u32> = List::new(2);
        let err = list.set(&(), &0, &mut 2, &[1u32], false).unwrap_err();
        assert!(matches!(err, ListError::OutOfBounds));
    }

    #[test]
    fn list_set_update_not_exist() {
        let mut list: List<u32> = List::new(2);
        // append idx=1 then delete it, leaving a null unit
        list.set(&(), &0, &mut 2, &[1u32, 2], false).unwrap();
        list.delete(&(), &1, &mut 2).unwrap();
        let err = list.set(&(), &1, &mut 2, &[3u32, 4], false).unwrap_err();
        assert!(matches!(err, ListError::NotExist));
    }

    #[test]
    fn list_set_update_out_of_bounds() {
        let mut list: List<u32> = List::new(2);
        let err = list.set(&(), &99, &mut 2, &[1u32, 2], false).unwrap_err();
        assert!(matches!(err, ListError::OutOfBounds));
    }

    #[test]
    fn list_get_sentinel_returns_not_exist() {
        let list: List<u32> = List::new(2);
        let err = list.get(&(), &0, &2).unwrap_err();
        assert!(matches!(err, ListError::NotExist));
    }

    // ── VariableList ─────────────────────────────────────────────────────────

    #[test]
    fn variable_list_delete_sentinel_returns_not_exist() {
        let mut vl: VariableList<u32> = VariableList::new();
        let err = vl.delete(&(), &0, &mut ()).unwrap_err();
        assert!(matches!(err, ListError::NotExist));
    }

    #[test]
    fn variable_list_compact_invalidates_old_index() {
        let mut vl: VariableList<u32> = VariableList::new();
        vl.set(&(), &0, &mut (), &[1u32, 2, 3], false).unwrap(); // idx=1
        vl.set(&(), &0, &mut (), &[4u32, 5, 6], false).unwrap(); // idx=2
        vl.delete(&(), &1, &mut ()).unwrap();
        vl.compact().unwrap();
        // old idx=2 is now idx=1; old idx=1 no longer exists
        assert!(vl.get(&(), &2, &()).is_err());
        assert_eq!(vl.get(&(), &1, &()).unwrap(), &[4u32, 5, 6]);
    }
}
