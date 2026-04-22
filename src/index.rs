use alloc::{
    collections::BTreeMap,
    string::String,
    vec::Vec,
};
use core::str::from_utf8;

use crate::dsl::{
    PATH_IS_LEAF_MASK,
    PATH_KEYWORD_ID_SHIFT, PATH_KEYWORD_ID_MASK,
    PATH_PARENT_ID_SHIFT,  PATH_PARENT_ID_MASK,
    PATH_CHILD_ID_SHIFT,   PATH_CHILD_ID_MASK,
    PATH_VALUE_ID_MASK,
    VALUE_IS_PLACEHOLDER_MASK, VALUE_WORD_ID_MASK,
    LEAF_WIDTH,
    LEAF_GET_STORE_ID,    LEAF_GET_KEY_ID,
    LEAF_SET_STORE_ID,    LEAF_SET_KEY_ID,
    LEAF_GET_MAP_KEY_ID,  LEAF_GET_MAP_VAL_ID,
    LEAF_GET_ARGS_KEY_ID, LEAF_GET_ARGS_VAL_ID,
    LEAF_SET_MAP_KEY_ID,  LEAF_SET_MAP_VAL_ID,
    LEAF_SET_ARGS_KEY_ID, LEAF_SET_ARGS_VAL_ID,
};
use crate::list::{List, VariableList};
use crate::provided::Tree;
use crate::debug_log;

// ── LeafRef ───────────────────────────────────────────────────────────────────

pub struct LeafRef {
    pub path_id: u16,
    pub leaf_id:  u16,
    pub value_id: u16,
}

// ── Index ─────────────────────────────────────────────────────────────────────

pub struct Index {
    paths:     List<u64>,
    children:  VariableList<u16>,
    leaves:    VariableList<u16>,
    values:    VariableList<u16>,
    words:     VariableList<u8>,
    map_keys:  VariableList<u16>,
    map_vals:  VariableList<u16>,
    args_keys: VariableList<u16>,
    args_vals: VariableList<u16>,
}

impl Index {
    pub fn new(
        paths:     List<u64>,
        children:  VariableList<u16>,
        leaves:    VariableList<u16>,
        values:    VariableList<u16>,
        words:     VariableList<u8>,
        map_keys:  VariableList<u16>,
        map_vals:  VariableList<u16>,
        args_keys: VariableList<u16>,
        args_vals: VariableList<u16>,
    ) -> Self {
        Self { paths, children, leaves, values, words, map_keys, map_vals, args_keys, args_vals }
    }

    /// Traverse to the path node matching `path` (dot-separated keywords),
    /// then collect all leaf descendants into a flat list.
    ///
    /// ```
    /// # extern crate alloc;
    /// use context_engine::{Tree, Index, dsl::Dsl};
    /// let tree = Tree::Mapping(alloc::vec![(b"id".to_vec(), Tree::Null)]);
    /// let (paths, children, leaves, values, words, map_keys, map_vals, args_keys, args_vals) = Dsl::compile(&tree, &[]).unwrap();
    /// let index = Index::new(paths, children, leaves, values, words, map_keys, map_vals, args_keys, args_vals);
    /// assert_eq!(index.traverse("id").len(), 1);
    /// assert!(index.traverse("missing").is_empty());
    /// ```
    pub fn traverse(&self, path: &str) -> alloc::boxed::Box<[LeafRef]> {
        let mut result = Vec::new();
        let Some(path_id) = self.find(path) else {
            debug_log!("Index", "traverse", path, "-> not found");
            return result.into_boxed_slice();
        };
        self.collect_leaves(path_id, &mut result);
        debug_log!("Index", "traverse", path, &alloc::format!("-> {} leaf(s)", result.len()));
        result.into_boxed_slice()
    }

    /// Return the keyword bytes for a path node.
    ///
    /// ```
    /// # extern crate alloc;
    /// use context_engine::{Tree, Index, dsl::Dsl};
    /// let tree = Tree::Mapping(alloc::vec![(b"name".to_vec(), Tree::Null)]);
    /// let (paths, children, leaves, values, words, map_keys, map_vals, args_keys, args_vals) = Dsl::compile(&tree, &[]).unwrap();
    /// let index = Index::new(paths, children, leaves, values, words, map_keys, map_vals, args_keys, args_vals);
    /// assert_eq!(index.keyword_of(1), b"name");
    /// ```
    pub fn keyword_of(&self, path_id: u16) -> &[u8] {
        let path = self.paths.data[path_id as usize];
        let word_id = ((path & PATH_KEYWORD_ID_MASK) >> PATH_KEYWORD_ID_SHIFT) as usize;
        let kw = self.word_bytes(word_id);
        debug_log!("Index", "keyword_of", &alloc::format!("path_id={path_id}"), &alloc::format!("-> {:?}", core::str::from_utf8(kw).unwrap_or("?")));
        kw
    }

