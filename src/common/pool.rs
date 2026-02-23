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

/// State value record: 32-bit fixed-length record.
///
/// Layout:
/// | field       | bit |
/// |-------------|-----|
/// | key_index   |  16 |
/// | value_index |  16 |
///
/// - key_index:   index into KeyList (for type info via key record's type index)
/// - value_index: index into StateValueList's value buffer (serde_json::Value)
pub type StateRecord = u32;

pub const STATE_OFFSET_KEY:   u32 = 16;
pub const STATE_OFFSET_VALUE: u32 = 0;
pub const STATE_MASK_KEY:   u32 = 0xFFFF;
pub const STATE_MASK_VALUE: u32 = 0xFFFF;

pub fn state_new() -> StateRecord { 0 }

pub fn state_get(record: StateRecord, offset: u32, mask: u32) -> u16 {
    ((record >> offset) & mask) as u16
}

pub fn state_set(record: StateRecord, offset: u32, mask: u32, value: u16) -> StateRecord {
    (record & !(mask << offset)) | (((value as u32) & mask) << offset)
}

/// Stores state value records (u32) and their associated serde_json::Value payloads.
/// Index 0 is reserved as null.
pub struct StateValueList {
    records: Vec<StateRecord>,
    values:  Vec<serde_json::Value>,
}

impl StateValueList {
    pub fn new() -> Self {
        let mut list = Self {
            records: Vec::new(),
            values:  Vec::new(),
        };
        list.records.push(0);
        list.values.push(serde_json::Value::Null);
        list
    }

    /// Appends a state record and its value, returning the record index.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::common::pool::{StateValueList, STATE_OFFSET_KEY, STATE_MASK_KEY, STATE_OFFSET_VALUE, STATE_MASK_VALUE, state_get};
    /// use serde_json::json;
    ///
    /// let mut list = StateValueList::new();
    /// let idx = list.push(42, json!("hello"));
    ///
    /// let record = list.get_record(idx).unwrap();
    /// assert_eq!(state_get(record, STATE_OFFSET_KEY, STATE_MASK_KEY), 42);
    ///
    /// let value = list.get_value(idx).unwrap();
    /// assert_eq!(value, &json!("hello"));
    /// ```
    pub fn push(&mut self, key_index: u16, value: serde_json::Value) -> u16 {
        let value_index = self.values.len() as u16;
        self.values.push(value);

        let record = state_set(state_new(), STATE_OFFSET_KEY, STATE_MASK_KEY, key_index);
        let record = state_set(record, STATE_OFFSET_VALUE, STATE_MASK_VALUE, value_index);
        self.records.push(record);

        (self.records.len() - 1) as u16
    }

    /// Updates the value at the given record index.
    pub fn update(&mut self, index: u16, value: serde_json::Value) -> bool {
        let record = match self.records.get(index as usize) {
            Some(&r) => r,
            None => return false,
        };
        let value_index = state_get(record, STATE_OFFSET_VALUE, STATE_MASK_VALUE) as usize;
        if let Some(slot) = self.values.get_mut(value_index) {
            *slot = value;
            true
        } else {
            false
        }
    }

    /// Removes a record by zeroing it (index remains valid, value becomes Null).
    pub fn remove(&mut self, index: u16) -> bool {
        let record = match self.records.get(index as usize) {
            Some(&r) => r,
            None => return false,
        };
        let value_index = state_get(record, STATE_OFFSET_VALUE, STATE_MASK_VALUE) as usize;
        if let Some(slot) = self.values.get_mut(value_index) {
            *slot = serde_json::Value::Null;
            self.records[index as usize] = 0;
            true
        } else {
            false
        }
    }

    pub fn get_record(&self, index: u16) -> Option<StateRecord> {
        self.records.get(index as usize).copied()
    }

    pub fn get_value(&self, index: u16) -> Option<&serde_json::Value> {
        let record = self.records.get(index as usize)?;
        let value_index = state_get(*record, STATE_OFFSET_VALUE, STATE_MASK_VALUE);
        self.values.get(value_index as usize)
    }
}

impl Default for StateValueList {
    fn default() -> Self {
        Self::new()
    }
}
