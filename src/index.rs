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
use crate::provided::Tree;

// ── LeafRef ───────────────────────────────────────────────────────────────────

/// Reference to a leaf path entry returned by `Index::traverse`.
pub struct LeafRef {
    pub path_idx:    u32,
    pub parent_idx:  u32,
    pub leaf_offset: u32,
}

// ── Index ─────────────────────────────────────────────────────────────────────

/// Compiled DSL index. Holds the five static arrays produced by `Dsl::compile` and supports path traversal.
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
    ///
    /// ```
    /// # extern crate alloc;
    /// use context_engine::{Tree, Index, dsl::Dsl};
    /// let tree = Tree::Mapping(alloc::vec![(b"id".to_vec(), Tree::Null)]);
    /// let (p, c, l, i, ii) = Dsl::compile(&tree);
    /// let idx = Index::new(p, c, l, i, ii);
    /// assert_eq!(idx.traverse("id").len(), 1);
    /// assert!(idx.traverse("missing").is_empty());
    /// ```
    pub fn traverse(&self, path: &str) -> Box<[LeafRef]> {
        let mut result = Vec::new();
        let Some(path_idx) = self.find(path) else {
            return result.into_boxed_slice();
        };
        self.collect_leaves(path_idx, &mut result);
        result.into_boxed_slice()
    }

    /// Resolve the keyword bytes of a path node from the interning list.
    ///
    /// ```
    /// # extern crate alloc;
    /// use context_engine::{Tree, Index, dsl::Dsl};
    /// let tree = Tree::Mapping(alloc::vec![(b"name".to_vec(), Tree::Null)]);
    /// let (p, c, l, i, ii) = Dsl::compile(&tree);
    /// let idx = Index::new(p, c, l, i, ii);
    /// // paths[1] = "name" leaf (paths[0] = virtual root)
    /// assert_eq!(idx.keyword_of(1), b"name");
    /// ```
    pub fn keyword_of(&self, path_idx: u32) -> &[u8] {
        let path = self.paths[path_idx as usize];
        let interning_idx = (path & PATH_KEYWORD_IDX_MASK) as usize;
        self.interning_str(interning_idx)
    }

    /// Extract _get store keyword, map entries, and args for the given leaf.
    /// Returns ("", empty, empty) if no _get is configured.
    pub fn get_args(&self, leaf: &LeafRef) -> (&str, Vec<(Tree, Tree)>, BTreeMap<String, Tree>) {
        self.decode_meta(leaf.path_idx, leaf.leaf_offset, MetaKind::Get)
    }

    /// Extract _set store keyword, map entries, and args for the given leaf.
    /// Returns ("", empty, empty) if no _set is configured.
    pub fn set_args(&self, leaf: &LeafRef) -> (&str, Vec<(Tree, Tree)>, BTreeMap<String, Tree>) {
        self.decode_meta(leaf.path_idx, leaf.leaf_offset, MetaKind::Set)
    }

    /// Return the value fragments encoded in the leaf.
    ///
    /// Each element is `(is_placeholder, bytes)`:
    /// - `is_placeholder = false` — static string literal
    /// - `is_placeholder = true`  — `${path}` reference whose bytes are the path string
    ///
    /// An empty slice means the leaf has no value (`Null`).
    ///
    /// ```
    /// # extern crate alloc;
    /// use context_engine::{Tree, Index, dsl::Dsl};
    /// let tree = Tree::Mapping(alloc::vec![
    ///     (b"driver".to_vec(), Tree::Scalar(b"postgres".to_vec())),
    /// ]);
    /// let (p, c, l, i, ii) = Dsl::compile(&tree);
    /// let idx = Index::new(p, c, l, i, ii);
    /// let leaves = idx.traverse("driver");
    /// let frags = idx.leaf_fragments(&leaves[0]);
    /// assert_eq!(frags.len(), 1);
    /// assert_eq!(frags[0].0, false);
    /// assert_eq!(frags[0].1, b"postgres");
    /// ```
    pub fn leaf_fragments(&self, leaf: &LeafRef) -> Vec<(bool, &[u8])> {
        let base = leaf.leaf_offset as usize;
        let h0 = self.leaves[base];
        let fragment_count = ((h0 >> 8) & 0xff) as usize;
        let frag_start = base + 4;
        let mut result = Vec::with_capacity(fragment_count);
        for i in 0..fragment_count {
            let word = self.leaves[frag_start + i];
            let is_placeholder = ((word >> 16) & 0x1) != 0;
            let idx = (word & 0xffff) as usize;
            result.push((is_placeholder, self.interning_str(idx)));
        }
        result
    }
}

// ── private ───────────────────────────────────────────────────────────────────

