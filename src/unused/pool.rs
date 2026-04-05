extern crate alloc;
use alloc::vec::Vec;

/// Interns unique byte slices and assigns each a u16 index.
/// Index 0 is reserved as null.
pub struct DynamicPool {
    slots: Vec<Vec<u8>>,
}

impl DynamicPool {
    pub fn new() -> Self {
        let mut slots = Vec::new();
        slots.push(Vec::new()); // index 0 = null
        Self { slots }
    }

    pub fn intern(&mut self, s: &[u8]) -> u16 {
        if let Some(idx) = self.slots.iter().position(|x| x == s) {
            return idx as u16;
        }
        let idx = self.slots.len() as u16;
        self.slots.push(s.to_vec());
        idx
    }

    pub fn get(&self, index: u16) -> Option<&[u8]> {
        self.slots.get(index as usize).map(|s| s.as_slice())
    }

    /// Returns the index of an already-interned byte slice, or None if not present.
    pub fn find(&self, s: &[u8]) -> Option<u16> {
        self.slots.iter().position(|x| x.as_slice() == s).map(|i| i as u16)
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
        let i0 = pool.intern(b"foo");
        let i1 = pool.intern(b"bar");
        assert_eq!(pool.intern(b"foo"), i0);
        assert_ne!(i0, i1);
        assert_ne!(i0, 0);
    }

    #[test]
    fn test_get() {
        let mut pool = DynamicPool::new();
        let i0 = pool.intern(b"foo");
        assert_eq!(pool.get(i0), Some(b"foo".as_slice()));
        assert_eq!(pool.get(0), Some(b"".as_slice()));  // null slot
        assert_eq!(pool.get(999), None);
    }
}
