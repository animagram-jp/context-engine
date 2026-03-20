extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

use super::fixed_bits;
use super::codec;
use super::pool::DynamicPool;
use super::parser::ParsedManifest;

/// A resolved or unresolved config value produced by `build_config`.
/// State layer is responsible for resolving `Placeholder` variants via `State::get()`.
#[derive(Debug)]
pub enum ConfigValue {
    /// Static string value (no placeholder resolution needed).
    Str(String),
    /// A placeholder path that must be resolved via State::get().
    /// Used for both scalar placeholders and object-valued placeholders (e.g. connection).
    Placeholder(String),
    /// A map of (yaml_key → db_column) pairs.
    Map(Vec<(String, String)>),
    /// Numeric client id.
    Client(u64),
}

/// Owns all parsed manifest data and provides decode queries.
/// Pure logic — no I/O, no std, no serde_json.
pub struct Manifest {
    pub files: BTreeMap<String, ParsedManifest>,
    pub dynamic: DynamicPool,
    pub keys: Vec<u64>,
    pub values: Vec<[u64; 2]>,
    pub path_map: Vec<Vec<u16>>,
    pub children_map: Vec<Vec<u16>>,
}

impl Manifest {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
            dynamic: DynamicPool::new(),
            keys: alloc::vec![0],
            values: alloc::vec![[0, 0]],
            path_map: alloc::vec![alloc::vec![]],
            children_map: alloc::vec![alloc::vec![]],
        }
    }

    pub fn is_loaded(&self, file: &str) -> bool {
        self.files.contains_key(file)
    }

    pub fn insert(&mut self, file: String, pm: ParsedManifest) {
        self.files.insert(file, pm);
    }

    /// Returns the direct field-key and meta-key children indices of a record.
    pub fn children_of(&self, record: u64) -> Vec<u16> {
        let child_idx = fixed_bits::get(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD) as usize;
        if child_idx == 0 {
            return alloc::vec![];
        }
        let has_children = fixed_bits::get(record, fixed_bits::K_OFFSET_HAS_CHILDREN, fixed_bits::K_MASK_HAS_CHILDREN);
        if has_children == 1 {
            self.children_map.get(child_idx)
                .map(|s| s.to_vec())
                .unwrap_or_default()
        } else {
            alloc::vec![child_idx as u16]
        }
    }

    /// Looks up a key record index by dot-separated path within a file.
    pub fn find(&self, file: &str, path: &str) -> Option<u16> {
        let file_idx = self.files.get(file)?.file_key_idx;
        let file_record = self.keys.get(file_idx as usize).copied()?;

        if path.is_empty() {
            return Some(file_idx);
        }

        let segments: Vec<&str> = path.split('.').collect();
        let top_level = self.children_of(file_record);
        self.find_in(&segments, &top_level)
    }

    fn find_in(&self, segments: &[&str], candidates: &[u16]) -> Option<u16> {
        let target = segments[0];
        let rest = &segments[1..];

        for &idx in candidates {
            let record = self.keys.get(idx as usize).copied()?;
            if fixed_bits::get(record, fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT) != fixed_bits::ROOT_NULL {
                continue;
            }
            let dyn_idx = fixed_bits::get(record, fixed_bits::K_OFFSET_DYNAMIC, fixed_bits::K_MASK_DYNAMIC) as u16;
            if self.dynamic.get(dyn_idx)? != target {
                continue;
            }
            if rest.is_empty() {
                return Some(idx);
            }
            let next = self.children_of(record);
            if next.is_empty() {
                return None;
            }
            return self.find_in(rest, &next);
        }
        None
    }

    /// Returns meta record indices (_load/_store/_state) for a dot-path node.
    /// Collects from root to node; child overrides parent.
    pub fn get_meta(&self, file: &str, path: &str) -> MetaIndices {
        let file_idx = match self.files.get(file) {
            Some(pm) => pm.file_key_idx,
            None => return MetaIndices::default(),
        };
        let file_record = match self.keys.get(file_idx as usize).copied() {
            Some(r) => r,
            None => return MetaIndices::default(),
        };

        let segments: Vec<&str> = if path.is_empty() { alloc::vec![] } else { path.split('.').collect() };
        let mut meta = MetaIndices::default();
        self.collect_meta(file_record, &mut meta);

        let mut candidates = self.children_of(file_record);
        for segment in &segments {
            let mut found_idx = None;
            for &idx in &candidates {
                let record = match self.keys.get(idx as usize).copied() {
                    Some(r) => r,
                    None => continue,
                };
                if fixed_bits::get(record, fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT) != fixed_bits::ROOT_NULL {
                    continue;
                }
                let dyn_idx = fixed_bits::get(record, fixed_bits::K_OFFSET_DYNAMIC, fixed_bits::K_MASK_DYNAMIC) as u16;
                if self.dynamic.get(dyn_idx) == Some(segment) {
                    self.collect_meta(record, &mut meta);
                    found_idx = Some(idx);
                    break;
                }
            }
            match found_idx {
                Some(idx) => {
                    let record = self.keys[idx as usize];
                    candidates = self.children_of(record);
                }
                None => return MetaIndices::default(),
            }
        }
        meta
    }

    fn collect_meta(&self, record: u64, meta: &mut MetaIndices) {
        for &idx in &self.children_of(record) {
            let child = match self.keys.get(idx as usize).copied() {
                Some(r) => r,
                None => continue,
            };
            let root = fixed_bits::get(child, fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT);
            match root {
                fixed_bits::ROOT_LOAD  => meta.load  = Some(idx),
                fixed_bits::ROOT_STORE => meta.store = Some(idx),
                fixed_bits::ROOT_STATE => meta.state = Some(idx),
                _ => {}
            }
        }
    }

    /// Returns the client id encoded in a meta record (e.g. _load or _store).
    pub fn get_client(&self, meta_idx: u16) -> u64 {
        let record = match self.keys.get(meta_idx as usize).copied() {
            Some(r) => r,
            None => return fixed_bits::CLIENT_NULL,
        };
        // client is stored directly on the meta record's children
        for &child_idx in &self.children_of(record) {
            let child = match self.keys.get(child_idx as usize).copied() {
                Some(r) => r,
                None => continue,
            };
            let client = fixed_bits::get(child, fixed_bits::K_OFFSET_CLIENT, fixed_bits::K_MASK_CLIENT);
            if client != fixed_bits::CLIENT_NULL {
                return client;
            }
        }
        fixed_bits::CLIENT_NULL
    }

    /// Decodes a meta record into a list of (prop_name, ConfigValue) pairs.
    /// The caller (State) is responsible for resolving any `ConfigValue::Placeholder` entries.
    pub fn build_config(&self, meta_idx: u16) -> Option<Vec<(String, ConfigValue)>> {
        let record = self.keys.get(meta_idx as usize).copied()?;
        let child_idx = fixed_bits::get(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD) as usize;
        if child_idx == 0 {
            return None;
        }
        let children = if fixed_bits::get(record, fixed_bits::K_OFFSET_HAS_CHILDREN, fixed_bits::K_MASK_HAS_CHILDREN) == 1 {
            self.children_map.get(child_idx)?.to_vec()
        } else {
            alloc::vec![child_idx as u16]
        };

        let mut entries: Vec<(String, ConfigValue)> = alloc::vec![];

        for &child_idx in &children {
            let child_record = match self.keys.get(child_idx as usize).copied() {
                Some(r) => r,
                None => continue,
            };
            let prop   = fixed_bits::get(child_record, fixed_bits::K_OFFSET_PROP,   fixed_bits::K_MASK_PROP)   as u8;
            let client = fixed_bits::get(child_record, fixed_bits::K_OFFSET_CLIENT, fixed_bits::K_MASK_CLIENT);
            let is_leaf = fixed_bits::get(child_record, fixed_bits::K_OFFSET_IS_LEAF, fixed_bits::K_MASK_IS_LEAF) == 1;
            let value_idx = if is_leaf {
                fixed_bits::get(child_record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD) as u16
            } else { 0 };

            if client != fixed_bits::CLIENT_NULL {
                entries.push(("client".into(), ConfigValue::Client(client)));
                continue;
            }

            let prop_name = match codec::prop_decode(prop as u64) {
                Some(name) => name,
                None => continue,
            };

            if prop_name == "map" {
                if let Some(pairs) = self.decode_map(child_idx) {
                    entries.push(("map".into(), ConfigValue::Map(pairs)));
                }
            } else if prop_name == "connection" {
                if value_idx != 0 {
                    if let Some(cv) = self.decode_value(value_idx) {
                        entries.push(("connection".into(), cv));
                    }
                }
            } else if value_idx != 0 {
                if let Some(cv) = self.decode_value(value_idx) {
                    entries.push((prop_name.into(), cv));
                }
            }
        }

        Some(entries)
    }

    /// Decodes a map prop's children into (yaml_key, db_column) pairs.
    pub fn decode_map(&self, map_idx: u16) -> Option<Vec<(String, String)>> {
        let record = self.keys.get(map_idx as usize).copied()?;
        let child_idx = fixed_bits::get(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD) as usize;
        if child_idx == 0 {
            return Some(alloc::vec![]);
        }
        let children = if fixed_bits::get(record, fixed_bits::K_OFFSET_HAS_CHILDREN, fixed_bits::K_MASK_HAS_CHILDREN) == 1 {
            self.children_map.get(child_idx)?.to_vec()
        } else {
            alloc::vec![child_idx as u16]
        };

        let mut pairs = alloc::vec![];
        for &c in &children {
            let child = self.keys.get(c as usize).copied()?;
            let dyn_idx   = fixed_bits::get(child, fixed_bits::K_OFFSET_DYNAMIC, fixed_bits::K_MASK_DYNAMIC) as u16;
            let value_idx = fixed_bits::get(child, fixed_bits::K_OFFSET_CHILD,   fixed_bits::K_MASK_CHILD) as usize;
            let is_path   = fixed_bits::get(child, fixed_bits::K_OFFSET_IS_PATH, fixed_bits::K_MASK_IS_PATH) == 1;

            let key_str = if is_path {
                let segs = self.path_map.get(dyn_idx as usize)?;
                segs.iter()
                    .filter_map(|&s| self.dynamic.get(s).map(|x| x.to_string()))
                    .collect::<Vec<_>>()
                    .join(".")
            } else {
                self.dynamic.get(dyn_idx)?.to_string()
            };

            let val_vo = self.values.get(value_idx).copied()?;
            let col_dyn = fixed_bits::get(val_vo[0], fixed_bits::V_OFFSET_T0_DYNAMIC, fixed_bits::V_MASK_DYNAMIC) as u16;
            let col_str = self.dynamic.get(col_dyn)?.to_string();

            pairs.push((key_str, col_str));
        }
        Some(pairs)
    }

    /// Decodes a value record into a ConfigValue.
    /// If the value is a placeholder path, returns Placeholder(path_string).
    /// If it's a template or static string, returns Str(string) or Placeholder for single-path tokens.
    pub fn decode_value(&self, value_idx: u16) -> Option<ConfigValue> {
        let vo = self.values.get(value_idx as usize).copied()?;
        let is_template = fixed_bits::get(vo[0], fixed_bits::V_OFFSET_IS_TEMPLATE, fixed_bits::V_MASK_IS_TEMPLATE) == 1;
        let is_path0 = fixed_bits::get(vo[0], fixed_bits::V_OFFSET_T0_IS_PATH, fixed_bits::V_MASK_IS_PATH) == 1;
        let dyn_idx0 = fixed_bits::get(vo[0], fixed_bits::V_OFFSET_T0_DYNAMIC, fixed_bits::V_MASK_DYNAMIC) as u16;

        // single pure placeholder (non-template, is_path) → Placeholder
        if is_path0 && dyn_idx0 != 0 && !is_template {
            let path = self.resolve_path(dyn_idx0)?;
            return Some(ConfigValue::Placeholder(path));
        }

        // template or static: collect all tokens
        Some(ConfigValue::Str(self.decode_value_tokens(vo)?))
    }

    /// Decodes all tokens of a value record into a raw string,
    /// embedding placeholder paths as `${path}` so the caller can resolve them.
    pub fn decode_value_tokens(&self, vo: [u64; 2]) -> Option<String> {
        const TOKEN_OFFSETS: [(u32, u32); 6] = [
            (fixed_bits::V_OFFSET_T0_IS_PATH, fixed_bits::V_OFFSET_T0_DYNAMIC),
            (fixed_bits::V_OFFSET_T1_IS_PATH, fixed_bits::V_OFFSET_T1_DYNAMIC),
            (fixed_bits::V_OFFSET_T2_IS_PATH, fixed_bits::V_OFFSET_T2_DYNAMIC),
            (fixed_bits::V_OFFSET_T3_IS_PATH, fixed_bits::V_OFFSET_T3_DYNAMIC),
            (fixed_bits::V_OFFSET_T4_IS_PATH, fixed_bits::V_OFFSET_T4_DYNAMIC),
            (fixed_bits::V_OFFSET_T5_IS_PATH, fixed_bits::V_OFFSET_T5_DYNAMIC),
        ];

        let mut result = String::new();
        for (i, (off_is_path, off_dynamic)) in TOKEN_OFFSETS.iter().enumerate() {
            let word = if i < 3 { 0 } else { 1 };
            let is_path = fixed_bits::get(vo[word], *off_is_path, fixed_bits::V_MASK_IS_PATH) == 1;
            let dyn_idx = fixed_bits::get(vo[word], *off_dynamic, fixed_bits::V_MASK_DYNAMIC) as u16;
            if dyn_idx == 0 { break; }

            if is_path {
                let path = self.resolve_path(dyn_idx)?;
                result.push_str("${");
                result.push_str(&path);
                result.push('}');
            } else {
                result.push_str(self.dynamic.get(dyn_idx)?);
            }
        }
        Some(result)
    }

    /// Resolves a path_map index to a dot-joined path string.
    fn resolve_path(&self, path_map_idx: u16) -> Option<String> {
        let segs = self.path_map.get(path_map_idx as usize)?;
        let path = segs.iter()
            .filter_map(|&s| self.dynamic.get(s).map(|x| x.to_string()))
            .collect::<Vec<_>>()
            .join(".");
        Some(path)
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self::new()
    }
}