enum MetaKind { Get, Set }

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

    /// Decode _get or _set from `leaves` at `leaf_offset`.
    /// Leaf layout: Architecture.md #leaf 参照
    fn decode_meta(&self, path_idx: u32, leaf_offset: u32, kind: MetaKind) -> (&str, Vec<(Tree, Tree)>, BTreeMap<String, Tree>) {
        let base  = leaf_offset as usize;
        let empty_map  = Vec::new();
        let empty_args = BTreeMap::new();

        if path_idx as usize >= self.paths.len() { return ("", empty_map, empty_args); }

        // header u32[0]: keyword_idx(16) | fragment_count(8) | get_map_count(8)
        let h0 = self.leaves[base];
        let fragment_count = ((h0 >> 8) & 0xff) as usize;
        let get_map_count  = (h0 & 0xff) as usize;

        // header u32[1]: get_args_count(8) | set_map_count(8) | set_args_count(8) | padding(8)
        let h1 = self.leaves[base + 1];
        let get_args_count = ((h1 >> 24) & 0xff) as usize;
        let set_map_count  = ((h1 >> 16) & 0xff) as usize;
        let set_args_count = ((h1 >> 8)  & 0xff) as usize;

        // header u32[2]: get_set_idx(16) | get_key_idx(16)
        let h2 = self.leaves[base + 2];
        let get_set_idx = ((h2 >> 16) & 0xffff) as usize;
        let get_key_idx   = (h2 & 0xffff) as usize;

        // header u32[3]: set_set_idx(16) | set_key_idx(16)
        let h3 = self.leaves[base + 3];
        let set_set_idx = ((h3 >> 16) & 0xffff) as usize;
        let set_key_idx   = (h3 & 0xffff) as usize;

        // variable section offsets
        let frag_start  = base + 4;
        let gmap_start  = frag_start + fragment_count;
        let gargs_start = gmap_start + get_map_count;
        let smap_start  = gargs_start + get_args_count;
        let sargs_start = smap_start + set_map_count;

        let (store_idx, key_idx, map_start, map_count, args_start, args_count) = match kind {
            MetaKind::Get => (get_set_idx, get_key_idx, gmap_start, get_map_count, gargs_start, get_args_count),
            MetaKind::Set => (set_set_idx, set_key_idx, smap_start, set_map_count, sargs_start, set_args_count),
        };

        let store_name = from_utf8(self.interning_str(store_idx)).unwrap_or("");
        if store_name.is_empty() {
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

        (store_name, map, args)
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

    // --- get_args ---

    #[test]
    fn get_args_store_name() {
        let idx = make_index(&mapping(vec![
            ("session", mapping(vec![
                ("_get", mapping(vec![
                    ("store", scalar("Memory")),
                    ("key",    scalar("session:1")),
                ])),
                ("user", mapping(vec![
                    ("id", Tree::Null),
                ])),
            ])),
        ]));
        let leaves = idx.traverse("session.user.id");
        let (store, _, _) = idx.get_args(&leaves[0]);
        assert_eq!(store, "Memory");
    }

    #[test]
    fn get_args_key() {
        let idx = make_index(&mapping(vec![
            ("session", mapping(vec![
                ("_get", mapping(vec![
                    ("store", scalar("Memory")),
                    ("key",    scalar("session:1")),
                ])),
                ("user", mapping(vec![
                    ("id", Tree::Null),
                ])),
            ])),
        ]));
        let leaves = idx.traverse("session.user.id");
        let (_, _, args) = idx.get_args(&leaves[0]);
        assert_eq!(args.get("key"), Some(&Tree::Scalar(b"session:1".to_vec())));
    }

    #[test]
    fn get_args_no_get_returns_empty() {
        let idx = make_index(&mapping(vec![
            ("user", mapping(vec![
                ("id", Tree::Null),
            ])),
        ]));
        let leaves = idx.traverse("user.id");
        let (store, _, args) = idx.get_args(&leaves[0]);
        assert!(store.is_empty() && args.is_empty());
    }

    // --- leaf_fragments ---

    #[test]
    fn leaf_fragments_static_value() {
        let idx = make_index(&mapping(vec![
            ("driver", scalar("postgres")),
        ]));
        let leaves = idx.traverse("driver");
        let frags = idx.leaf_fragments(&leaves[0]);
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0], (false, b"postgres" as &[u8]));
    }

    #[test]
    fn leaf_fragments_null_value_is_empty() {
        let idx = make_index(&mapping(vec![
            ("id", Tree::Null),
        ]));
        let leaves = idx.traverse("id");
        let frags = idx.leaf_fragments(&leaves[0]);
        assert!(frags.is_empty());
    }

    #[test]
    fn leaf_fragments_single_placeholder() {
        let idx = make_index(&mapping(vec![
            ("copy", scalar("${session.user.name}")),
        ]));
        let leaves = idx.traverse("copy");
        let frags = idx.leaf_fragments(&leaves[0]);
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0], (true, b"session.user.name" as &[u8]));
    }

    #[test]
    fn leaf_fragments_template() {
        let idx = make_index(&mapping(vec![
            ("key", scalar("prefix.${some.path}.suffix")),
        ]));
        let leaves = idx.traverse("key");
        let frags = idx.leaf_fragments(&leaves[0]);
        assert_eq!(frags.len(), 3);
        assert_eq!(frags[0], (false, b"prefix." as &[u8]));
        assert_eq!(frags[1], (true,  b"some.path" as &[u8]));
        assert_eq!(frags[2], (false, b".suffix" as &[u8]));
    }

    // --- set_args ---

    #[test]
    fn set_args_store_name() {
        let idx = make_index(&mapping(vec![
            ("session", mapping(vec![
                ("_set", mapping(vec![
                    ("store", scalar("Kvs")),
                    ("key",    scalar("session:1")),
                ])),
                ("user", mapping(vec![
                    ("id", Tree::Null),
                ])),
            ])),
        ]));
        let leaves = idx.traverse("session.user.id");
        let (store, _, _) = idx.set_args(&leaves[0]);
        assert_eq!(store, "Kvs");
    }

    #[test]
    fn set_args_no_set_returns_empty() {
        let idx = make_index(&mapping(vec![
            ("user", mapping(vec![
                ("id", Tree::Null),
            ])),
        ]));
        let leaves = idx.traverse("user.id");
        let (store, _, args) = idx.set_args(&leaves[0]);
        assert!(store.is_empty() && args.is_empty());
    }
}
