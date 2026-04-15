// --- file global ---

use alloc::vec::Vec;
use alloc::collections::BTreeMap;

pub trait Store {

    type Identity;  // declares what the caller is addressing within the store
    type Index;     // resolves which element within the addressed set
    type Schema;    // the structure that maps values to indices
    type Delegate;  // store delegated to: memory reference or TCP endpoint
    type Error;
    type Value: ?Sized;  // the element type stored

    fn get(
        &self,
        identity: &Self::Identity,
        index: &Self::Index,
        schema: &Self::Schema,
        delegate: &Self::Delegate,
    ) -> Result<&Self::Value, Self::Error>;

    /// intern: if true, returns existing idx for matching content instead of allocating a new one
    fn set(
        &mut self,
        identity: &Self::Identity,
        index: &Self::Index,
        schema: &Self::Schema,
        delegate: &Self::Delegate,
        value: &Self::Value,
        intern: bool,
    ) -> Result<SetOutcome, Self::Error>;

    fn delete(
        &mut self,
        identity: &Self::Identity,
        index: &Self::Index,
        schema: &Self::Schema,
        delegate: &Self::Delegate,
    ) -> Result<(), Self::Error>;
}

// --- List ---

#[derive(Debug)]
pub enum ListError {
    OutOfBounds,
    NotExist,
}
#[derive(Debug)]
pub enum SetOutcome {
    Created(usize),
    Updated,
}

pub mod list {
    use alloc::vec::Vec;
    use core::result::Result;
    use super::{ListError, SetOutcome};

    fn is_vacant(slot: &[usize]) -> bool {
        slot.iter().all(|&x| x == 0)
    }

    /// list: line
    /// idx:  index number of target
    /// unit: units of target extent
    pub fn get(list: &[usize], idx: usize, unit: usize) -> Result<&[usize], ListError> {
        let start = idx * unit;
        let end = start + unit;
        let slot = list.get(start..end).ok_or(ListError::OutOfBounds)?;
        if is_vacant(slot) {
            return Err(ListError::NotExist);
        }
        Ok(slot)
    }

