use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::ports::provided::Tree;

// ── meta_key keywords ─────────────────────────────────────────────────────────

pub const META_LOAD:  &[u8] = b"_load";
pub const META_STORE: &[u8] = b"_store";
pub const META_STATE: &[u8] = b"_state";

// ── prop keywords (within _load / _store) ────────────────────────────────────

pub const PROP_CLIENT: &[u8] = b"client";
pub const PROP_KEY:    &[u8] = b"key";
pub const PROP_MAP:    &[u8] = b"map";

// ── path field layout (u64) ───────────────────────────────────────────────────
//
// | field       | bits |
// |-------------|------|
// | is_leaf     |    1 | bit 63
// | offset      |   16 | bits 62..47
// | count       |    4 | bits 46..43  is_leaf=0: 子path数, is_leaf=1: unused
// | padding     |   11 | bits 42..32
// | parent_idx  |   16 | bits 31..16  virtual root is self-referential (0)
// | keyword_idx |   16 | bits 15..0   interning_idx of this path's keyword

pub const PATH_IS_LEAF_SHIFT:     u64 = 63;
pub const PATH_OFFSET_SHIFT:      u64 = 47;
pub const PATH_COUNT_SHIFT:       u64 = 43;
pub const PATH_PARENT_IDX_SHIFT:  u64 = 16;
pub const PATH_KEYWORD_IDX_SHIFT: u64 = 0;

pub const PATH_IS_LEAF_MASK:     u64 = 0x1    << PATH_IS_LEAF_SHIFT;
pub const PATH_OFFSET_MASK:      u64 = 0xffff << PATH_OFFSET_SHIFT;
pub const PATH_COUNT_MASK:       u64 = 0xf    << PATH_COUNT_SHIFT;
pub const PATH_PARENT_IDX_MASK:  u64 = 0xffff << PATH_PARENT_IDX_SHIFT;
pub const PATH_KEYWORD_IDX_MASK: u64 = 0xffff; // bits 15..0

// ── Dsl ───────────────────────────────────────────────────────────────────────

pub struct Dsl {
    paths:         Box<[u64]>,
    children:      Box<[u32]>,
    leaves:        Box<[u8]>,
    interning:     Box<[u8]>,
    interning_idx: Box<[u64]>,
}

impl Dsl {
    pub fn new(
        paths:         Box<[u64]>,
        children:      Box<[u32]>,
        leaves:        Box<[u8]>,
        interning:     Box<[u8]>,
        interning_idx: Box<[u64]>,
    ) -> Self {
        Self { paths, children, leaves, interning, interning_idx }
    }

    pub fn compile(tree: &Tree) -> (
        Box<[u64]>,
        Box<[u32]>,
        Box<[u8]>,
        Box<[u8]>,
        Box<[u64]>,
    ) {
        let mut compiler = Compiler::new();
        // paths[0] = virtual root (keyword_idx=0 = empty string)
        compiler.intern(b""); // interning[0] = ""
        compiler.paths.push(0u64); // placeholder, filled after walking top-level
        if let Tree::Mapping(pairs) = tree {
            let children_offset = compiler.children.len() as u32;
            let field_pairs: Vec<_> = pairs.iter()
                .filter(|(k, _)| k.first() != Some(&b'_'))
                .collect();
            let child_count = field_pairs.len() as u32;

            for _ in 0..child_count {
                compiler.children.push(0); // placeholder
            }
            for (i, (k, v)) in field_pairs.iter().enumerate() {
                let child_idx = compiler.paths.len() as u32;
                compiler.children[children_offset as usize + i] = child_idx;
                compiler.walk_field_key(k, v, 0, None, None); // parent=virtual root(0)
            }

            let count_bits = (child_count as u64) & 0xf;
            compiler.paths[0] =
                (children_offset as u64) << PATH_OFFSET_SHIFT
                | count_bits             << PATH_COUNT_SHIFT
                | 0u64 << PATH_PARENT_IDX_SHIFT  // self-referential
                | 0u64; // keyword_idx=0 (empty)
        }
        compiler.finish()
    }

