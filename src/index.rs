use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    vec::Vec
};
use core::str::from_utf8;

use crate::dsl::{
    PATH_IS_LEAF_MASK,
    PATH_OFFSET_SHIFT, PATH_OFFSET_MASK,
    PATH_COUNT_SHIFT,  PATH_COUNT_MASK,
    PATH_PARENT_IDX_SHIFT, PATH_PARENT_IDX_MASK,
    PATH_KEYWORD_IDX_MASK,
};
use crate::ports::provided::Tree;

// ── LeafRef ───────────────────────────────────────────────────────────────────

pub struct LeafRef {
    pub path_idx:    u32,
    pub parent_idx:  u32,
    pub leaf_offset: u32,
}

// ── Index ─────────────────────────────────────────────────────────────────────

pub struct Index {
    paths:         Box<[u64]>,
    children:      Box<[u16]>,
    leaves:        Box<[u32]>,
    interning:     Box<[u8]>,
    interning_idx: Box<[u64]>,
}

impl Index {
    pub fn new(
        paths:         Box<[u64]>,
        children:      Box<[u16]>,
        leaves:        Box<[u32]>,
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

    /// Extract _load client keyword, map entries, and args for the given leaf.
    /// Returns ("", empty, empty) if no _load is configured.
    pub fn load_args(&self, leaf: &LeafRef) -> (&str, Vec<(Tree, Tree)>, BTreeMap<String, Tree>) {
        self.decode_meta(leaf.path_idx, leaf.leaf_offset, MetaKind::Load)
    }

    /// Extract _store client keyword, map entries, and args for the given leaf.
    /// Returns ("", empty, empty) if no _store is configured.
    pub fn store_args(&self, leaf: &LeafRef) -> (&str, Vec<(Tree, Tree)>, BTreeMap<String, Tree>) {
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
            let child_idx = self.children[offset + i] as u32;
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
            let parent_idx  = ((path & PATH_PARENT_IDX_MASK) >> PATH_PARENT_IDX_SHIFT) as u32;
            out.push(LeafRef { path_idx, parent_idx, leaf_offset });
            return;
        }
        let offset = ((path & PATH_OFFSET_MASK) >> PATH_OFFSET_SHIFT) as usize;
        let count  = (((path & PATH_COUNT_MASK) >> PATH_COUNT_SHIFT) & 0xf) as usize;
        for i in 0..count {
            self.collect_leaves(self.children[offset + i] as u32, out);
        }
    }

    /// Decode _load or _store from `leaves` at `leaf_offset`.
    /// Leaf layout: Architecture.md #leaf 参照
    fn decode_meta(&self, path_idx: u32, leaf_offset: u32, kind: MetaKind) -> (&str, Vec<(Tree, Tree)>, BTreeMap<String, Tree>) {
        let base  = leaf_offset as usize;
        let empty_map  = Vec::new();
        let empty_args = BTreeMap::new();

        if path_idx as usize >= self.paths.len() { return ("", empty_map, empty_args); }

        // header u32[0]: keyword_idx(16) | fragment_count(8) | load_map_count(8)
        let h0 = self.leaves[base];
        let fragment_count  = ((h0 >> 8) & 0xff) as usize;
        let load_map_count  = (h0 & 0xff) as usize;

        // header u32[1]: load_args_count(8) | store_map_count(8) | store_args_count(8) | padding(8)
        let h1 = self.leaves[base + 1];
        let load_args_count  = ((h1 >> 24) & 0xff) as usize;
        let store_map_count  = ((h1 >> 16) & 0xff) as usize;
        let store_args_count = ((h1 >> 8)  & 0xff) as usize;

        // header u32[2]: load_client_idx(16) | load_key_idx(16)
        let h2 = self.leaves[base + 2];
        let load_client_idx = ((h2 >> 16) & 0xffff) as usize;
        let load_key_idx    = (h2 & 0xffff) as usize;

        // header u32[3]: store_client_idx(16) | store_key_idx(16)
        let h3 = self.leaves[base + 3];
        let store_client_idx = ((h3 >> 16) & 0xffff) as usize;
        let store_key_idx    = (h3 & 0xffff) as usize;

        // variable section offsets
        let frag_start      = base + 4;
        let lmap_start      = frag_start + fragment_count;
        let largs_start     = lmap_start + load_map_count;
        let smap_start      = largs_start + load_args_count;
        let sargs_start     = smap_start + store_map_count;

        let (client_idx, key_idx, map_start, map_count, args_start, args_count) = match kind {
            MetaKind::Load  => (load_client_idx,  load_key_idx,  lmap_start,  load_map_count,  largs_start, load_args_count),
            MetaKind::Store => (store_client_idx, store_key_idx, smap_start,  store_map_count, sargs_start, store_args_count),
        };

        let client_name = from_utf8(self.interning_str(client_idx)).unwrap_or("");
        if client_name.is_empty() {
            return ("", empty_map, empty_args);
        }

        // map entries
        let mut map: Vec<(Tree, Tree)> = Vec::with_capacity(map_count);
        for i in 0..map_count {
            let entry = self.leaves[map_start + i];
            let dst = self.interning_str((entry >> 16) as usize).to_vec();
            let src = self.interning_str((entry & 0xffff) as usize).to_vec();
            map.push((Tree::Scalar(dst), Tree::Scalar(src)));
        }

        // scalar args
        let mut args: BTreeMap<String, Tree> = BTreeMap::new();
        let key_str = from_utf8(self.interning_str(key_idx)).unwrap_or("");
        if !key_str.is_empty() {
            args.insert(String::from("key"), Tree::Scalar(key_str.as_bytes().to_vec()));
        }
        for i in 0..args_count {
            let entry = self.leaves[args_start + i];
            let ak = from_utf8(self.interning_str((entry >> 16) as usize)).unwrap_or("");
            let av = self.interning_str((entry & 0xffff) as usize);
            if !ak.is_empty() {
                args.insert(String::from(ak), Tree::Scalar(av.to_vec()));
            }
        }

        (client_name, map, args)
    }