    /// list: line
    /// idx:  index number of target (1-based; idx=0 is the null sentinel)
    /// unit: units of target extent
    /// value:
    /// reuse_vacant: write to first match 00...00 slice (skips idx=0 sentinel)
    ///
    /// On first use, call set(None, ...) to initialise: it reserves idx=0 as the
    /// null sentinel and returns Created(1) for the first real entry.
    pub fn set(list: &mut Vec<usize>, idx: Option<usize>, unit: usize, value: &[usize], reuse_vacant: bool) -> Result<SetOutcome, ListError> {
        if value.len() != unit {
            return Err(ListError::OutOfBounds);
        }
        match idx {
            Some(idx) => {
                if idx == 0 {
                    return Err(ListError::NotExist);
                }
                let start = idx * unit;
                let end = start + unit;
                if end > list.len() {
                    return Err(ListError::OutOfBounds);
                }
                if is_vacant(&list[start..end]) {
                    return Err(ListError::NotExist);
                }
                list[start..end].copy_from_slice(value);
                Ok(SetOutcome::Updated)
            }
            None => {
                // Ensure idx=0 sentinel slot exists
                if list.is_empty() {
                    list.extend(core::iter::repeat(0).take(unit));
                }
                let vacant = if reuse_vacant {
                    (1..list.len() / unit)
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

    /// list: line
    /// idx:  index number of target
    /// unit: units of target extent
    pub fn delete(list: &mut Vec<usize>, idx: usize, unit: usize) -> Result<(), ListError> {
        let start = idx * unit;
        let end = start + unit;
        if end > list.len() {
            return Err(ListError::OutOfBounds);
        }
        list[start..end].fill(0);
        Ok(())
    }
}

// --- Variable List  ---

#[derive(Debug)]
pub enum VariableListError {
    List(ListError),
    Compact,
}

pub mod variable_list {
    use alloc::vec::Vec;
    use alloc::vec;
    use core::result::Result;
    use super::{ListError, SetOutcome, VariableListError};
    use super::list;

    fn is_vacant(slot: &[usize]) -> bool {
        slot.iter().all(|&x| x == 0)
    }

    /// index: line that has ranges
    /// data:  line that has values
    /// idx:   index number of target (1-based; idx=0 is the null sentinel)
    ///
    /// example:
    /// ```
    /// use context_engine::list::variable_list;
    /// // idx=0 is the null sentinel (2 zeros); real entries start at idx=1
    /// let index = vec![0, 0, 0, 3, 3, 6];
    /// let data  = vec![1, 2, 3, 4, 5, 6];
    /// assert_eq!(variable_list::get(&index, &data, 1).unwrap(), &[1, 2, 3]);
    /// assert_eq!(variable_list::get(&index, &data, 2).unwrap(), &[4, 5, 6]);
    /// ```
    pub fn get<'a>(index: &[usize], data: &'a [usize], idx: usize) -> Result<&'a [usize], ListError> {
        let idx_start = idx * 2;
        let idx_end = idx_start + 2;
        let idx_slot = index.get(idx_start..idx_end).ok_or(ListError::OutOfBounds)?;
        if is_vacant(idx_slot) {
            return Err(ListError::NotExist);
        }
        let start = idx_slot[0];
        let end = idx_slot[1];
        data.get(start..end).ok_or(ListError::OutOfBounds)
    }

    /// index: line that has ranges
    /// data:  line that has values
    /// idx:   index number of target (1-based; idx=0 is the null sentinel)
    /// value:
    /// intern: when idx: None, search data and return first-match idx if found
    ///
    /// note: update tries in-place if value fits the existing slot; otherwise
    ///       appends to data and rewrites the index range (old bytes become unreachable
    ///       until compact is called).
    ///
    /// example:
    /// ```
    /// use context_engine::list::variable_list;
    /// use context_engine::list::SetOutcome;
    /// let mut index = vec![];
    /// let mut data  = vec![];
    ///
    /// // append: first real entry is idx=1 (idx=0 is the null sentinel)
    /// let r = variable_list::set(&mut index, &mut data, None, &[1, 2, 3], false).unwrap();
    /// assert!(matches!(r, SetOutcome::Created(1)));
    /// assert_eq!(variable_list::get(&index, &data, 1).unwrap(), &[1, 2, 3]);
    ///
    /// // update in-place (same length)
    /// let r = variable_list::set(&mut index, &mut data, Some(1), &[7, 8, 9], false).unwrap();
    /// assert!(matches!(r, SetOutcome::Updated));
    /// assert_eq!(variable_list::get(&index, &data, 1).unwrap(), &[7, 8, 9]);
    ///
    /// // intern: same value returns existing idx
    /// let r = variable_list::set(&mut index, &mut data, None, &[7, 8, 9], true).unwrap();
    /// assert!(matches!(r, SetOutcome::Created(1)));
    /// ```
    pub fn set(index: &mut Vec<usize>, data: &mut Vec<usize>, idx: Option<usize>, value: &[usize], intern: bool) -> Result<SetOutcome, ListError> {
        match idx {
            Some(idx) => {
                if idx == 0 {
                    return Err(ListError::NotExist);
                }
                let idx_start = idx * 2;
                let idx_end = idx_start + 2;
                if idx_end > index.len() {
                    return Err(ListError::OutOfBounds);
                }
                if is_vacant(&index[idx_start..idx_end]) {
                    return Err(ListError::NotExist);
                }
                let old_start = index[idx_start];
                let old_end   = index[idx_start + 1];
                let old_len   = old_end - old_start;
                if value.len() <= old_len {
                    // in-place: value fits within the existing slot
                    data[old_start..old_start + value.len()].copy_from_slice(value);
                    index[idx_start + 1] = old_start + value.len();
                } else {
                    // append: value does not fit; old bytes are unreachable until compact
                    let start = data.len();
                    let end = start + value.len();
                    data.extend_from_slice(value);
                    index[idx_start..idx_end].copy_from_slice(&[start, end]);
                }
                Ok(SetOutcome::Updated)
            }
            None => {
                if intern {
                    let count = index.len() / 2;
                    for i in 1..count {
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

    /// index: line that has ranges
    /// data:  line that has values
    /// idx:   index number of target (1-based; idx=0 is the null sentinel)
    ///
    /// example:
    /// ```
    /// use context_engine::list::variable_list;
    /// use context_engine::list::ListError;
    /// // idx=0 sentinel, idx=1 -> [1,2,3], idx=2 -> [4,5,6]
    /// let mut index = vec![0, 0, 0, 3, 3, 6];
    /// variable_list::delete(&mut index, 1).unwrap();
    /// assert!(matches!(variable_list::get(&index, &[1,2,3,4,5,6], 1), Err(ListError::NotExist)));
    /// ```
    pub fn delete(index: &mut Vec<usize>, idx: usize) -> Result<(), ListError> {
        if idx == 0 {
            return Err(ListError::NotExist);
        }
        let idx_start = idx * 2;
        let idx_end = idx_start + 2;
        if idx_end > index.len() {
            return Err(ListError::OutOfBounds);
        }
        index[idx_start..idx_end].fill(0);
        Ok(())
    }

    /// index: line that has ranges
    /// data:  line that has values
    ///
    /// Rebuilds both index and data from scratch:
    /// - vacant slots are removed from index (index shrinks)
    /// - update-leaked bytes in data are reclaimed
    /// - idx=0 sentinel is preserved at the head of the new index
    /// - surviving entries are re-assigned sequential idx values starting at 1
    ///
    /// Returns a mapping of old idx -> new idx for callers that hold external references.
    ///
    /// example:
    /// ```
    /// use context_engine::list::variable_list;
    /// // idx=0 sentinel, idx=1 -> [1,2,3], idx=2 is vacant, idx=3 -> [4,5,6]
    /// let mut index = vec![0, 0, 0, 3, 0, 0, 3, 6];
    /// let mut data  = vec![1, 2, 3, 4, 5, 6];
    /// let remap = variable_list::compact(&mut index, &mut data).unwrap();
    /// // vacant idx=2 removed; survivors re-assigned to idx=1 and idx=2
    /// assert_eq!(remap[&1], 1);
    /// assert_eq!(remap[&3], 2);
    /// assert_eq!(variable_list::get(&index, &data, 1).unwrap(), &[1, 2, 3]);
    /// assert_eq!(variable_list::get(&index, &data, 2).unwrap(), &[4, 5, 6]);
    /// ```
    pub fn compact(index: &mut Vec<usize>, data: &mut Vec<usize>) -> Result<alloc::collections::BTreeMap<usize, usize>, VariableListError> {
        let mut new_index = vec![0, 0]; // idx=0 sentinel
        let mut new_data  = Vec::new();
        let mut remap     = alloc::collections::BTreeMap::new();
        let count = index.len() / 2;
        // skip i=0 (sentinel)
        for i in 1..count {
            let idx_start = i * 2;
            if is_vacant(&index[idx_start..idx_start + 2]) {
                continue;
            }
            let start = index[idx_start];
            let end   = index[idx_start + 1];
            let slice = data.get(start..end).ok_or(VariableListError::Compact)?;
            let new_start = new_data.len();
            new_data.extend_from_slice(slice);
            let new_end = new_data.len();
            let new_idx = new_index.len() / 2;
            new_index.push(new_start);
            new_index.push(new_end);
            remap.insert(i, new_idx);
        }
        *index = new_index;
        *data  = new_data;
        Ok(remap)
    }
}

// --- List struct (Store impl) ---

/// Fixed-width slot store backed by a flat Vec<usize>.
/// Identity, Schema, Delegate are unused (pass `&()`).
/// Index is the slot index (1-based; 0 is the null sentinel).
/// The unit (slot width) is fixed at construction time.
pub struct List {
    pub data: Vec<usize>,
    pub unit: usize,
}

impl List {
    pub fn new(unit: usize) -> Self {
        Self { data: Vec::new(), unit }
    }
}

impl Store for List {
    type Identity = ();
    type Index    = usize;
    type Schema   = ();
    type Delegate = ();
    type Error    = ListError;
    type Value    = [usize];

    fn get(
        &self,
        _identity: &(),
        index: &usize,
        _schema: &(),
        _delegate: &(),
    ) -> Result<&[usize], ListError> {
        list::get(&self.data, *index, self.unit)
    }

    fn set(
        &mut self,
        _identity: &(),
        index: &usize,
        _schema: &(),
        _delegate: &(),
        value: &[usize],
        reuse_vacant: bool,
    ) -> Result<SetOutcome, ListError> {
        let idx = if *index == 0 { None } else { Some(*index) };
        list::set(&mut self.data, idx, self.unit, value, reuse_vacant)
    }

    fn delete(
        &mut self,
        _identity: &(),
        index: &usize,
        _schema: &(),
        _delegate: &(),
    ) -> Result<(), ListError> {
        list::delete(&mut self.data, *index, self.unit)
    }
}

// --- VariableList struct (Store impl) ---

/// Variable-width slot store backed by an index Vec and a data Vec.
/// Identity, Schema, Delegate are unused (pass `&()`).
/// Index is the slot index (1-based; 0 is the null sentinel).
/// set with index=0 appends a new entry; set with index>0 updates in-place or re-appends.
pub struct VariableList {
    pub index: Vec<usize>,
    pub data:  Vec<usize>,
}

impl VariableList {
    pub fn new() -> Self {
        Self { index: Vec::new(), data: Vec::new() }
    }

    pub fn compact(&mut self) -> Result<BTreeMap<usize, usize>, VariableListError> {
        variable_list::compact(&mut self.index, &mut self.data)
    }
}

impl Store for VariableList {
    type Identity = ();
    type Index    = usize;
    type Schema   = ();
    type Delegate = ();
    type Error    = ListError;
    type Value    = [usize];

    fn get(
        &self,
        _identity: &(),
        index: &usize,
        _schema: &(),
        _delegate: &(),
    ) -> Result<&[usize], ListError> {
        variable_list::get(&self.index, &self.data, *index)
    }

    fn set(
        &mut self,
        _identity: &(),
        index: &usize,
        _schema: &(),
        _delegate: &(),
        value: &[usize],
        intern: bool,
    ) -> Result<SetOutcome, ListError> {
        let idx = if *index == 0 { None } else { Some(*index) };
        variable_list::set(&mut self.index, &mut self.data, idx, value, intern)
    }

    fn delete(
        &mut self,
        _identity: &(),
        index: &usize,
        _schema: &(),
        _delegate: &(),
    ) -> Result<(), ListError> {
        variable_list::delete(&mut self.index, *index)
    }
}
