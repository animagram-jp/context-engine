use alloc::{boxed::Box, vec::Vec};
use crate::provided::{DslError, Tree};
use crate::list::{List, VariableList};

// ── meta_key keywords ─────────────────────────────────────────────────────────

pub const META_GET:   &[u8] = b"_get";
pub const META_SET:   &[u8] = b"_set";
pub const META_STATE: &[u8] = b"_state";

// ── prop keywords (within _get / _set) ───────────────────────────────────────

pub const PROP_STORE: &[u8] = b"store";
pub const PROP_KEY:   &[u8] = b"key";
pub const PROP_MAP:   &[u8] = b"map";

// ── path field layout (u64) ───────────────────────────────────────────────────
//
// | field       | bits  |
// |-------------|-------|
// | is_leaf     |     1 | bit 63
// | keyword_id  |    15 | bits 62..48
// | parent_id   |    16 | bits 47..32
// | child/leaf_id |  16 | bits 31..16  is_leaf=0: children id / is_leaf=1: leaves id
// | value_id    |    16 | bits 15..0

pub const PATH_IS_LEAF_SHIFT:    u64 = 63;
pub const PATH_KEYWORD_ID_SHIFT: u64 = 48;
pub const PATH_PARENT_ID_SHIFT:  u64 = 32;
pub const PATH_CHILD_ID_SHIFT:   u64 = 16;
pub const PATH_VALUE_ID_SHIFT:   u64 = 0;

pub const PATH_IS_LEAF_MASK:    u64 = 0x1     << PATH_IS_LEAF_SHIFT;
pub const PATH_KEYWORD_ID_MASK: u64 = 0x7fff  << PATH_KEYWORD_ID_SHIFT;
pub const PATH_PARENT_ID_MASK:  u64 = 0xffff  << PATH_PARENT_ID_SHIFT;
pub const PATH_CHILD_ID_MASK:   u64 = 0xffff  << PATH_CHILD_ID_SHIFT;
pub const PATH_VALUE_ID_MASK:   u64 = 0xffff;

// ── leaf layout (VariableList<u16>, fixed 12 u16s) ───────────────────────────
//
// | Field           | u16 | Note                           |
// |-----------------|-----|--------------------------------|
// | get_store_id    |   0 | stores id (u8 as u16)          |
// | get_key_id      |   1 | values id                      |
// | set_store_id    |   2 | stores id (u8 as u16)          |
// | set_key_id      |   3 | values id                      |
// | get_map_key_id  |   4 | map_keys id                    |
// | get_map_val_id  |   5 | map_vals id                    |
// | get_args_key_id |   6 | args_keys id                   |
// | get_args_val_id |   7 | args_vals id                   |
// | set_map_key_id  |   8 | map_keys id                    |
// | set_map_val_id  |   9 | map_vals id                    |
// | set_args_key_id |  10 | args_keys id                   |
// | set_args_val_id |  11 | args_vals id                   |

pub const LEAF_WIDTH: usize = 12;

pub const LEAF_GET_STORE_ID:    usize = 0;
pub const LEAF_GET_KEY_ID:      usize = 1;
pub const LEAF_SET_STORE_ID:    usize = 2;
pub const LEAF_SET_KEY_ID:      usize = 3;
pub const LEAF_GET_MAP_KEY_ID:  usize = 4;
pub const LEAF_GET_MAP_VAL_ID:  usize = 5;
pub const LEAF_GET_ARGS_KEY_ID: usize = 6;
pub const LEAF_GET_ARGS_VAL_ID: usize = 7;
pub const LEAF_SET_MAP_KEY_ID:  usize = 8;
pub const LEAF_SET_MAP_VAL_ID:  usize = 9;
pub const LEAF_SET_ARGS_KEY_ID: usize = 10;
pub const LEAF_SET_ARGS_VAL_ID: usize = 11;

// ── value fragment encoding (u16) ────────────────────────────────────────────
//
// bit15: is_placeholder, bits14..0: word_id

pub const VALUE_IS_PLACEHOLDER_MASK: u16 = 0x8000;
pub const VALUE_WORD_ID_MASK:        u16 = 0x7fff;

// ── Dsl ───────────────────────────────────────────────────────────────────────

pub struct Dsl;

// ── Limitations ───────────────────────────────────────────────────────────────

const MAX_PATH_ID:  usize = 0xffff;   // 16 bit
const MAX_WORD_ID:  usize = 0x7fff;   // 15 bit (bit15 reserved for is_placeholder)
const MAX_STORE_ID: usize = 0xff;     // 8 bit
// identity arrays are emitted as u16 (offset into data); data arrays are indexed by these offsets
const MAX_VL_U16_DATA: usize = 0xffff; // VariableList<u16> data length fits u16 offset
const MAX_VL_U8_DATA:  usize = 0xffff; // VariableList<u8>  data length fits u16 offset (words identity emitted as u16 too)