    /// Parse YAML source, compile, and write static Rust data to `out_path`.
    #[cfg(feature = "precompile")]
    pub fn write(src: &[u8], out_path: &str) -> Result<(), alloc::string::String> {
        extern crate std;
        use std::string::{String, ToString};
        use std::format;

        let tree = parse_yaml(src)?;
        let (paths, children, leaves, interning, interning_idx) = Self::compile(&tree);

        let mut out = String::new();
        out.push_str("// @generated — do not edit by hand\n\n");
        emit_u64_slice(&mut out, "PATHS",         &paths);
        emit_u32_slice(&mut out, "CHILDREN",      &children);
        emit_u8_slice (&mut out, "LEAVES",        &leaves);
        emit_u8_slice (&mut out, "INTERNING",     &interning);
        emit_u64_slice(&mut out, "INTERNING_IDX", &interning_idx);

        std::fs::write(out_path, out)
            .map_err(|e| format!("write error: {e}"))
    }
}

// ── MetaBlock ─────────────────────────────────────────────────────────────────
//
// Intermediate representation of a resolved _load or _store block.
// Carried down the recursion for inheritance.

#[derive(Clone)]
struct MetaBlock {
    client_idx: u32,              // interning_idx of client keyword
    key_idx:    u32,              // interning_idx of key value
    args:       Vec<(u32, u32)>,  // (key_interning_idx, value_interning_idx)
}

// ── Compiler (internal) ───────────────────────────────────────────────────────

struct Compiler {
    paths:         Vec<u64>,
    children:      Vec<u32>,
    leaves:        Vec<u8>,
    interning:     Vec<u8>,
    interning_idx: Vec<u64>,
}

impl Compiler {
    fn new() -> Self {
        Self {
            paths:         Vec::new(),
            children:      Vec::new(),
            leaves:        Vec::new(),
            interning:     Vec::new(),
            interning_idx: Vec::new(),
        }
    }

    // ── walk ──────────────────────────────────────────────────────────────────

    /// Process a single field_key.
    fn walk_field_key(
        &mut self,
        keyword:    &[u8],
        value:      &Tree,
        parent_idx: u32,
        inh_load:   Option<&MetaBlock>,
        inh_store:  Option<&MetaBlock>,
    ) {
        let path_idx = self.paths.len() as u32;
        self.paths.push(0u64); // placeholder, filled below

        let keyword_idx = self.intern(keyword);

        match value {
            Tree::Mapping(pairs) => {
                // Extract _load / _store from this node, merging with inherited.
                let load  = self.resolve_meta(pairs, META_LOAD,  inh_load);
                let store = self.resolve_meta(pairs, META_STORE, inh_store);

                // Collect child field_keys.
                // Reserve children slots first so they are contiguous, then walk.
                let children_offset = self.children.len() as u32;
                let field_pairs: Vec<_> = pairs.iter()
                    .filter(|(k, _)| k.first() != Some(&b'_'))
                    .collect();
                let child_count = field_pairs.len() as u32;

                // Reserve placeholder slots for each child's path_idx.
                let first_child_path_idx = self.paths.len() as u32;
                for i in 0..child_count {
                    self.children.push(first_child_path_idx + i); // will be correct after walk
                }

                // Now walk each child; paths are pushed in order so indices are sequential.
                for (i, (k, v)) in field_pairs.iter().enumerate() {
                    let child_idx = self.paths.len() as u32;
                    self.children[children_offset as usize + i] = child_idx;
                    self.walk_field_key(k, v, path_idx, load.as_ref(), store.as_ref());
                }

                if child_count == 0 {
                    // No child field_keys → treat as leaf.
                    self.write_leaf(path_idx, keyword_idx, parent_idx, &Tree::Null, load.as_ref(), store.as_ref());
                } else {
                    let count_bits = (child_count as u64) & 0xf;
                    self.paths[path_idx as usize] =
                        (children_offset as u64) << PATH_OFFSET_SHIFT
                        | count_bits              << PATH_COUNT_SHIFT
                        | (parent_idx as u64)     << PATH_PARENT_IDX_SHIFT
                        | (keyword_idx as u64)    & PATH_KEYWORD_IDX_MASK;
                }
            }
            // Scalar or Null → leaf with optional hardcoded value.
            _ => {
                self.write_leaf(path_idx, keyword_idx, parent_idx, value, inh_load, inh_store);
            }
        }
    }

