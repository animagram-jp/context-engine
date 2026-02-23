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
    /// use state_engine::common::pool::DynamicPool;
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
    /// use state_engine::common::pool::DynamicPool;
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

/// Stores path sequences as lists of dynamic pool indices.
/// Index 0 is reserved as null.
pub struct PathMap {
    slots: Vec<Vec<u16>>,
}

impl PathMap {
    pub fn new() -> Self {
        let mut map = Self { slots: Vec::new() };
        map.slots.push(Vec::new()); // index 0 = null
        map
    }

    /// Appends a path (as dynamic pool indices) and returns its index.
    pub fn push(&mut self, indices: Vec<u16>) -> u16 {
        let idx = self.slots.len() as u16;
        self.slots.push(indices);
        idx
    }

    /// Returns the path indices at the given index.
    pub fn get(&self, index: u16) -> Option<&[u16]> {
        self.slots.get(index as usize).map(|v| v.as_slice())
    }
}

impl Default for PathMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Stores child key index lists for nodes with multiple children.
/// Index 0 is reserved as null.
pub struct ChildrenMap {
    slots: Vec<Vec<u16>>,
}

impl ChildrenMap {
    pub fn new() -> Self {
        let mut map = Self { slots: Vec::new() };
        map.slots.push(Vec::new()); // index 0 = null
        map
    }

    /// Appends a children list (as key list indices) and returns its index.
    pub fn push(&mut self, indices: Vec<u16>) -> u16 {
        let idx = self.slots.len() as u16;
        self.slots.push(indices);
        idx
    }

    /// Returns the child indices at the given index.
    pub fn get(&self, index: u16) -> Option<&[u16]> {
        self.slots.get(index as usize).map(|v| v.as_slice())
    }
}

impl Default for ChildrenMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Stores key records (u64) forming the Trie structure.
/// Index 0 is reserved as null.
pub struct KeyList {
    slots: Vec<u64>,
}

impl KeyList {
    pub fn new() -> Self {
        let mut list = Self { slots: Vec::new() };
        list.slots.push(0u64); // index 0 = null
        list
    }

    /// Appends a key record and returns its index.
    pub fn push(&mut self, record: u64) -> u16 {
        let idx = self.slots.len() as u16;
        self.slots.push(record);
        idx
    }

    /// Returns the key record at the given index.
    pub fn get(&self, index: u16) -> Option<u64> {
        self.slots.get(index as usize).copied()
    }

    /// Updates the key record at the given index.
    pub fn set(&mut self, index: u16, record: u64) {
        if let Some(slot) = self.slots.get_mut(index as usize) {
            *slot = record;
        }
    }
}

impl Default for KeyList {
    fn default() -> Self {
        Self::new()
    }
}

/// Stores YAML value records ([u64; 2]) for leaf values.
/// Index 0 is reserved as null.
pub struct YamlValueList {
    slots: Vec<[u64; 2]>,
}

impl YamlValueList {
    pub fn new() -> Self {
        let mut list = Self { slots: Vec::new() };
        list.slots.push([0u64; 2]); // index 0 = null
        list
    }

    /// Appends a value record and returns its index.
    pub fn push(&mut self, record: [u64; 2]) -> u16 {
        let idx = self.slots.len() as u16;
        self.slots.push(record);
        idx
    }

    /// Returns the value record at the given index.
    pub fn get(&self, index: u16) -> Option<[u64; 2]> {
        self.slots.get(index as usize).copied()
    }
}

impl Default for YamlValueList {
    fn default() -> Self {
        Self::new()
    }
}