impl Dsl {
    /// Compiles a `Tree` into static index data structures consumed by `Index`.
    ///
    /// `store_ids` is the ordered list of store identifier strings as defined by the caller.
    /// The index position becomes the `store_id` (u8) baked into leaf data.
    ///
    /// Returns `Err(DslError::LimitExceeded)` if any compile-time limit is exceeded.
    ///
    /// ```
    /// # extern crate alloc;
    /// use context_engine::{Tree, dsl::Dsl};
    /// let tree = Tree::Mapping(alloc::vec![
    ///     (b"id".to_vec(), Tree::Null),
    /// ]);
    /// let (paths, ..) = Dsl::compile(&tree, &[]).unwrap();
    /// assert_eq!(paths.data.len(), 2); // virtual root + id
    /// ```
    pub fn compile(tree: &Tree, store_ids: &[&str]) -> Result<(
        List<u64>,
        VariableList<u16>,
        VariableList<u16>,
        VariableList<u16>,
        VariableList<u8>,
        VariableList<u16>,
        VariableList<u16>,
        VariableList<u16>,
        VariableList<u16>,
    ), DslError> {
        if store_ids.len() > MAX_STORE_ID {
            return Err(DslError::LimitExceeded(alloc::format!(
                "store_ids length {} exceeds max {}", store_ids.len(), MAX_STORE_ID
            )));
        }
        let mut compiler = Compiler::new(store_ids);
        compiler.intern_word(b"")?; // word_id=0: empty string sentinel
        // paths[0] = virtual root; List::new(1) already reserves the slot

        if let Tree::Mapping(pairs) = tree {
            let field_pairs: Vec<_> = pairs.iter()
                .filter(|(k, _)| k.first() != Some(&b'_'))
                .collect();

            let children_id = compiler.alloc_children_slots(field_pairs.len())?;

            for (i, (k, v)) in field_pairs.iter().enumerate() {
                let child_idx = compiler.paths.data.len() as u16;
                compiler.set_child(children_id, i, child_idx);
                compiler.walk_field_key(k, v, 0, None, None)?;
            }

            let root = (children_id as u64) << PATH_CHILD_ID_SHIFT;
            compiler.paths.data[0] = root;
        }

        compiler.finish()
    }

    /// Parse YAML source, compile, and write static Rust data to `out_path`.
    #[cfg(feature = "precompile")]
    pub fn write(src: &[u8], store_ids: &[&str], out_path: &str) -> Result<(), alloc::string::String> {
        extern crate std;
        use std::string::{String, ToString};
        use std::format;

        let tree = parse_yaml(src)?;
        let (paths, children, leaves, values, words, map_keys, map_vals, args_keys, args_vals)
            = Self::compile(&tree, store_ids).map_err(|e| e.to_string())?;

        // identity arrays are emitted as u16; verify offsets fit before casting
        check_vl_u16_identity(&children,  "children")?;
        check_vl_u16_identity(&leaves,    "leaves")?;
        check_vl_u16_identity(&values,    "values")?;
        check_vl_u16_identity_u8(&words,  "words")?;
        check_vl_u16_identity(&map_keys,  "map_keys")?;
        check_vl_u16_identity(&map_vals,  "map_vals")?;
        check_vl_u16_identity(&args_keys, "args_keys")?;
        check_vl_u16_identity(&args_vals, "args_vals")?;

        let mut out = String::new();
        out.push_str("// @generated — do not edit by hand\n\n");
        emit_u64_slice(&mut out, "PATHS",              &paths.data);
        emit_u16_slice(&mut out, "CHILDREN_IDENTITY",  &children.identity.iter().map(|&x| x as u16).collect::<alloc::vec::Vec<_>>());
        emit_u16_slice(&mut out, "CHILDREN_DATA",      &children.data);
        emit_u16_slice(&mut out, "LEAVES_IDENTITY",    &leaves.identity.iter().map(|&x| x as u16).collect::<alloc::vec::Vec<_>>());
        emit_u16_slice(&mut out, "LEAVES_DATA",        &leaves.data);
        emit_u16_slice(&mut out, "VALUES_IDENTITY",    &values.identity.iter().map(|&x| x as u16).collect::<alloc::vec::Vec<_>>());
        emit_u16_slice(&mut out, "VALUES_DATA",        &values.data);
        emit_u16_slice(&mut out, "WORDS_IDENTITY",     &words.identity.iter().map(|&x| x as u16).collect::<alloc::vec::Vec<_>>());
        emit_u8_slice  (&mut out, "WORDS_DATA",        &words.data);
        emit_u16_slice(&mut out, "MAP_KEYS_IDENTITY",  &map_keys.identity.iter().map(|&x| x as u16).collect::<alloc::vec::Vec<_>>());
        emit_u16_slice(&mut out, "MAP_KEYS_DATA",      &map_keys.data);
        emit_u16_slice(&mut out, "MAP_VALS_IDENTITY",  &map_vals.identity.iter().map(|&x| x as u16).collect::<alloc::vec::Vec<_>>());
        emit_u16_slice(&mut out, "MAP_VALS_DATA",      &map_vals.data);
        emit_u16_slice(&mut out, "ARGS_KEYS_IDENTITY", &args_keys.identity.iter().map(|&x| x as u16).collect::<alloc::vec::Vec<_>>());
        emit_u16_slice(&mut out, "ARGS_KEYS_DATA",     &args_keys.data);
        emit_u16_slice(&mut out, "ARGS_VALS_IDENTITY", &args_vals.identity.iter().map(|&x| x as u16).collect::<alloc::vec::Vec<_>>());
        emit_u16_slice(&mut out, "ARGS_VALS_DATA",     &args_vals.data);

        std::fs::write(out_path, out)
            .map_err(|e| format!("write error: {e}"))
    }
}