    // ── meta resolution ───────────────────────────────────────────────────────

    /// Resolve a _load or _store block from this node's pairs, merging with inherited.
    /// Returns None if neither this node nor ancestors define the block.
    fn resolve_meta(
        &mut self,
        pairs:    &[(Vec<u8>, Tree)],
        meta_key: &[u8],
        inherited: Option<&MetaBlock>,
    ) -> Option<MetaBlock> {
        let local = pairs.iter().find(|(k, _)| k.as_slice() == meta_key);
        match (local, inherited) {
            (None, None) => None,
            (None, Some(inh)) => Some(inh.clone()),
            (Some((_, Tree::Mapping(meta_pairs))), inh) => {
                // Start from inherited, overwrite with local fields.
                let mut client_idx = inh.map(|b| b.client_idx).unwrap_or(0);
                let mut key_idx    = inh.map(|b| b.key_idx).unwrap_or(0);
                let mut args: Vec<(u32, u32)> = inh.map(|b| b.args.clone()).unwrap_or_default();

                for (k, v) in meta_pairs {
                    if k.as_slice() == PROP_CLIENT {
                        if let Tree::Scalar(b) = v {
                            client_idx = self.intern(b);
                        }
                    } else if k.as_slice() == PROP_KEY {
                        key_idx = if let Tree::Scalar(b) = v { self.intern(b) } else { 0 };
                    } else if k.as_slice() == PROP_MAP {
                        // map entries: each value is a string (store column name etc.)
                        // stored as (dst_path_interning_idx, src_value_interning_idx)
                        if let Tree::Mapping(map_pairs) = v {
                            args.clear(); // local map overrides inherited
                            for (mk, mv) in map_pairs {
                                let mk_idx = self.intern(mk);
                                let mv_idx = if let Tree::Scalar(b) = mv { self.intern(b) } else { 0 };
                                args.push((mk_idx, mv_idx));
                            }
                        }
                    } else if k.as_slice() != META_LOAD
                           && k.as_slice() != META_STORE
                           && k.as_slice() != META_STATE {
                        // arbitrary implementor arg
                        let ak = self.intern(k);
                        let av = if let Tree::Scalar(b) = v { self.intern(b) } else { 0 };
                        // overwrite if key already present, otherwise append
                        if let Some(entry) = args.iter_mut().find(|(ek, _)| *ek == ak) {
                            entry.1 = av;
                        } else {
                            args.push((ak, av));
                        }
                    }
                }
                Some(MetaBlock { client_idx, key_idx, args })
            }
            _ => inherited.cloned(),
        }
    }

    // ── leaf serialization ────────────────────────────────────────────────────

