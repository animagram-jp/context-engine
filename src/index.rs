use crate::dsl::{
    PATH_IS_LEAF_MASK, PATH_OFFSET_SHIFT, PATH_OFFSET_MASK, PATH_COUNT_SHIFT, PATH_COUNT_MASK,
};

// ── LeafRef ───────────────────────────────────────────────────────────────────

pub struct LeafRef {
    pub path_idx:    u32,
    pub leaf_offset: u32,
}

// ── Index ─────────────────────────────────────────────────────────────────────

pub struct Index {
    paths:         Box<[u64]>,
    children:      Box<[u32]>,
    leaves:        Box<[u8]>,
    interning:     Box<[u8]>,
    interning_idx: Box<[u64]>,
}

impl Index {
    pub fn new(
        paths:         Box<[u64]>,
        children:      Box<[u32]>,
        leaves:        Box<[u8]>,
        interning:     Box<[u8]>,
        interning_idx: Box<[u64]>,
    ) -> Self {
        Self { paths, children, leaves, interning, interning_idx }
    }

    /// Traverse to the path node matching `path` (dot-separated keywords),
    /// then collect all leaf descendants into a flat list.
    pub fn traverse(&self, path: &str) -> Box<[LeafRef]> {
        let mut result = Vec::new();
        let Some(path_idx) = self.find(path) else {
            return result.into_boxed_slice();
        };
        self.collect_leaves(path_idx, &mut result);
        result.into_boxed_slice()
    }

    /// Walk the interning list to find the path_idx matching the dot-separated `path`.
    fn find(&self, path: &str) -> Option<u32> {
        let mut current: u32 = 0; // root
        for keyword in path.split('.') {
            current = self.find_child(current, keyword.as_bytes())?;
        }
        Some(current)
    }

    /// Among the children of `path_idx`, find the one whose interning keyword matches `keyword`.
    fn find_child(&self, path_idx: u32, keyword: &[u8]) -> Option<u32> {
        let path = self.paths[path_idx as usize];
        let offset = ((path & PATH_OFFSET_MASK) >> PATH_OFFSET_SHIFT) as usize;
        let count  = (((path & PATH_COUNT_MASK) >> PATH_COUNT_SHIFT) & 0xf) as usize;

        for i in 0..count {
            let child_idx = self.children[offset + i];
            if self.keyword_of(child_idx) == keyword {
                return Some(child_idx);
            }
        }
        None
    }

    /// Recursively collect all leaf descendants of `path_idx`.
    fn collect_leaves(&self, path_idx: u32, out: &mut Vec<LeafRef>) {
        let path = self.paths[path_idx as usize];
        if path & PATH_IS_LEAF_MASK != 0 {
            let leaf_offset = ((path & PATH_OFFSET_MASK) >> PATH_OFFSET_SHIFT) as u32;
            out.push(LeafRef { path_idx, leaf_offset });
            return;
        }
        let offset = ((path & PATH_OFFSET_MASK) >> PATH_OFFSET_SHIFT) as usize;
        let count  = (((path & PATH_COUNT_MASK) >> PATH_COUNT_SHIFT) & 0xf) as usize;
        for i in 0..count {
            self.collect_leaves(self.children[offset + i], out);
        }
    }

    /// Resolve the keyword bytes of a path node from the interning list.
    pub fn keyword_of(&self, path_idx: u32) -> &[u8] {
        todo!("resolve keyword from interning via path_idx")
    }

    /// Extract _load client yaml_name and args from leaves at `leaf_offset`.
    /// Returns ("", empty) if no _load is configured.
    pub fn load_args(&self, leaf_offset: u32) -> (&str, std::collections::HashMap<String, crate::ports::provided::Tree>) {
        todo!("decode _load from leaves[leaf_offset..]")
    }

    /// Extract _store client yaml_name and args from leaves at `leaf_offset`.
    /// Returns ("", empty) if no _store is configured.
    pub fn store_args(&self, leaf_offset: u32) -> (&str, std::collections::HashMap<String, crate::ports::provided::Tree>) {
        todo!("decode _store from leaves[leaf_offset..]")
    }
}
