use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::str::from_utf8;

use crate::dsl::{
    PATH_IS_LEAF_MASK,
    PATH_OFFSET_SHIFT, PATH_OFFSET_MASK,
    PATH_COUNT_SHIFT,  PATH_COUNT_MASK,
    PATH_KEYWORD_IDX_MASK,
};
use crate::ports::provided::Tree;

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

    /// Resolve the keyword bytes of a path node from the interning list.
    pub fn keyword_of(&self, path_idx: u32) -> &[u8] {
        let path = self.paths[path_idx as usize];
        let interning_idx = (path & PATH_KEYWORD_IDX_MASK) as usize;
        self.interning_str(interning_idx)
    }

    /// Extract _load client yaml_name and args for the given leaf.
    /// Returns ("", empty) if no _load is configured.
    pub fn load_args(&self, leaf: &LeafRef) -> (&str, BTreeMap<String, Tree>) {
        self.decode_meta(leaf.path_idx, leaf.leaf_offset, MetaKind::Load)
    }

    /// Extract _store client yaml_name and args for the given leaf.
    /// Returns ("", empty) if no _store is configured.
    pub fn store_args(&self, leaf: &LeafRef) -> (&str, BTreeMap<String, Tree>) {
        self.decode_meta(leaf.path_idx, leaf.leaf_offset, MetaKind::Store)
    }
}

// ── private ───────────────────────────────────────────────────────────────────

enum MetaKind { Load, Store }

impl Index {
    /// Walk dot-separated `path` from the virtual root (paths[0]).
    fn find(&self, path: &str) -> Option<u32> {
        let mut current: u32 = 0; // paths[0] = virtual root
        for segment in path.split('.') {
            current = self.find_child(current, segment.as_bytes())?;
        }
        Some(current)
    }

    /// Among the children of `path_idx`, find the one whose keyword matches.
    fn find_child(&self, path_idx: u32, keyword: &[u8]) -> Option<u32> {
        let path   = self.paths[path_idx as usize];
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

    /// Decode _load or _store from `leaves` at `leaf_offset`.
    ///
    /// Leaf layout (u32le each):
    ///   +0  keyword_idx
    ///   +4  value_idx
    ///   +8  load_client_idx
    ///   +12 load_key_idx
    ///   +16 store_client_idx
    ///   +20 store_key_idx
    ///   +24 load.args  × load_args_count  : key_idx | value_idx
    ///   +24+N store.args × store_args_count : key_idx | value_idx
    ///
    /// args counts are in path.count: [7:4]=load, [3:0]=store
    fn decode_meta(&self, path_idx: u32, leaf_offset: u32, kind: MetaKind) -> (&str, BTreeMap<String, Tree>) {
        let base = leaf_offset as usize;
        let empty = BTreeMap::new();

        if path_idx as usize >= self.paths.len() { return ("", empty); }
        let path_entry = self.paths[path_idx as usize];

        let count_byte  = ((path_entry & PATH_COUNT_MASK) >> PATH_COUNT_SHIFT) as u8;
        let load_count  = ((count_byte >> 4) & 0xf) as usize;
        let store_count = (count_byte & 0xf) as usize;

        let (client_offset, key_offset, args_count, args_start) = match kind {
            MetaKind::Load  => (8,  12, load_count,  24),
            MetaKind::Store => (16, 20, store_count, 24 + load_count * 8),
        };

        let client_idx = self.read_u32(base + client_offset) as usize;
        let key_idx    = self.read_u32(base + key_offset) as usize;

        let client_name = from_utf8(self.interning_str(client_idx)).unwrap_or("");
        if client_name.is_empty() {
            return ("", empty);
        }

        let mut args: BTreeMap<String, Tree> = BTreeMap::new();

        // key arg
        let key_str = from_utf8(self.interning_str(key_idx)).unwrap_or("");
        if !key_str.is_empty() {
            args.insert(
                String::from("key"),
                Tree::Scalar(key_str.as_bytes().to_vec()),
            );
        }

        // additional args
        for i in 0..args_count {
            let off = base + args_start + i * 8;
            let ak  = self.read_u32(off) as usize;
            let av  = self.read_u32(off + 4) as usize;
            let k   = from_utf8(self.interning_str(ak)).unwrap_or("");
            let v   = self.interning_str(av);
            if !k.is_empty() {
                args.insert(
                    String::from(k),
                    Tree::Scalar(v.to_vec()),
                );
            }
        }

        (client_name, args)
    }

    /// Read a u32le from `leaves` at byte offset `off`.
    fn read_u32(&self, off: usize) -> u32 {
        let b = &self.leaves[off..off + 4];
        u32::from_le_bytes(b.try_into().unwrap())
    }

    /// Resolve interning bytes by interning_idx index.
    fn interning_str(&self, idx: usize) -> &[u8] {
        if idx >= self.interning_idx.len() { return b""; }
        let entry  = self.interning_idx[idx];
        let offset = (entry >> 32) as usize;
        let len    = (entry & 0xffff_ffff) as usize;
        self.interning.get(offset..offset + len).unwrap_or(b"")
    }
}