    /// Write leaf data to `leaves` and update `paths[path_idx]`.
    ///
    /// Leaf layout (Architecture.md #データ構造仕様 参照):
    ///   keyword_idx        (u32le)
    ///   value_token_count  (u32le)
    ///   token_type[i]      (u8)     0=static(interning_idx), 1=placeholder(path文字列のinterning_idx)
    ///   token_idx[i]       (u32le)
    ///   ... × value_token_count
    ///   _load  client_idx (u32le) | key_idx (u32le)
    ///   _store client_idx (u32le) | key_idx (u32le)
    ///   _load.args  × load_args_count  : key_idx(u32le) | value_idx(u32le)
    ///   _store.args × store_args_count : key_idx(u32le) | value_idx(u32le)
    fn write_leaf(
        &mut self,
        path_idx:    u32,
        keyword_idx: u32,
        parent_idx:  u32,
        value:       &Tree,
        load:        Option<&MetaBlock>,
        store:       Option<&MetaBlock>,
    ) {
        let leaf_offset = self.leaves.len() as u32;

        let load_args_count  = load.map(|b| b.args.len()).unwrap_or(0);
        let store_args_count = store.map(|b| b.args.len()).unwrap_or(0);

        // keyword
        self.push_u32(keyword_idx);

        // value tokens: tokenize scalar by ${}, Null → 0 tokens
        if let Tree::Scalar(b) = value {
            // split by ${...} into (type, bytes) pairs
            let mut tokens: Vec<(u8, u32)> = Vec::new();
            let mut rest = b.as_slice();
            while !rest.is_empty() {
                if let Some(start) = rest.windows(2).position(|w| w == b"${") {
                    if start > 0 {
                        // static prefix
                        let idx = self.intern(&rest[..start]);
                        tokens.push((0, idx));
                    }
                    rest = &rest[start + 2..];
                    if let Some(end) = rest.iter().position(|&c| c == b'}') {
                        // placeholder path string
                        let idx = self.intern(&rest[..end]);
                        tokens.push((1, idx));
                        rest = &rest[end + 1..];
                    } else {
                        // malformed: treat remainder as static
                        let idx = self.intern(rest);
                        tokens.push((0, idx));
                        break;
                    }
                } else {
                    // no more placeholders
                    let idx = self.intern(rest);
                    tokens.push((0, idx));
                    break;
                }
            }
            self.push_u32(tokens.len() as u32);
            for (t, idx) in tokens {
                self.leaves.push(t);
                self.push_u32(idx);
            }
        } else {
            // Null → 0 tokens
            self.push_u32(0);
        }

        // _load header
        self.push_u32(load.map(|b| b.client_idx).unwrap_or(0));
        self.push_u32(load.map(|b| b.key_idx).unwrap_or(0));

        // _store header
        self.push_u32(store.map(|b| b.client_idx).unwrap_or(0));
        self.push_u32(store.map(|b| b.key_idx).unwrap_or(0));

        // _load.args
        if let Some(b) = load {
            for &(ak, av) in &b.args {
                self.push_u32(ak);
                self.push_u32(av);
            }
        }

        // _store.args
        if let Some(b) = store {
            for &(ak, av) in &b.args {
                self.push_u32(ak);
                self.push_u32(av);
            }
        }

        // Update path entry: is_leaf=1, offset=leaf_offset, count=unused
        self.paths[path_idx as usize] =
            PATH_IS_LEAF_MASK
            | (leaf_offset as u64)  << PATH_OFFSET_SHIFT
            | (parent_idx as u64)   << PATH_PARENT_IDX_SHIFT
            | (keyword_idx as u64)  & PATH_KEYWORD_IDX_MASK;
    }

    // ── interning ─────────────────────────────────────────────────────────────