    /// Read a u32 from `leaves` at element index `idx`.
    fn read_u32(&self, idx: usize) -> u32 {
        self.leaves[idx]
    }

    /// Resolve interning bytes by interning_idx index.
    fn interning_str(&self, idx: usize) -> &[u8] {
        if idx >= self.interning_idx.len() { return b""; }
        let entry  = self.interning_idx[idx];
        let offset = (entry >> 32) as usize;
        let len    = (entry & 0xffff) as usize; // bits15..0 = len(u16)
        self.interning.get(offset..offset + len).unwrap_or(b"")
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use crate::dsl::Dsl;

    fn scalar(s: &str) -> Tree { Tree::Scalar(s.as_bytes().to_vec()) }
    fn mapping(pairs: Vec<(&str, Tree)>) -> Tree {
        Tree::Mapping(pairs.into_iter().map(|(k, v)| (k.as_bytes().to_vec(), v)).collect())
    }

    fn make_index(tree: &Tree) -> Index {
        let (paths, children, leaves, interning, interning_idx) = Dsl::compile(tree);
        Index::new(paths, children, leaves, interning, interning_idx)
    }

    // --- traverse ---

    #[test]
    fn traverse_leaf_path() {
        let idx = make_index(&mapping(vec![
            ("session", mapping(vec![
                ("user", mapping(vec![
                    ("id", Tree::Null),
                ])),
            ])),
        ]));
        let leaves = idx.traverse("session.user.id");
        assert_eq!(leaves.len(), 1);
    }

    #[test]
    fn traverse_intermediate_collects_all_leaves() {
        let idx = make_index(&mapping(vec![
            ("session", mapping(vec![
                ("user", mapping(vec![
                    ("id",   Tree::Null),
                    ("name", Tree::Null),
                ])),
            ])),
        ]));
        let leaves = idx.traverse("session.user");
        assert_eq!(leaves.len(), 2);
    }

    #[test]
    fn traverse_nonexistent_returns_empty() {
        let idx = make_index(&mapping(vec![
            ("session", mapping(vec![
                ("user", mapping(vec![
                    ("id", Tree::Null),
                ])),
            ])),
        ]));
        let leaves = idx.traverse("session.user.missing");
        assert!(leaves.is_empty());
    }

    // --- keyword_of ---

    #[test]
    fn keyword_of_leaf() {
        let idx = make_index(&mapping(vec![
            ("user", mapping(vec![
                ("id", Tree::Null),
            ])),
        ]));
        // root(0), user(1), id(2)
        assert_eq!(idx.keyword_of(2), b"id");
    }

    #[test]
    fn keyword_of_intermediate() {
        let idx = make_index(&mapping(vec![
            ("user", mapping(vec![
                ("id", Tree::Null),
            ])),
        ]));
        assert_eq!(idx.keyword_of(1), b"user");
    }

    // --- load_args ---

    #[test]
    fn load_args_client_name() {
        let idx = make_index(&mapping(vec![
            ("session", mapping(vec![
                ("_load", mapping(vec![
                    ("client", scalar("Memory")),
                    ("key",    scalar("session:1")),
                ])),
                ("user", mapping(vec![
                    ("id", Tree::Null),
                ])),
            ])),
        ]));
        let leaves = idx.traverse("session.user.id");
        let (client, _, _) = idx.load_args(&leaves[0]);
        assert_eq!(client, "Memory");
    }

    #[test]
    fn load_args_key() {
        let idx = make_index(&mapping(vec![
            ("session", mapping(vec![
                ("_load", mapping(vec![
                    ("client", scalar("Memory")),
                    ("key",    scalar("session:1")),
                ])),
                ("user", mapping(vec![
                    ("id", Tree::Null),
                ])),
            ])),
        ]));
        let leaves = idx.traverse("session.user.id");
        let (_, _, args) = idx.load_args(&leaves[0]);
        assert_eq!(args.get("key"), Some(&Tree::Scalar(b"session:1".to_vec())));
    }

    #[test]
    fn load_args_no_load_returns_empty() {
        let idx = make_index(&mapping(vec![
            ("user", mapping(vec![
                ("id", Tree::Null),
            ])),
        ]));
        let leaves = idx.traverse("user.id");
        let (client, _, args) = idx.load_args(&leaves[0]);
        assert!(client.is_empty() && args.is_empty());
    }

    // --- store_args ---

    #[test]
    fn store_args_client_name() {
        let idx = make_index(&mapping(vec![
            ("session", mapping(vec![
                ("_store", mapping(vec![
                    ("client", scalar("Kvs")),
                    ("key",    scalar("session:1")),
                ])),
                ("user", mapping(vec![
                    ("id", Tree::Null),
                ])),
            ])),
        ]));
        let leaves = idx.traverse("session.user.id");
        let (client, _, _) = idx.store_args(&leaves[0]);
        assert_eq!(client, "Kvs");
    }

    #[test]
    fn store_args_no_store_returns_empty() {
        let idx = make_index(&mapping(vec![
            ("user", mapping(vec![
                ("id", Tree::Null),
            ])),
        ]));
        let leaves = idx.traverse("user.id");
        let (client, _, args) = idx.store_args(&leaves[0]);
        assert!(client.is_empty() && args.is_empty());
    }
}
