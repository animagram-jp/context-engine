use std::collections::hash_map::Entry;
use std::collections::HashMap;

pub struct BiMap {
    slots: Vec<Option<Vec<u8>>>,
    forward: HashMap<Vec<u8>, usize>,
    free: Vec<usize>,
}

impl BiMap {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            forward: HashMap::new(),
            free: Vec::new(),
        }
    }

    /// Returns the index for `value`, interning it if not already present.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::common::BiMap;
    ///
    /// let mut map = BiMap::new();
    /// let i0 = map.set(b"alpha".to_vec());
    /// let i1 = map.set(b"beta".to_vec());
    ///
    /// // same value returns same index (intern)
    /// assert_eq!(map.set(b"alpha".to_vec()), i0);
    /// assert_ne!(i0, i1);
    /// ```
    pub fn set(&mut self, value: Vec<u8>) -> usize {
        match self.forward.entry(value) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let idx = if let Some(idx) = self.free.pop() {
                    self.slots[idx] = Some(e.key().clone());
                    idx
                } else {
                    let idx = self.slots.len();
                    self.slots.push(Some(e.key().clone()));
                    idx
                };
                e.insert(idx);
                idx
            }
        }
    }

    /// Returns the value at `index`, or `None` if absent or out of range.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::common::BiMap;
    ///
    /// let mut map = BiMap::new();
    /// let i0 = map.set(b"alpha".to_vec());
    ///
    /// assert_eq!(map.get(i0), Some(b"alpha".as_ref()));
    /// assert_eq!(map.get(99), None); // out of range
    ///
    /// map.unset(i0);
    /// assert_eq!(map.get(i0), None); // unset
    /// ```
    pub fn get(&self, index: usize) -> Option<&[u8]> {
        self.slots.get(index)?.as_deref()
    }

    /// Returns `true` if `index` is occupied.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::common::BiMap;
    ///
    /// let mut map = BiMap::new();
    /// let i0 = map.set(b"alpha".to_vec());
    ///
    /// assert!(map.exists(i0));
    /// assert!(!map.exists(99)); // out of range
    ///
    /// map.unset(i0);
    /// assert!(!map.exists(i0));
    /// ```
    pub fn exists(&self, index: usize) -> bool {
        matches!(self.slots.get(index), Some(Some(_)))
    }

    /// Removes the value at `index`. Returns `true` if it was present.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::common::BiMap;
    ///
    /// let mut map = BiMap::new();
    /// let i0 = map.set(b"alpha".to_vec());
    /// let i1 = map.set(b"beta".to_vec());
    ///
    /// assert!(map.unset(i0));
    /// assert!(!map.unset(i0)); // already absent
    /// assert!(!map.unset(99)); // out of range
    ///
    /// // i1 unaffected
    /// assert_eq!(map.get(i1), Some(b"beta".as_ref()));
    /// ```
    pub fn unset(&mut self, index: usize) -> bool {
        match self.slots.get_mut(index) {
            Some(slot @ Some(_)) => {
                let value = slot.take().unwrap();
                self.forward.remove(&value);
                self.free.push(index);
                true
            }
            _ => false,
        }
    }
}

impl Default for BiMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unset_does_not_shift_indices() {
        let mut map = BiMap::new();
        let i0 = map.set(b"a".to_vec());
        let i1 = map.set(b"b".to_vec());
        let i2 = map.set(b"c".to_vec());

        map.unset(i1);

        assert_eq!(map.get(i0), Some(b"a".as_ref()));
        assert_eq!(map.get(i1), None);
        assert_eq!(map.get(i2), Some(b"c".as_ref()));
    }

    #[test]
    fn test_unset_removes_from_forward_table() {
        let mut map = BiMap::new();
        let i0 = map.set(b"alpha".to_vec());
        map.unset(i0);

        let i1 = map.set(b"alpha".to_vec());
        assert_eq!(map.get(i1), Some(b"alpha".as_ref()));

        // duplicate set now returns i1 (not i0, which is no longer in the table)
        assert_eq!(map.set(b"alpha".to_vec()), i1);
    }

    #[test]
    fn test_set_reuses_freed_slot() {
        let mut map = BiMap::new();
        let i0 = map.set(b"a".to_vec());
        let i1 = map.set(b"b".to_vec());
        map.unset(i1);

        let i2 = map.set(b"c".to_vec());
        assert_eq!(i2, i1);
        assert_eq!(map.get(i0), Some(b"a".as_ref()));
        assert_eq!(map.get(i2), Some(b"c".as_ref()));
    }

}