/// Indices of meta records for a given node, collected from root to node (child overrides parent).
#[derive(Debug, Default)]
pub struct MetaIndices {
    pub load:  Option<u16>,
    pub store: Option<u16>,
    pub state: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::parser::{Value, parse};

    /// Builds a Manifest from a inline DSL mapping.
    /// `entries` is the top-level key→subtree mapping for a single file.
    fn make(filename: &str, entries: Vec<(&str, Value)>) -> Manifest {
        let mut m = Manifest::new();
        let root = Value::Mapping(entries.into_iter().map(|(k, v)| (k.to_string(), v)).collect());
        let pm = parse(filename, root, &mut m.dynamic, &mut m.keys, &mut m.values, &mut m.path_map, &mut m.children_map).unwrap();
        m.insert(filename.to_string(), pm);
        m
    }

    fn scalar(s: &str) -> Value { Value::Scalar(s.to_string()) }
    fn mapping(entries: Vec<(&str, Value)>) -> Value {
        Value::Mapping(entries.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    fn cache_manifest() -> Manifest {
        make("cache", vec![
            ("user", mapping(vec![
                ("_store", mapping(vec![
                    ("client", scalar("KVS")),
                    ("key", scalar("user:${session.sso_user_id}")),
                ])),
                ("_load", mapping(vec![
                    ("client", scalar("Db")),
                    ("connection", scalar("${connection.tenant}")),
                    ("table", scalar("users")),
                    ("map", mapping(vec![
                        ("id", scalar("id")),
                        ("org_id", scalar("sso_org_id")),
                    ])),
                ])),
                ("id", mapping(vec![
                    ("_state", mapping(vec![("type", scalar("integer"))])),
                ])),
                ("tenant_id", mapping(vec![
                    ("_state", mapping(vec![("type", scalar("integer"))])),
                    ("_load", mapping(vec![
                        ("client", scalar("State")),
                        ("key", scalar("${org_id}")),
                    ])),
                ])),
            ])),
        ])
    }

    // --- find ---

    #[test]
    fn test_find_file_not_loaded_returns_none() {
        let m = Manifest::new();
        assert!(m.find("cache", "user").is_none());
    }

    #[test]
    fn test_find_top_level() {
        let m = cache_manifest();
        assert!(m.find("cache", "user").is_some());
    }

    #[test]
    fn test_find_nested() {
        let m = cache_manifest();
        assert!(m.find("cache", "user.id").is_some());
    }

    #[test]
    fn test_find_unknown_returns_none() {
        let m = cache_manifest();
        assert!(m.find("cache", "nonexistent").is_none());
    }

    #[test]
    fn test_find_unknown_nested_returns_none() {
        let m = cache_manifest();
        assert!(m.find("cache", "user.nonexistent").is_none());
    }

    #[test]
    fn test_find_unique_indices_across_files() {
        let mut m = cache_manifest();
        let root2 = Value::Mapping(vec![
            ("common".to_string(), Value::Mapping(vec![
                ("_store".to_string(), Value::Mapping(vec![
                    ("client".to_string(), Value::Scalar("InMemory".to_string())),
                ])),
            ])),
        ]);
        let pm2 = parse("connection", root2, &mut m.dynamic, &mut m.keys, &mut m.values, &mut m.path_map, &mut m.children_map).unwrap();
        m.insert("connection".to_string(), pm2);

        let cache_idx = m.find("cache", "user").unwrap();
        let conn_idx  = m.find("connection", "common").unwrap();
        assert_ne!(cache_idx, conn_idx);
    }

    // --- get_meta ---

    #[test]
    fn test_get_meta_has_load_and_store() {
        let m = cache_manifest();
        let meta = m.get_meta("cache", "user");
        assert!(meta.load.is_some());
        assert!(meta.store.is_some());
    }

    #[test]
    fn test_get_meta_leaf_has_state() {
        let m = cache_manifest();
        let meta = m.get_meta("cache", "user.id");
        assert!(meta.state.is_some());
    }

    #[test]
    fn test_get_meta_child_inherits_parent_store() {
        let m = cache_manifest();
        let parent = m.get_meta("cache", "user");
        let child  = m.get_meta("cache", "user.id");
        assert!(child.store.is_some());
        assert_eq!(child.store, parent.store);
    }

    #[test]
    fn test_get_meta_child_overrides_parent_load() {
        let m = cache_manifest();
        let parent = m.get_meta("cache", "user");
        let child  = m.get_meta("cache", "user.tenant_id");
        assert!(child.load.is_some());
        assert_ne!(child.load, parent.load);
    }

    #[test]
    fn test_get_meta_unknown_path_returns_default() {
        let m = cache_manifest();
        let meta = m.get_meta("cache", "nonexistent");
        assert!(meta.load.is_none());
        assert!(meta.store.is_none());
        assert!(meta.state.is_none());
    }

    #[test]
    fn test_get_meta_file_not_loaded_returns_default() {
        let m = Manifest::new();
        let meta = m.get_meta("cache", "user");
        assert!(meta.load.is_none());
    }

    // --- get_client ---

    #[test]
    fn test_get_client_kvs() {
        let m = cache_manifest();
        let meta = m.get_meta("cache", "user");
        let client = m.get_client(meta.store.unwrap());
        assert_eq!(client, super::super::fixed_bits::CLIENT_KVS);
    }

    #[test]
    fn test_get_client_db() {
        let m = cache_manifest();
        let meta = m.get_meta("cache", "user");
        let client = m.get_client(meta.load.unwrap());
        assert_eq!(client, super::super::fixed_bits::CLIENT_DB);
    }

    #[test]
    fn test_get_client_state() {
        let m = cache_manifest();
        let meta = m.get_meta("cache", "user.tenant_id");
        let client = m.get_client(meta.load.unwrap());
        assert_eq!(client, super::super::fixed_bits::CLIENT_STATE);
    }

    // --- build_config ---

    #[test]
    fn test_build_config_contains_client() {
        let m = cache_manifest();
        let meta = m.get_meta("cache", "user");
        let entries = m.build_config(meta.store.unwrap()).unwrap();
        assert!(entries.iter().any(|(k, _)| k == "client"));
    }

    #[test]
    fn test_build_config_connection_is_placeholder() {
        let m = cache_manifest();
        let meta = m.get_meta("cache", "user");
        let entries = m.build_config(meta.load.unwrap()).unwrap();
        let conn = entries.iter().find(|(k, _)| k == "connection");
        assert!(matches!(conn, Some((_, ConfigValue::Placeholder(_)))));
    }

    #[test]
    fn test_build_config_map_is_map_variant() {
        let m = cache_manifest();
        let meta = m.get_meta("cache", "user");
        let entries = m.build_config(meta.load.unwrap()).unwrap();
        let map = entries.iter().find(|(k, _)| k == "map");
        assert!(matches!(map, Some((_, ConfigValue::Map(_)))));
        if let Some((_, ConfigValue::Map(pairs))) = map {
            assert!(!pairs.is_empty());
        }
    }

    #[test]
    fn test_build_config_key_with_template_is_str() {
        let m = cache_manifest();
        let meta = m.get_meta("cache", "user");
        let entries = m.build_config(meta.store.unwrap()).unwrap();
        let key = entries.iter().find(|(k, _)| k == "key");
        assert!(matches!(key, Some((_, ConfigValue::Str(_)))));
        if let Some((_, ConfigValue::Str(s))) = key {
            assert!(s.contains("${"));
        }
    }

    // --- decode_value ---

    #[test]
    fn test_decode_value_single_placeholder() {
        // connection: ${connection.tenant} → Placeholder
        let m = cache_manifest();
        let meta = m.get_meta("cache", "user");
        let entries = m.build_config(meta.load.unwrap()).unwrap();
        let conn = entries.iter().find(|(k, _)| k == "connection");
        assert!(matches!(conn, Some((_, ConfigValue::Placeholder(p))) if p == "connection.tenant"));
    }

    #[test]
    fn test_decode_value_template_embeds_placeholder() {
        // key: "user:${session.sso_user_id}" → Str containing ${...}
        let m = cache_manifest();
        let meta = m.get_meta("cache", "user");
        let entries = m.build_config(meta.store.unwrap()).unwrap();
        let key = entries.iter().find(|(k, _)| k == "key");
        if let Some((_, ConfigValue::Str(s))) = key {
            assert!(s.contains("${session.sso_user_id}"));
        } else {
            panic!("expected Str");
        }
    }
}