    /// Intern a byte string, returning its interning_idx index.
    /// Deduplicates: if already interned, returns existing index.
    fn intern(&mut self, s: &[u8]) -> u32 {
        // Linear scan for dedup (DSL strings are small in number).
        for (i, entry) in self.interning_idx.iter().enumerate() {
            let offset = (entry >> 32) as usize;
            let len    = (entry & 0xffff_ffff) as usize;
            if self.interning.get(offset..offset + len) == Some(s) {
                return i as u32;
            }
        }
        let offset = self.interning.len();
        self.interning.extend_from_slice(s);
        let idx = self.interning_idx.len() as u32;
        self.interning_idx.push(((offset as u64) << 32) | s.len() as u64);
        idx
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    fn push_u32(&mut self, v: u32) {
        self.leaves.extend_from_slice(&v.to_le_bytes());
    }

    fn finish(self) -> (Box<[u64]>, Box<[u32]>, Box<[u8]>, Box<[u8]>, Box<[u64]>) {
        (
            self.paths.into_boxed_slice(),
            self.children.into_boxed_slice(),
            self.leaves.into_boxed_slice(),
            self.interning.into_boxed_slice(),
            self.interning_idx.into_boxed_slice(),
        )
    }
}

// ── precompile helpers ────────────────────────────────────────────────────────

#[cfg(feature = "precompile")]
pub fn parse_yaml(src: &[u8]) -> Result<Tree, alloc::string::String> {
    extern crate std;
    use std::string::ToString;
    use std::format;

    let s = std::str::from_utf8(src)
        .map_err(|e| format!("UTF-8 error: {e}"))?;
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(s)
        .map_err(|e| format!("YAML parse error: {e}"))?;
    Ok(yaml_value_to_tree(yaml))
}

#[cfg(feature = "precompile")]
fn yaml_value_to_tree(v: serde_yaml_ng::Value) -> Tree {
    extern crate std;
    use std::string::ToString;

    match v {
        serde_yaml_ng::Value::Mapping(m) => Tree::Mapping(
            m.into_iter()
                .filter_map(|(k, v)| {
                    if let serde_yaml_ng::Value::String(s) = k {
                        Some((s.into_bytes(), yaml_value_to_tree(v)))
                    } else {
                        None
                    }
                })
                .collect(),
        ),
        serde_yaml_ng::Value::Sequence(s) => {
            Tree::Sequence(s.into_iter().map(yaml_value_to_tree).collect())
        }
        serde_yaml_ng::Value::String(s)  => Tree::Scalar(s.into_bytes()),
        serde_yaml_ng::Value::Number(n)  => Tree::Scalar(n.to_string().into_bytes()),
        serde_yaml_ng::Value::Bool(b)    => Tree::Scalar(b.to_string().into_bytes()),
        serde_yaml_ng::Value::Null       => Tree::Null,
        _                                => Tree::Null,
    }
}

#[cfg(feature = "precompile")]
fn emit_u64_slice(out: &mut alloc::string::String, name: &str, data: &[u64]) {
    extern crate std;
    use std::format;
    out.push_str(&format!("pub static {name}: &[u64] = &[\n"));
    for chunk in data.chunks(8) {
        out.push_str("    ");
        for v in chunk { out.push_str(&format!("0x{v:016x}, ")); }
        out.push('\n');
    }
    out.push_str("];\n\n");
}

#[cfg(feature = "precompile")]
fn emit_u32_slice(out: &mut alloc::string::String, name: &str, data: &[u32]) {
    extern crate std;
    use std::format;
    out.push_str(&format!("pub static {name}: &[u32] = &[\n"));
    for chunk in data.chunks(8) {
        out.push_str("    ");
        for v in chunk { out.push_str(&format!("0x{v:08x}, ")); }
        out.push('\n');
    }
    out.push_str("];\n\n");
}

#[cfg(feature = "precompile")]
fn emit_u8_slice(out: &mut alloc::string::String, name: &str, data: &[u8]) {
    extern crate std;
    use std::format;
    out.push_str(&format!("pub static {name}: &[u8] = &[\n"));
    for chunk in data.chunks(16) {
        out.push_str("    ");
        for v in chunk { out.push_str(&format!("0x{v:02x}, ")); }
        out.push('\n');
    }
    out.push_str("];\n\n");
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn scalar(s: &str) -> Tree { Tree::Scalar(s.as_bytes().to_vec()) }
    fn mapping(pairs: Vec<(&str, Tree)>) -> Tree {
        Tree::Mapping(pairs.into_iter().map(|(k, v)| (k.as_bytes().to_vec(), v)).collect())
    }

    fn compile(tree: &Tree) -> (Vec<u64>, Vec<u32>, Vec<u8>, Vec<u8>, Vec<u64>) {
        let (p, c, l, i, ii) = Dsl::compile(tree);
        (p.into_vec(), c.into_vec(), l.into_vec(), i.into_vec(), ii.into_vec())
    }

    // --- single_leaf ---

    #[test]
    fn single_leaf() {
        let (paths, ..) = compile(&mapping(vec![
            ("name", Tree::Null),
        ]));
        assert_eq!(paths.len(), 2);                       // root(0) + name(1)
        assert!(paths[0] & PATH_IS_LEAF_MASK == 0);       // root is not a leaf
        assert!(paths[1] & PATH_IS_LEAF_MASK != 0);       // name is a leaf
    }

    // --- nested ---

    #[test]
    fn nested() {
        let (paths, children, ..) = compile(&mapping(vec![
            ("user", mapping(vec![
                ("id",   Tree::Null),
                ("name", Tree::Null),
            ])),
        ]));
        assert_eq!(paths.len(), 4);                       // root(0) + user(1) + id(2) + name(3)
        assert!(paths[1] & PATH_IS_LEAF_MASK == 0);       // user is not a leaf
        assert_eq!(children.len(), 3);                    // root→user(1) + user→id,name(2)
    }

    // --- meta_key ---

    #[test]
    fn meta_key_excluded_from_paths() {
        let (paths, ..) = compile(&mapping(vec![
            ("user", mapping(vec![
                ("_load", mapping(vec![
                    ("client", scalar("Memory")),
                    ("key",    scalar("user:1")),
                ])),
                ("id", Tree::Null),
            ])),
        ]));
        // root(0) + user(1) + id(2) — _load must not appear
        assert_eq!(paths.len(), 3);
    }

    // --- load in leaf ---

    #[test]
    fn load_client_stored_in_leaf() {
        let (paths, _, leaves, interning, interning_idx) = compile(&mapping(vec![
            ("user", mapping(vec![
                ("_load", mapping(vec![
                    ("client", scalar("Memory")),
                    ("key",    scalar("user:1")),
                ])),
                ("id", Tree::Null),
            ])),
        ]));
        // root(0), user(1), id(2)
        // leaf: keyword(4) + token_count(4) + tokens(0×5) + load_client(4)
        let leaf_offset = ((paths[2] & PATH_OFFSET_MASK) >> PATH_OFFSET_SHIFT) as usize;
        let token_count = u32::from_le_bytes(leaves[leaf_offset+4..leaf_offset+8].try_into().unwrap()) as usize;
        let meta_base = leaf_offset + 8 + token_count * 5;
        let client_idx = u32::from_le_bytes(leaves[meta_base..meta_base+4].try_into().unwrap()) as usize;
        let off = (interning_idx[client_idx] >> 32) as usize;
        let len = (interning_idx[client_idx] & 0xffff_ffff) as usize;
        assert_eq!(&interning[off..off+len], b"Memory");
    }

    // --- store inheritance ---

    #[test]
    fn store_inherited_to_child_leaf() {
        let (paths, _, leaves, interning, interning_idx) = compile(&mapping(vec![
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
        // root(0), session(1), user(2), id(3)
        // leaf: keyword(4) + token_count(4) + tokens(0×5) + load_client(4) + load_key(4) + store_client(4)
        let leaf_offset = ((paths[3] & PATH_OFFSET_MASK) >> PATH_OFFSET_SHIFT) as usize;
        let token_count = u32::from_le_bytes(leaves[leaf_offset+4..leaf_offset+8].try_into().unwrap()) as usize;
        let meta_base = leaf_offset + 8 + token_count * 5;
        let client_idx = u32::from_le_bytes(leaves[meta_base+8..meta_base+12].try_into().unwrap()) as usize;
        let off = (interning_idx[client_idx] >> 32) as usize;
        let len = (interning_idx[client_idx] & 0xffff_ffff) as usize;
        assert_eq!(&interning[off..off+len], b"Kvs");
    }

    // --- intern ---

    #[test]
    fn intern_dedup() {
        let (_, _, _, interning, interning_idx) = compile(&mapping(vec![
            ("a", scalar("hello")),
            ("b", scalar("hello")),
        ]));
        let hello_count = (0..interning_idx.len()).filter(|&i| {
            let off = (interning_idx[i] >> 32) as usize;
            let len = (interning_idx[i] & 0xffff_ffff) as usize;
            interning.get(off..off+len) == Some(b"hello" as &[u8])
        }).count();
        assert_eq!(hello_count, 1);
    }

    // --- precompile ---

    #[cfg(feature = "precompile")]
    #[test]
    fn write_tenant_yml() {
        extern crate std;
        let src = std::include_bytes!("../examples/tenant.yml");
        let out = std::env::temp_dir().join("tenant_compiled.rs");
        std::fs::remove_file(&out).ok();
        Dsl::write(src, out.to_str().unwrap()).expect("write failed");
        let content = std::fs::read_to_string(&out).expect("output not written");
        assert!(content.contains("pub static PATHS:"));
    }
}