    /// Extract `_get` store_id and args for the given leaf.
    /// Returns (0, empty) if no `_get` is configured.
    pub fn get_args(&self, leaf: &LeafRef) -> (u8, BTreeMap<String, Tree>) {
        let r = self.decode_meta(leaf, MetaKind::Get);
        debug_log!("Index", "get_args", &alloc::format!("path_id={}", leaf.path_id), &alloc::format!("-> store_id={}", r.0));
        r
    }

    /// Extract `_set` store_id and args for the given leaf.
    /// Returns (0, empty) if no `_set` is configured.
    pub fn set_args(&self, leaf: &LeafRef) -> (u8, BTreeMap<String, Tree>) {
        let r = self.decode_meta(leaf, MetaKind::Set);
        debug_log!("Index", "set_args", &alloc::format!("path_id={}", leaf.path_id), &alloc::format!("-> store_id={}", r.0));
        r
    }

    /// Return the value fragments encoded in the leaf.
    ///
    /// Each element is `(is_placeholder, bytes)`:
    /// - `false` — static string literal
    /// - `true`  — `${path}` reference whose bytes are the path string
    ///
    /// An empty slice means the leaf has no value (`Null`).
    ///
    /// ```
    /// # extern crate alloc;
    /// use context_engine::{Tree, Index, dsl::Dsl};
    /// let tree = Tree::Mapping(alloc::vec![
    ///     (b"driver".to_vec(), Tree::Scalar(b"postgres".to_vec())),
    /// ]);
    /// let (paths, children, leaves, values, words, map_keys, map_vals, args_keys, args_vals) = Dsl::compile(&tree, &[]).unwrap();
    /// let index = Index::new(paths, children, leaves, values, words, map_keys, map_vals, args_keys, args_vals);
    /// let leaves = index.traverse("driver");
    /// let frags = index.leaf_fragments(&leaves[0]);
    /// assert_eq!(frags.len(), 1);
    /// assert_eq!(frags[0].0, false);
    /// assert_eq!(frags[0].1, b"postgres");
    /// ```
    pub fn leaf_fragments(&self, leaf: &LeafRef) -> Vec<(bool, &[u8])> {
        let value_id = leaf.value_id as usize;
        if value_id == 0 {
            debug_log!("Index", "leaf_fragments", &alloc::format!("path_id={}", leaf.path_id), "-> null");
            return Vec::new();
        }
        let Some(frags) = self.values_slice(value_id) else { return Vec::new(); };
        let result: Vec<(bool, &[u8])> = frags.iter().map(|&f| {
            let is_ph  = (f & VALUE_IS_PLACEHOLDER_MASK) != 0;
            let word_id = (f & VALUE_WORD_ID_MASK) as usize;
            (is_ph, self.word_bytes(word_id))
        }).collect();
        debug_log!("Index", "leaf_fragments", &alloc::format!("path_id={}", leaf.path_id), &alloc::format!("-> {} frag(s)", result.len()));
        result
    }
}

// ── private ───────────────────────────────────────────────────────────────────

enum MetaKind { Get, Set }

impl Index {
    fn find(&self, path: &str) -> Option<u16> {
        let mut current: u16 = 0;
        for segment in path.split('.') {
            current = self.find_child(current, segment.as_bytes())?;
        }
        Some(current)
    }

    fn find_child(&self, path_id: u16, keyword: &[u8]) -> Option<u16> {
        let path = self.paths.data[path_id as usize];
        if path & PATH_IS_LEAF_MASK != 0 { return None; }
        let children_id = ((path & PATH_CHILD_ID_MASK) >> PATH_CHILD_ID_SHIFT) as usize;
        if children_id == 0 { return None; }
        let Some(children) = self.children_slice(children_id) else { return None; };
        children.iter().copied().find(|&child_id| self.keyword_of(child_id) == keyword)
    }

