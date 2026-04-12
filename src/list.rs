// --- file global ---

// --- StoreClient (draft) ----

// pub trait StoreError {}

// pub trait StoreClient {
//     type Error: StoreError;

//     /// index: line that has ranges
//     /// data:  line that has values
//     /// idx:   index number of target
//     fn get(&mut self, index: &[usize], data: &[usize], idx: usize) -> Result<&[usize], Self::Error>;
//     /// index: line that has ranges
//     /// data:  line that has values
//     /// idx:   index number of target (optional)
//     /// value:
//     /// intern: when idx: null, search data and return first-match idx or not
//     fn set(&mut self, index: &mut Vec<usize>, data: &mut Vec<usize>, idx: Option<usize>, value: &[usize], intern: bool) -> Result<SetOutcome, Self::Error>;
//     /// index: line that has ranges
//     /// data:  line that has values
//     /// idx:   index number of target
//     fn delete(&mut self, index: &mut Vec<usize>, idx: usize) -> Result<(), Self::Error>;
//     fn compact(&mut self, index: &mut Vec<usize>, data: &mut Vec<usize>) -> Result<(), Self::Error>;
// }

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
    /// idx:  index number of target
    /// unit: units of target extent
    /// value:
    /// resue_vacant: write to first match 00...00 slice
    pub fn set(list: &mut Vec<usize>, idx: Option<usize>, unit: usize, value: &[usize], reuse_vacant: bool) -> Result<SetOutcome, ListError> {
        if value.len() != unit {
            return Err(ListError::OutOfBounds);
        }
        match idx {
            Some(idx) => {
                let start = idx * unit;
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
    use core::result::Result;
    use super::{ListError, SetOutcome, VariableListError};
    use super::list;

    fn is_vacant(slot: &[usize]) -> bool {
        slot.iter().all(|&x| x == 0)
    }

    /// index: line that has ranges
    /// data:  line that has values
    /// idx:   index number of target
    ///
    /// example:
    /// ```
    /// use context_engine::list::variable_list;
    /// let index = vec![0, 3, 3, 6];
    /// let data  = vec![1, 2, 3, 4, 5, 6];
    /// assert_eq!(variable_list::get(&index, &data, 0).unwrap(), &[1, 2, 3]);
    /// assert_eq!(variable_list::get(&index, &data, 1).unwrap(), &[4, 5, 6]);
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
    /// idx:   index number of target (optional)
    /// value:
    /// intern: when idx: null, search data and return first-match idx or not
    ///
    /// note: when writing, always appends-only (warning!: to both index and data).
    ///
    /// example:
    /// ```
    /// use context_engine::list::variable_list;
    /// use context_engine::list::SetOutcome;
    /// let mut index = vec![];
    /// let mut data  = vec![];
    ///
    /// // append
    /// let r = variable_list::set(&mut index, &mut data, None, &[1, 2, 3], false).unwrap();
    /// assert!(matches!(r, SetOutcome::Created(0)));
    /// assert_eq!(variable_list::get(&index, &data, 0).unwrap(), &[1, 2, 3]);
    ///
    /// // update
    /// let r = variable_list::set(&mut index, &mut data, Some(0), &[7, 8, 9], false).unwrap();
    /// assert!(matches!(r, SetOutcome::Updated));
    /// assert_eq!(variable_list::get(&index, &data, 0).unwrap(), &[7, 8, 9]);
    ///
    /// // intern: same value returns existing idx
    /// let r = variable_list::set(&mut index, &mut data, None, &[7, 8, 9], true).unwrap();
    /// assert!(matches!(r, SetOutcome::Created(0)));
    /// ```
    pub fn set(index: &mut Vec<usize>, data: &mut Vec<usize>, idx: Option<usize>, value: &[usize], intern: bool) -> Result<SetOutcome, ListError> {
        match idx {
            Some(idx) => {
                let idx_start = idx * 2;
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

    /// index: line that has ranges
    /// data:  line that has values
    /// idx:   index number of target
    ///
    /// example:
    /// ```
    /// use context_engine::list::variable_list;
    /// use context_engine::list::ListError;
    /// let mut index = vec![0, 3, 3, 6];
    /// variable_list::delete(&mut index, 0).unwrap();
    /// assert!(matches!(variable_list::get(&index, &[1,2,3,4,5,6], 0), Err(ListError::NotExist)));
    /// ```
    pub fn delete(index: &mut Vec<usize>, idx: usize) -> Result<(), ListError> {
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
    /// example:
    /// ```
    /// use context_engine::list::variable_list;
    /// let mut index = vec![0, 3, 0, 0, 3, 6]; // idx=1 is vacant
    /// let mut data  = vec![1, 2, 3, 4, 5, 6];
    /// variable_list::compact(&mut index, &mut data).unwrap();
    /// assert_eq!(data, vec![1, 2, 3, 4, 5, 6]); // vacant skipped, data compacted
    /// assert_eq!(variable_list::get(&index, &data, 0).unwrap(), &[1, 2, 3]);
    /// assert_eq!(variable_list::get(&index, &data, 2).unwrap(), &[4, 5, 6]);
    /// ```
    pub fn compact(index: &mut Vec<usize>, data: &mut Vec<usize>) -> Result<(), VariableListError> {
        let mut new_data = Vec::new();
        let count = index.len() / 2;
        for i in 0..count {
            let idx_start = i * 2;
            let start = index[idx_start];
            let end = index[idx_start + 1];
            if is_vacant(&index[idx_start..idx_start + 2]) {
                continue;
            }
            let slice = data.get(start..end).ok_or(VariableListError::Compact)?;
            let new_start = new_data.len();
            new_data.extend_from_slice(slice);
            let new_end = new_data.len();
            index[idx_start] = new_start;
            index[idx_start + 1] = new_end;
        }
        *data = new_data;
        Ok(())
    }
}
