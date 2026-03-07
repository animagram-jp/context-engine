use std::collections::HashMap;

/// Interns unique strings and assigns each a u16 index.
/// Index 0 is reserved as null.
pub struct DynamicPool {
    slots: Vec<String>,
    forward: HashMap<String, u16>,
}

impl DynamicPool {
    pub fn new() -> Self {
        let mut pool = Self {
            slots: Vec::new(),
            forward: HashMap::new(),
        };
        pool.slots.push(String::new()); // index 0 = null
        pool
    }

    /// Interns a string and returns its index.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine_core::common::pool::DynamicPool;
    ///
    /// let mut pool = DynamicPool::new();
    /// let i0 = pool.intern("foo");
    /// let i1 = pool.intern("bar");
    ///
    /// // same string returns same index
    /// assert_eq!(pool.intern("foo"), i0);
    /// assert_ne!(i0, i1);
    ///
    /// // index 0 is reserved as null
    /// assert_ne!(i0, 0);
    /// ```
    pub fn intern(&mut self, s: &str) -> u16 {
        if let Some(&idx) = self.forward.get(s) {
            return idx;
        }
        let idx = self.slots.len() as u16;
        self.slots.push(s.to_string());
        self.forward.insert(s.to_string(), idx);
        idx
    }

    /// Returns the string at the given index, or None if out of range.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine_core::common::pool::DynamicPool;
    ///
    /// let mut pool = DynamicPool::new();
    /// let i0 = pool.intern("foo");
    /// assert_eq!(pool.get(i0), Some("foo"));
    /// assert_eq!(pool.get(0), Some(""));  // null slot
    /// assert_eq!(pool.get(999), None);
    /// ```
    pub fn get(&self, index: u16) -> Option<&str> {
        self.slots.get(index as usize).map(|s| s.as_str())
    }
}

impl Default for DynamicPool {
    fn default() -> Self {
        Self::new()
    }
}