    fn collect_leaves(&self, path_id: u16, out: &mut Vec<LeafRef>) {
        let path = self.paths.data[path_id as usize];
        if path & PATH_IS_LEAF_MASK != 0 {
            let leaf_id  = ((path & PATH_CHILD_ID_MASK) >> PATH_CHILD_ID_SHIFT) as u16;
            let value_id = (path & PATH_VALUE_ID_MASK) as u16;
            out.push(LeafRef { path_id, leaf_id, value_id });
            return;
        }
        let children_id = ((path & PATH_CHILD_ID_MASK) >> PATH_CHILD_ID_SHIFT) as usize;
        if children_id == 0 { return; }
        let children: Vec<u16> = match self.children_slice(children_id) {
            Some(s) => s.to_vec(),
            None    => return,
        };
        for child_id in children {
            self.collect_leaves(child_id, out);
        }
    }

    fn decode_meta(&self, leaf: &LeafRef, kind: MetaKind) -> (u8, BTreeMap<String, Tree>) {
        let leaf_id = leaf.leaf_id as usize;
        if leaf_id == 0 { return (0, BTreeMap::new()); }
        let Some(leaf_data) = self.leaves_slice(leaf_id) else { return (0, BTreeMap::new()); };
        if leaf_data.len() < LEAF_WIDTH { return (0, BTreeMap::new()); }

        let (store_id_raw, key_id, map_key_id, map_val_id, args_key_id, args_val_id) = match kind {
            MetaKind::Get => (
                leaf_data[LEAF_GET_STORE_ID],
                leaf_data[LEAF_GET_KEY_ID],
                leaf_data[LEAF_GET_MAP_KEY_ID],
                leaf_data[LEAF_GET_MAP_VAL_ID],
                leaf_data[LEAF_GET_ARGS_KEY_ID],
                leaf_data[LEAF_GET_ARGS_VAL_ID],
            ),
            MetaKind::Set => (
                leaf_data[LEAF_SET_STORE_ID],
                leaf_data[LEAF_SET_KEY_ID],
                leaf_data[LEAF_SET_MAP_KEY_ID],
                leaf_data[LEAF_SET_MAP_VAL_ID],
                leaf_data[LEAF_SET_ARGS_KEY_ID],
                leaf_data[LEAF_SET_ARGS_VAL_ID],
            ),
        };

        let store_id = store_id_raw as u8;
        if store_id == 0 { return (0, BTreeMap::new()); }

        let mut args: BTreeMap<String, Tree> = BTreeMap::new();

        // key
        if key_id != 0 {
            if let Some(frags) = self.values_slice(key_id as usize) {
                let key_str = self.resolve_static_frags(frags);
                if !key_str.is_empty() {
                    args.insert(String::from("key"), Tree::Scalar(key_str.into_bytes()));
                }
            }
        }

        // map entries: parallel map_keys / map_vals lists
        if map_key_id != 0 && map_val_id != 0 {
            if let (Some(dsts), Some(srcs)) = (
                self.map_keys_slice(map_key_id as usize),
                self.map_vals_slice(map_val_id as usize),
            ) {
                let mut map_pairs: Vec<(Vec<u8>, Tree)> = Vec::new();
                for (&dst_word_id, &src_word_id) in dsts.iter().zip(srcs.iter()) {
                    let dst = self.word_bytes(dst_word_id as usize).to_vec();
                    let src = self.word_bytes(src_word_id as usize).to_vec();
                    map_pairs.push((dst, Tree::Scalar(src)));
                }
                if !map_pairs.is_empty() {
                    args.insert(String::from("map"), Tree::Mapping(map_pairs));
                }
            }
        }

        // scalar args
        if args_key_id != 0 && args_val_id != 0 {
            if let (Some(keys), Some(vals)) = (
                self.args_keys_slice(args_key_id as usize),
                self.args_vals_slice(args_val_id as usize),
            ) {
                for (&key_word_id, &val_values_id) in keys.iter().zip(vals.iter()) {
                    let k = from_utf8(self.word_bytes(key_word_id as usize)).unwrap_or("");
                    if k.is_empty() { continue; }
                    let v = if val_values_id == 0 {
                        Tree::Null
                    } else if let Some(frags) = self.values_slice(val_values_id as usize) {
                        Tree::Scalar(self.resolve_static_frags(frags).into_bytes())
                    } else {
                        Tree::Null
                    };
                    args.insert(String::from(k), v);
                }
            }
        }

        (store_id, args)
    }