// ── MetaBlock ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct MetaBlock {
    store_id:   u8,               // index into store_ids (0 = none)
    key_value:  Vec<u16>,         // value fragment u16s for key
    map_entries: Vec<(u16, u16)>, // (dst_word_id, src_word_id)
    arg_entries: Vec<(u16, Vec<u16>)>, // (key_word_id, val fragment u16s)
}

// ── Compiler ──────────────────────────────────────────────────────────────────

struct Compiler<'s> {
    store_ids:  &'s [&'s str],
    paths:      List<u64>,
    children:   VariableList<u16>,
    leaves:     VariableList<u16>,
    values:     VariableList<u16>,
    words:      VariableList<u8>,
    map_keys:   VariableList<u16>,
    map_vals:   VariableList<u16>,
    args_keys:  VariableList<u16>,
    args_vals:  VariableList<u16>,
}

impl<'s> Compiler<'s> {
    fn new(store_ids: &'s [&'s str]) -> Self {
        Self {
            store_ids,
            paths:     List::new(1),   // paths[0] placeholder
            children:  VariableList::new(),
            leaves:    VariableList::new(),
            values:    VariableList::new(),
            words:     VariableList::new(),
            map_keys:  VariableList::new(),
            map_vals:  VariableList::new(),
            args_keys: VariableList::new(),
            args_vals: VariableList::new(),
        }
    }

    // ── path push ─────────────────────────────────────────────────────────────

    fn push_path(&mut self, entry: u64) -> Result<u16, DslError> {
        let idx = self.paths.data.len();
        if idx > MAX_PATH_ID {
            return Err(DslError::LimitExceeded(alloc::format!(
                "path_id {} exceeds max {}", idx, MAX_PATH_ID
            )));
        }
        self.paths.data.push(entry);
        Ok(idx as u16)
    }

    // ── children slot helpers ─────────────────────────────────────────────────

    fn alloc_children_slots(&mut self, count: usize) -> Result<usize, DslError> {
        if count == 0 { return Ok(0); }
        let zeros: Vec<u16> = alloc::vec![0u16; count];
        match self.children.set(&0, &zeros, false) {
            Ok(crate::required::SetOutcome::Created(id)) => Ok(id),
            _ => Err(DslError::LimitExceeded("children allocation failed".into())),
        }
    }

    fn set_child(&mut self, children_id: usize, slot: usize, path_idx: u16) {
        if children_id == 0 { return; }
        let identity_start = children_id * 2;
        let start = self.children.identity[identity_start];
        self.children.data[start + slot] = path_idx;
    }

    // ── walk ──────────────────────────────────────────────────────────────────

    fn walk_field_key(
        &mut self,
        keyword:    &[u8],
        value:      &Tree,
        parent_idx: u16,
        inh_get:    Option<&MetaBlock>,
        inh_set:    Option<&MetaBlock>,
    ) -> Result<(), DslError> {
        let path_idx = self.push_path(0u64)?; // placeholder, filled below

        let keyword_id = self.intern_word(keyword)?;

        match value {
            Tree::Mapping(pairs) => {
                let get = self.resolve_meta(pairs, META_GET, inh_get)?;
                let set = self.resolve_meta(pairs, META_SET, inh_set)?;

                let field_pairs: Vec<_> = pairs.iter()
                    .filter(|(k, _)| k.first() != Some(&b'_'))
                    .collect();
                let child_count = field_pairs.len();

                if child_count == 0 {
                    self.write_leaf(path_idx, keyword_id, parent_idx, &Tree::Null, get.as_ref(), set.as_ref())?;
                } else {
                    let children_id = self.alloc_children_slots(child_count)?;
                    for (i, (k, v)) in field_pairs.iter().enumerate() {
                        let child_idx = self.paths.data.len() as u16;
                        self.set_child(children_id, i, child_idx);
                        self.walk_field_key(k, v, path_idx, get.as_ref(), set.as_ref())?;
                    }
                    self.paths.data[path_idx as usize] =
                        ((keyword_id as u64) << PATH_KEYWORD_ID_SHIFT)
                        | ((parent_idx as u64) << PATH_PARENT_ID_SHIFT)
                        | ((children_id as u64) << PATH_CHILD_ID_SHIFT);
                }
            }
            _ => {
                self.write_leaf(path_idx, keyword_id, parent_idx, value, inh_get, inh_set)?;
            }
        }
        Ok(())
    }

    // ── meta resolution ───────────────────────────────────────────────────────

    fn resolve_meta(
        &mut self,
        pairs:     &[(Vec<u8>, Tree)],
        meta_key:  &[u8],
        inherited: Option<&MetaBlock>,
    ) -> Result<Option<MetaBlock>, DslError> {
        let local = pairs.iter().find(|(k, _)| k.as_slice() == meta_key);
        match (local, inherited) {
            (None, None) => Ok(None),
            (None, Some(inh)) => Ok(Some(inh.clone())),
            (Some((_, Tree::Mapping(meta_pairs))), inh) => {
                let mut store_id    = inh.map(|b| b.store_id).unwrap_or(0);
                let mut key_value: Vec<u16>               = inh.map(|b| b.key_value.clone()).unwrap_or_default();
                let mut map_entries: Vec<(u16, u16)>      = inh.map(|b| b.map_entries.clone()).unwrap_or_default();
                let mut arg_entries: Vec<(u16, Vec<u16>)> = inh.map(|b| b.arg_entries.clone()).unwrap_or_default();

                for (k, v) in meta_pairs {
                    if k.as_slice() == PROP_STORE {
                        if let Tree::Scalar(b) = v {
                            store_id = self.resolve_store_id(b);
                        }
                    } else if k.as_slice() == PROP_KEY {
                        key_value = self.encode_value(v)?;
                    } else if k.as_slice() == PROP_MAP {
                        if let Tree::Mapping(map_pairs) = v {
                            map_entries.clear();
                            for (mk, mv) in map_pairs {
                                let dst = self.intern_word(mk)?;
                                let src = if let Tree::Scalar(b) = mv { self.intern_word(b)? } else { 0 };
                                map_entries.push((dst, src));
                            }
                        }
                    } else if k.as_slice() != META_GET
                           && k.as_slice() != META_SET
                           && k.as_slice() != META_STATE {
                        let ak = self.intern_word(k)?;
                        let av = self.encode_value(v)?;
                        if let Some(entry) = arg_entries.iter_mut().find(|(ek, _)| *ek == ak) {
                            entry.1 = av;
                        } else {
                            arg_entries.push((ak, av));
                        }
                    }
                }
                Ok(Some(MetaBlock { store_id, key_value, map_entries, arg_entries }))
            }
            _ => Ok(inherited.cloned()),
        }
    }

    // ── leaf serialization ────────────────────────────────────────────────────

    fn write_leaf(
        &mut self,
        path_idx:   u16,
        keyword_id: u16,
        parent_idx: u16,
        value:      &Tree,
        get:        Option<&MetaBlock>,
        set:        Option<&MetaBlock>,
    ) -> Result<(), DslError> {
        let value_frags = self.encode_value(value)?;
        let value_id = if value_frags.is_empty() {
            0u16
        } else {
            self.push_values(&value_frags)?
        };

        let (get_store_id, get_key_id, get_map_key_id, get_map_val_id, get_args_key_id, get_args_val_id)
            = self.encode_meta(get)?;
        let (set_store_id, set_key_id, set_map_key_id, set_map_val_id, set_args_key_id, set_args_val_id)
            = self.encode_meta(set)?;

        let leaf_data: [u16; LEAF_WIDTH] = [
            get_store_id, get_key_id,
            set_store_id, set_key_id,
            get_map_key_id, get_map_val_id,
            get_args_key_id, get_args_val_id,
            set_map_key_id, set_map_val_id,
            set_args_key_id, set_args_val_id,
        ];

        let leaf_id = match self.leaves.set(&0, &leaf_data, false) {
            Ok(crate::required::SetOutcome::Created(id)) => {
                if id > MAX_PATH_ID {
                    return Err(DslError::LimitExceeded(alloc::format!(
                        "leaf_id {} exceeds max {}", id, MAX_PATH_ID
                    )));
                }
                id as u16
            }
            _ => return Err(DslError::LimitExceeded("leaf allocation failed".into())),
        };

        self.paths.data[path_idx as usize] =
            PATH_IS_LEAF_MASK
            | ((keyword_id as u64) << PATH_KEYWORD_ID_SHIFT)
            | ((parent_idx as u64) << PATH_PARENT_ID_SHIFT)
            | ((leaf_id as u64)    << PATH_CHILD_ID_SHIFT)
            | (value_id as u64);
        Ok(())
    }

    fn encode_meta(&mut self, meta: Option<&MetaBlock>) -> Result<(u16, u16, u16, u16, u16, u16), DslError> {
        let Some(b) = meta else {
            return Ok((0, 0, 0, 0, 0, 0));
        };

        let key_id = if b.key_value.is_empty() { 0u16 } else { self.push_values(&b.key_value)? };

        let (map_key_id, map_val_id) = if b.map_entries.is_empty() {
            (0u16, 0u16)
        } else {
            let dsts: Vec<u16> = b.map_entries.iter().map(|&(d, _)| d).collect();
            let srcs: Vec<u16> = b.map_entries.iter().map(|&(_, s)| s).collect();
            (self.push_map_keys(&dsts)?,
             self.push_map_vals(&srcs)?)
        };

        let (args_key_id, args_val_id) = if b.arg_entries.is_empty() {
            (0u16, 0u16)
        } else {
            let keys: Vec<u16> = b.arg_entries.iter().map(|&(k, _)| k).collect();
            let mut val_ids: Vec<u16> = Vec::with_capacity(b.arg_entries.len());
            for (_, frags) in &b.arg_entries {
                let id = if frags.is_empty() { 0u16 } else { self.push_values(frags)? };
                val_ids.push(id);
            }
            let ak = match self.args_keys.set(&0, &keys, false) {
                Ok(crate::required::SetOutcome::Created(id)) => {
                    if id > MAX_PATH_ID { return Err(DslError::LimitExceeded("args_keys id overflow".into())); }
                    id as u16
                }
                _ => return Err(DslError::LimitExceeded("args_keys allocation failed".into())),
            };
            let av = match self.args_vals.set(&0, &val_ids, false) {
                Ok(crate::required::SetOutcome::Created(id)) => {
                    if id > MAX_PATH_ID { return Err(DslError::LimitExceeded("args_vals id overflow".into())); }
                    id as u16
                }
                _ => return Err(DslError::LimitExceeded("args_vals allocation failed".into())),
            };
            (ak, av)
        };

        Ok((b.store_id as u16, key_id, map_key_id, map_val_id, args_key_id, args_val_id))
    }

    // ── VariableList push helpers ─────────────────────────────────────────────

    fn push_values(&mut self, frags: &[u16]) -> Result<u16, DslError> {
        match self.values.set(&0, frags, false) {
            Ok(crate::required::SetOutcome::Created(id)) => {
                if id > MAX_PATH_ID {
                    return Err(DslError::LimitExceeded(alloc::format!(
                        "values id {} exceeds max {}", id, MAX_PATH_ID
                    )));
                }
                Ok(id as u16)
            }
            _ => Err(DslError::LimitExceeded("values allocation failed".into())),
        }
    }

    fn push_map_keys(&mut self, data: &[u16]) -> Result<u16, DslError> {
        match self.map_keys.set(&0, data, false) {
            Ok(crate::required::SetOutcome::Created(id)) => {
                if id > MAX_PATH_ID {
                    return Err(DslError::LimitExceeded(alloc::format!(
                        "map_keys id {} exceeds max {}", id, MAX_PATH_ID
                    )));
                }
                Ok(id as u16)
            }
            _ => Err(DslError::LimitExceeded("map_keys allocation failed".into())),
        }
    }

    fn push_map_vals(&mut self, data: &[u16]) -> Result<u16, DslError> {
        match self.map_vals.set(&0, data, false) {
            Ok(crate::required::SetOutcome::Created(id)) => {
                if id > MAX_PATH_ID {
                    return Err(DslError::LimitExceeded(alloc::format!(
                        "map_vals id {} exceeds max {}", id, MAX_PATH_ID
                    )));
                }
                Ok(id as u16)
            }
            _ => Err(DslError::LimitExceeded("map_vals allocation failed".into())),
        }
    }

    // ── value encoding ────────────────────────────────────────────────────────

    fn encode_value(&mut self, value: &Tree) -> Result<Vec<u16>, DslError> {
        let Tree::Scalar(b) = value else { return Ok(Vec::new()); };
        let mut frags: Vec<u16> = Vec::new();
        let mut rest = b.as_slice();
        while !rest.is_empty() {
            if let Some(start) = rest.windows(2).position(|w| w == b"${") {
                if start > 0 {
                    let word_id = self.intern_word(&rest[..start])?;
                    frags.push(word_id & VALUE_WORD_ID_MASK);
                }
                rest = &rest[start + 2..];
                if let Some(end) = rest.iter().position(|&c| c == b'}') {
                    let word_id = self.intern_word(&rest[..end])?;
                    frags.push(VALUE_IS_PLACEHOLDER_MASK | (word_id & VALUE_WORD_ID_MASK));
                    rest = &rest[end + 1..];
                } else {
                    let word_id = self.intern_word(rest)?;
                    frags.push(word_id & VALUE_WORD_ID_MASK);
                    break;
                }
            } else {
                let word_id = self.intern_word(rest)?;
                frags.push(word_id & VALUE_WORD_ID_MASK);
                break;
            }
        }
        Ok(frags)
    }

    // ── word interning ────────────────────────────────────────────────────────

    fn intern_word(&mut self, s: &[u8]) -> Result<u16, DslError> {
        match self.words.set(&0, s, true) {
            Ok(crate::required::SetOutcome::Created(id)) => {
                if id > MAX_WORD_ID {
                    return Err(DslError::LimitExceeded(alloc::format!(
                        "word_id {} exceeds max {} (15-bit limit)", id, MAX_WORD_ID
                    )));
                }
                Ok(id as u16)
            }
            _ => Err(DslError::LimitExceeded("word interning failed".into())),
        }
    }

    // ── store_id resolution ───────────────────────────────────────────────────

    fn resolve_store_id(&self, name: &[u8]) -> u8 {
        self.store_ids.iter().position(|s| s.as_bytes() == name)
            .map(|i| (i + 1) as u8) // 1-based; 0 = none
            .unwrap_or(0)
    }

    // ── finish ────────────────────────────────────────────────────────────────

    fn finish(self) -> Result<(
        List<u64>,
        VariableList<u16>,
        VariableList<u16>,
        VariableList<u16>,
        VariableList<u8>,
        VariableList<u16>,
        VariableList<u16>,
        VariableList<u16>,
        VariableList<u16>,
    ), DslError> {
        // Validate data lengths fit u16 offsets (checked again in write, but also needed for runtime compile)
        check_vl_data_u16(&self.children,  "children")?;
        check_vl_data_u16(&self.leaves,    "leaves")?;
        check_vl_data_u16(&self.values,    "values")?;
        check_vl_data_u16_from_u8(&self.words, "words")?;
        check_vl_data_u16(&self.map_keys,  "map_keys")?;
        check_vl_data_u16(&self.map_vals,  "map_vals")?;
        check_vl_data_u16(&self.args_keys, "args_keys")?;
        check_vl_data_u16(&self.args_vals, "args_vals")?;
        Ok((
            self.paths,
            self.children,
            self.leaves,
            self.values,
            self.words,
            self.map_keys,
            self.map_vals,
            self.args_keys,
            self.args_vals,
        ))
    }
}

