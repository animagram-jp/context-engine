extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Interns unique strings and assigns each a u16 index.
/// Index 0 is reserved as null.
pub struct DynamicPool {
    slots: Vec<String>,
}

impl DynamicPool {
    pub fn new() -> Self {
        let mut slots = Vec::new();
        slots.push(String::new()); // index 0 = null
        Self { slots }
    }

    pub fn intern(&mut self, s: &str) -> u16 {
        if let Some(idx) = self.slots.iter().position(|x| x == s) {
            return idx as u16;
        }
        let idx = self.slots.len() as u16;
        self.slots.push(s.to_string());
        idx
    }

    pub fn get(&self, index: u16) -> Option<&str> {
        self.slots.get(index as usize).map(|s: &String| s.as_str())
    }
}

impl Default for DynamicPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_dedup() {
        let mut pool = DynamicPool::new();
        let i0 = pool.intern("foo");
        let i1 = pool.intern("bar");
        assert_eq!(pool.intern("foo"), i0);
        assert_ne!(i0, i1);
        assert_ne!(i0, 0);
    }

    #[test]
    fn test_get() {
        let mut pool = DynamicPool::new();
        let i0 = pool.intern("foo");
        assert_eq!(pool.get(i0), Some("foo"));
        assert_eq!(pool.get(0), Some(""));  // null slot
        assert_eq!(pool.get(999), None);
    }
}