    /// Resolve a fragment list that is expected to be purely static (no placeholders at index time).
    fn resolve_static_frags(&self, frags: &[u16]) -> String {
        let mut buf = String::new();
        for &f in frags {
            let word_id = (f & VALUE_WORD_ID_MASK) as usize;
            buf.push_str(from_utf8(self.word_bytes(word_id)).unwrap_or(""));
        }
        buf
    }

    // ── slice helpers ─────────────────────────────────────────────────────────

    fn children_slice(&self, id: usize) -> Option<&[u16]> {
        self.vl_slice_u16(&self.children, id)
    }

    fn leaves_slice(&self, id: usize) -> Option<&[u16]> {
        self.vl_slice_u16(&self.leaves, id)
    }

    fn values_slice(&self, id: usize) -> Option<&[u16]> {
        self.vl_slice_u16(&self.values, id)
    }

    fn map_keys_slice(&self, id: usize) -> Option<&[u16]> {
        self.vl_slice_u16(&self.map_keys, id)
    }

    fn map_vals_slice(&self, id: usize) -> Option<&[u16]> {
        self.vl_slice_u16(&self.map_vals, id)
    }

    fn args_keys_slice(&self, id: usize) -> Option<&[u16]> {
        self.vl_slice_u16(&self.args_keys, id)
    }

    fn args_vals_slice(&self, id: usize) -> Option<&[u16]> {
        self.vl_slice_u16(&self.args_vals, id)
    }

    fn vl_slice_u16<'a>(&'a self, vl: &'a VariableList<u16>, id: usize) -> Option<&'a [u16]> {
        let identity_start = id * 2;
        let start = *vl.identity.get(identity_start)?;
        let end   = *vl.identity.get(identity_start + 1)?;
        vl.data.get(start..end)
    }

    fn word_bytes(&self, id: usize) -> &[u8] {
        let identity_start = id * 2;
        let start = match self.words.identity.get(identity_start) {
            Some(&s) => s,
            None     => return b"",
        };
        let end = match self.words.identity.get(identity_start + 1) {
            Some(&e) => e,
            None     => return b"",
        };
        self.words.data.get(start..end).unwrap_or(b"")
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
        let (paths, children, leaves, values, words, map_keys, map_vals, args_keys, args_vals)
            = Dsl::compile(tree, &[]).unwrap();
        Index::new(paths, children, leaves, values, words, map_keys, map_vals, args_keys, args_vals)
    }