// ── compile-time limit checks ─────────────────────────────────────────────────

fn check_vl_data_u16<T>(vl: &VariableList<T>, name: &str) -> Result<(), DslError> {
    if vl.data.len() > MAX_VL_U16_DATA {
        return Err(DslError::LimitExceeded(alloc::format!(
            "{} data length {} exceeds max {}", name, vl.data.len(), MAX_VL_U16_DATA
        )));
    }
    Ok(())
}

fn check_vl_data_u16_from_u8(vl: &VariableList<u8>, name: &str) -> Result<(), DslError> {
    if vl.data.len() > MAX_VL_U8_DATA {
        return Err(DslError::LimitExceeded(alloc::format!(
            "{} data length {} exceeds max {}", name, vl.data.len(), MAX_VL_U8_DATA
        )));
    }
    Ok(())
}

#[cfg(feature = "precompile")]
fn check_vl_u16_identity<T>(vl: &VariableList<T>, name: &str) -> Result<(), alloc::string::String> {
    for &offset in &vl.identity {
        if offset > MAX_VL_U16_DATA {
            return Err(alloc::format!(
                "{} identity offset {} exceeds u16 max {}", name, offset, MAX_VL_U16_DATA
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "precompile")]
fn check_vl_u16_identity_u8(vl: &VariableList<u8>, name: &str) -> Result<(), alloc::string::String> {
    for &offset in &vl.identity {
        if offset > MAX_VL_U8_DATA {
            return Err(alloc::format!(
                "{} identity offset {} exceeds u16 max {}", name, offset, MAX_VL_U8_DATA
            ));
        }
    }
    Ok(())
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
        serde_yaml_ng::Value::String(s) => Tree::Scalar(s.into_bytes()),
        serde_yaml_ng::Value::Number(n) => Tree::Scalar(n.to_string().into_bytes()),
        serde_yaml_ng::Value::Bool(b)   => Tree::Scalar(b.to_string().into_bytes()),
        serde_yaml_ng::Value::Null      => Tree::Null,
        _                               => Tree::Null,
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
fn emit_u16_slice(out: &mut alloc::string::String, name: &str, data: &[u16]) {
    extern crate std;
    use std::format;
    out.push_str(&format!("pub static {name}: &[u16] = &[\n"));
    for chunk in data.chunks(8) {
        out.push_str("    ");
        for v in chunk { out.push_str(&format!("0x{v:04x}, ")); }
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

    fn compile(tree: &Tree) -> (List<u64>, VariableList<u16>, VariableList<u16>, VariableList<u16>, VariableList<u8>, VariableList<u16>, VariableList<u16>, VariableList<u16>, VariableList<u16>) {
        Dsl::compile(tree, &[]).unwrap()
    }

    fn compile_with_stores<'a>(tree: &Tree, store_ids: &[&'a str]) -> (List<u64>, VariableList<u16>, VariableList<u16>, VariableList<u16>, VariableList<u8>, VariableList<u16>, VariableList<u16>, VariableList<u16>, VariableList<u16>) {
        Dsl::compile(tree, store_ids).unwrap()
    }

    // --- single_leaf ---

    #[test]
    fn single_leaf() {
        let (paths, ..) = compile(&mapping(vec![
            ("name", Tree::Null),
        ]));
        assert_eq!(paths.data.len(), 2);                             // root(0) + name(1)
        assert!(paths.data[0] & PATH_IS_LEAF_MASK == 0);             // root is not a leaf
        assert!(paths.data[1] & PATH_IS_LEAF_MASK != 0);             // name is a leaf
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
        assert_eq!(paths.data.len(), 4);                             // root(0) + user(1) + id(2) + name(3)
        assert!(paths.data[1] & PATH_IS_LEAF_MASK == 0);             // user is not a leaf
        assert!(children.data.len() >= 3);                           // root→user + user→id,name
    }

    // --- meta_key ---

    #[test]
    fn meta_key_excluded_from_paths() {
        let (paths, ..) = compile(&mapping(vec![
            ("user", mapping(vec![
                ("_get", mapping(vec![
                    ("store", scalar("Memory")),
                    ("key",    scalar("user:1")),
                ])),
                ("id", Tree::Null),
            ])),
        ]));
        // root(0) + user(1) + id(2) — _get must not appear
        assert_eq!(paths.data.len(), 3);
    }

    // --- get in leaf (store_id) ---

    #[test]
    fn get_store_id_set_in_leaf() {
        let store_ids = &["Memory", "Kvs"];
        let (paths, _, leaves, ..) = compile_with_stores(&mapping(vec![
            ("user", mapping(vec![
                ("_get", mapping(vec![
                    ("store", scalar("Memory")),
                    ("key",    scalar("user:1")),
                ])),
                ("id", Tree::Null),
            ])),
        ]), store_ids);
        // root(0), user(1), id(2) — id is leaf
        let id_path = paths.data[2];
        assert!(id_path & PATH_IS_LEAF_MASK != 0);
        let leaf_id = ((id_path & PATH_CHILD_ID_MASK) >> PATH_CHILD_ID_SHIFT) as usize;
        let leaf_start = leaves.identity[leaf_id * 2];
        let get_store_id = leaves.data[leaf_start + LEAF_GET_STORE_ID];
        // "Memory" is store_ids[0] → store_id = 1 (1-based)
        assert_eq!(get_store_id, 1);
    }

    // --- store inheritance ---

    #[test]
    fn store_inherited_to_child_leaf() {
        let store_ids = &["Memory", "Kvs"];
        let (paths, _, leaves, ..) = compile_with_stores(&mapping(vec![
            ("session", mapping(vec![
                ("_set", mapping(vec![
                    ("store", scalar("Kvs")),
                    ("key",    scalar("session:1")),
                ])),
                ("user", mapping(vec![
                    ("id", Tree::Null),
                ])),
            ])),
        ]), store_ids);
        // root(0), session(1), user(2), id(3) — id is leaf
        let id_path = paths.data[3];
        assert!(id_path & PATH_IS_LEAF_MASK != 0);
        let leaf_id = ((id_path & PATH_CHILD_ID_MASK) >> PATH_CHILD_ID_SHIFT) as usize;
        let leaf_start = leaves.identity[leaf_id * 2];
        let set_store_id = leaves.data[leaf_start + LEAF_SET_STORE_ID];
        // "Kvs" is store_ids[1] → store_id = 2 (1-based)
        assert_eq!(set_store_id, 2);
    }

    // --- word intern ---

    #[test]
    fn intern_dedup() {
        let (_, _, _, _, words, ..) = compile(&mapping(vec![
            ("a", scalar("hello")),
            ("b", scalar("hello")),
        ]));
        let hello_count = (1..words.identity.len() / 2).filter(|&i| {
            let start = words.identity[i * 2];
            let end   = words.identity[i * 2 + 1];
            words.data.get(start..end) == Some(b"hello" as &[u8])
        }).count();
        assert_eq!(hello_count, 1);
    }

    // --- value fragments ---

    #[test]
    fn static_value_encoded_as_single_fragment() {
        let (paths, _, _, values, words, ..) = compile(&mapping(vec![
            ("key", scalar("hello")),
        ]));
        let path = paths.data[1];
        assert!(path & PATH_IS_LEAF_MASK != 0);
        let value_id = (path & PATH_VALUE_ID_MASK) as usize;
        assert!(value_id != 0);
        let vstart = values.identity[value_id * 2];
        let vend   = values.identity[value_id * 2 + 1];
        let frags = &values.data[vstart..vend];
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0] & VALUE_IS_PLACEHOLDER_MASK, 0); // static
        let word_id = (frags[0] & VALUE_WORD_ID_MASK) as usize;
        let wstart = words.identity[word_id * 2];
        let wend   = words.identity[word_id * 2 + 1];
        assert_eq!(&words.data[wstart..wend], b"hello");
    }

    #[test]
    fn placeholder_value_encoded_as_single_fragment_is_placeholder() {
        let (paths, _, _, values, ..) = compile(&mapping(vec![
            ("copy", scalar("${session.user.id}")),
        ]));
        let path = paths.data[1];
        let value_id = (path & PATH_VALUE_ID_MASK) as usize;
        let vstart = values.identity[value_id * 2];
        let vend   = values.identity[value_id * 2 + 1];
        let frags = &values.data[vstart..vend];
        assert_eq!(frags.len(), 1);
        assert_ne!(frags[0] & VALUE_IS_PLACEHOLDER_MASK, 0); // placeholder
    }

    #[test]
    fn template_value_encoded_as_multiple_fragments() {
        let (paths, _, _, values, ..) = compile(&mapping(vec![
            ("key", scalar("prefix.${some.path}.suffix")),
        ]));
        let path = paths.data[1];
        let value_id = (path & PATH_VALUE_ID_MASK) as usize;
        let vstart = values.identity[value_id * 2];
        let vend   = values.identity[value_id * 2 + 1];
        let frags = &values.data[vstart..vend];
        assert_eq!(frags.len(), 3);
        assert_eq!(frags[0] & VALUE_IS_PLACEHOLDER_MASK, 0); // static
        assert_ne!(frags[1] & VALUE_IS_PLACEHOLDER_MASK, 0); // placeholder
        assert_eq!(frags[2] & VALUE_IS_PLACEHOLDER_MASK, 0); // static
    }

    // --- resolve_meta ---

    #[test]
    fn local_get_overrides_inherited_store() {
        let store_ids = &["ClientA", "ClientB"];
        let (paths, _, leaves, ..) = compile_with_stores(&mapping(vec![
            ("parent", mapping(vec![
                ("_get", mapping(vec![
                    ("store", scalar("ClientA")),
                    ("key",    scalar("k")),
                ])),
                ("child", mapping(vec![
                    ("_get", mapping(vec![
                        ("store", scalar("ClientB")),
                        ("key",    scalar("k")),
                    ])),
                    ("leaf", Tree::Null),
                ])),
            ])),
        ]), store_ids);
        // root(0), parent(1), child(2), leaf(3)
        let leaf_path = paths.data[3];
        assert!(leaf_path & PATH_IS_LEAF_MASK != 0);
        let leaf_id = ((leaf_path & PATH_CHILD_ID_MASK) >> PATH_CHILD_ID_SHIFT) as usize;
        let leaf_start = leaves.identity[leaf_id * 2];
        let get_store_id = leaves.data[leaf_start + LEAF_GET_STORE_ID];
        // "ClientB" is store_ids[1] → store_id = 2
        assert_eq!(get_store_id, 2);
    }

    #[test]
    fn get_inherited_when_no_local_override() {
        let store_ids = &["Inherited"];
        let (paths, _, leaves, ..) = compile_with_stores(&mapping(vec![
            ("parent", mapping(vec![
                ("_get", mapping(vec![
                    ("store", scalar("Inherited")),
                    ("key",    scalar("k")),
                ])),
                ("leaf", Tree::Null),
            ])),
        ]), store_ids);
        // root(0), parent(1), leaf(2)
        let leaf_path = paths.data[2];
        assert!(leaf_path & PATH_IS_LEAF_MASK != 0);
        let leaf_id = ((leaf_path & PATH_CHILD_ID_MASK) >> PATH_CHILD_ID_SHIFT) as usize;
        let leaf_start = leaves.identity[leaf_id * 2];
        let get_store_id = leaves.data[leaf_start + LEAF_GET_STORE_ID];
        // "Inherited" is store_ids[0] → store_id = 1
        assert_eq!(get_store_id, 1);
    }

    // --- precompile ---

    #[cfg(feature = "precompile")]
    #[test]
    fn write_tenant_yml() {
        extern crate std;
        let src = std::include_bytes!("../examples/tenant.yml");
        let out = std::env::temp_dir().join("tenant_compiled.rs");
        std::fs::remove_file(&out).ok();
        Dsl::write(src, &["Memory", "Kvs", "TenantDb", "CommonDb", "Env"], out.to_str().unwrap()).expect("write failed");
        let content = std::fs::read_to_string(&out).expect("output not written");
        assert!(content.contains("pub static PATHS:"));
    }
}