    fn make_index_with_stores<'a>(tree: &Tree, store_ids: &[&'a str]) -> Index {
        let (paths, children, leaves, values, words, map_keys, map_vals, args_keys, args_vals)
            = Dsl::compile(tree, store_ids).unwrap();
        Index::new(paths, children, leaves, values, words, map_keys, map_vals, args_keys, args_vals)
    }

    // --- traverse ---

    #[test]
    fn traverse_leaf_path() {
        let index = make_index(&mapping(vec![
            ("session", mapping(vec![
                ("user", mapping(vec![
                    ("id", Tree::Null),
                ])),
            ])),
        ]));
        let leaves = index.traverse("session.user.id");
        assert_eq!(leaves.len(), 1);
    }

    #[test]
    fn traverse_intermediate_collects_all_leaves() {
        let index = make_index(&mapping(vec![
            ("session", mapping(vec![
                ("user", mapping(vec![
                    ("id",   Tree::Null),
                    ("name", Tree::Null),
                ])),
            ])),
        ]));
        let leaves = index.traverse("session.user");
        assert_eq!(leaves.len(), 2);
    }

    #[test]
    fn traverse_nonexistent_returns_empty() {
        let index = make_index(&mapping(vec![
            ("session", mapping(vec![
                ("user", mapping(vec![
                    ("id", Tree::Null),
                ])),
            ])),
        ]));
        let leaves = index.traverse("session.user.missing");
        assert!(leaves.is_empty());
    }

    // --- keyword_of ---

    #[test]
    fn keyword_of_leaf() {
        let index = make_index(&mapping(vec![
            ("user", mapping(vec![
                ("id", Tree::Null),
            ])),
        ]));
        assert_eq!(index.keyword_of(2), b"id");
    }

    #[test]
    fn keyword_of_intermediate() {
        let index = make_index(&mapping(vec![
            ("user", mapping(vec![
                ("id", Tree::Null),
            ])),
        ]));
        assert_eq!(index.keyword_of(1), b"user");
    }

    // --- get_args ---

    #[test]
    fn get_args_store_id() {
        let index = make_index_with_stores(&mapping(vec![
            ("session", mapping(vec![
                ("_get", mapping(vec![
                    ("store", scalar("Memory")),
                    ("key",    scalar("session:1")),
                ])),
                ("user", mapping(vec![
                    ("id", Tree::Null),
                ])),
            ])),
        ]), &["Memory"]);
        let leaves = index.traverse("session.user.id");
        let (store_id, _) = index.get_args(&leaves[0]);
        assert_eq!(store_id, 1); // "Memory" is store_ids[0] → id=1
    }

    #[test]
    fn get_args_key() {
        let index = make_index_with_stores(&mapping(vec![
            ("session", mapping(vec![
                ("_get", mapping(vec![
                    ("store", scalar("Memory")),
                    ("key",    scalar("session:1")),
                ])),
                ("user", mapping(vec![
                    ("id", Tree::Null),
                ])),
            ])),
        ]), &["Memory"]);
        let leaves = index.traverse("session.user.id");
        let (_, args) = index.get_args(&leaves[0]);
        assert_eq!(args.get("key"), Some(&Tree::Scalar(b"session:1".to_vec())));
    }

    #[test]
    fn get_args_no_get_returns_empty() {
        let index = make_index(&mapping(vec![
            ("user", mapping(vec![
                ("id", Tree::Null),
            ])),
        ]));
        let leaves = index.traverse("user.id");
        let (store_id, args) = index.get_args(&leaves[0]);
        assert_eq!(store_id, 0);
        assert!(args.is_empty());
    }

    // --- leaf_fragments ---

    #[test]
    fn leaf_fragments_static_value() {
        let index = make_index(&mapping(vec![
            ("driver", scalar("postgres")),
        ]));
        let leaves = index.traverse("driver");
        let frags = index.leaf_fragments(&leaves[0]);
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0], (false, b"postgres" as &[u8]));
    }

    #[test]
    fn leaf_fragments_null_value_is_empty() {
        let index = make_index(&mapping(vec![
            ("id", Tree::Null),
        ]));
        let leaves = index.traverse("id");
        let frags = index.leaf_fragments(&leaves[0]);
        assert!(frags.is_empty());
    }

    #[test]
    fn leaf_fragments_single_placeholder() {
        let index = make_index(&mapping(vec![
            ("copy", scalar("${session.user.name}")),
        ]));
        let leaves = index.traverse("copy");
        let frags = index.leaf_fragments(&leaves[0]);
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0], (true, b"session.user.name" as &[u8]));
    }

    #[test]
    fn leaf_fragments_template() {
        let index = make_index(&mapping(vec![
            ("key", scalar("prefix.${some.path}.suffix")),
        ]));
        let leaves = index.traverse("key");
        let frags = index.leaf_fragments(&leaves[0]);
        assert_eq!(frags.len(), 3);
        assert_eq!(frags[0], (false, b"prefix." as &[u8]));
        assert_eq!(frags[1], (true,  b"some.path" as &[u8]));
        assert_eq!(frags[2], (false, b".suffix" as &[u8]));
    }

    // --- set_args ---

    #[test]
    fn set_args_store_id() {
        let index = make_index_with_stores(&mapping(vec![
            ("session", mapping(vec![
                ("_set", mapping(vec![
                    ("store", scalar("Kvs")),
                    ("key",    scalar("session:1")),
                ])),
                ("user", mapping(vec![
                    ("id", Tree::Null),
                ])),
            ])),
        ]), &["Memory", "Kvs"]);
        let leaves = index.traverse("session.user.id");
        let (store_id, _) = index.set_args(&leaves[0]);
        assert_eq!(store_id, 2); // "Kvs" is store_ids[1] → id=2
    }

    #[test]
    fn set_args_no_set_returns_empty() {
        let index = make_index(&mapping(vec![
            ("user", mapping(vec![
                ("id", Tree::Null),
            ])),
        ]));
        let leaves = index.traverse("user.id");
        let (store_id, args) = index.set_args(&leaves[0]);
        assert_eq!(store_id, 0);
        assert!(args.is_empty());
    }
}
