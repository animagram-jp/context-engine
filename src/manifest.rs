use std::collections::HashMap;
use std::path::PathBuf;
use crate::core::parser::{ParsedManifest, Value, parse};
use crate::core::pool::DynamicPool;
use crate::core::fixed_bits;
use crate::ports::provided::ManifestError;
use crate::ports::required::FileClient;

/// Indices of meta records for a given node, collected from root to node (child overrides parent).
#[derive(Debug, Default)]
pub struct MetaIndices {
    pub load:  Option<u16>,
    pub store: Option<u16>,
    pub state: Option<u16>,
}

/// Manages parsed manifest files with all vecs shared globally across files.
/// Each file is parsed on first access and appended to the shared vecs.
pub struct Manifest {
    manifest_dir: PathBuf,
    file: Box<dyn FileClient>,
    files: HashMap<String, ParsedManifest>,
    pub dynamic: DynamicPool,
    pub keys: Vec<u64>,
    pub values: Vec<[u64; 2]>,
    pub path_map: Vec<Vec<u16>>,
    pub children_map: Vec<Vec<u16>>,
}

impl Manifest {
    pub fn new(manifest_dir: &str) -> Self {
        Self {
            manifest_dir: PathBuf::from(manifest_dir),
            file: Box::new(crate::ports::default::DefaultFileClient),
            files: HashMap::new(),
            dynamic: DynamicPool::new(),
            keys: vec![0],
            values: vec![[0, 0]],
            path_map: vec![vec![]],
            children_map: vec![vec![]],
        }
    }

    /// Replaces the default FileClient. Useful for WASI/JS environments without std::fs.
    pub fn with_file(mut self, client: impl FileClient + 'static) -> Self {
        self.file = Box::new(client);
        self
    }

    /// Loads and parses a manifest file by name (without extension) if not cached.
    /// Second call for the same file is a no-op (cached).
    pub fn load(&mut self, file: &str) -> Result<(), ManifestError> {
        crate::fn_log!("Manifest", "load", file);
        if self.files.contains_key(file) {
            return Ok(());
        }

        let yml_path  = self.manifest_dir.join(format!("{}.yml", file));
        let yaml_path = self.manifest_dir.join(format!("{}.yaml", file));

        let yml_key  = yml_path.to_string_lossy();
        let yaml_key = yaml_path.to_string_lossy();
        let yml_content  = self.file.get(&yml_key);
        let yaml_content = self.file.get(&yaml_key);

        let content = match (yml_content, yaml_content) {
            (Some(_), Some(_)) => return Err(ManifestError::AmbiguousFile(format!(
                "both '{}.yml' and '{}.yaml' exist.", file, file
            ))),
            (Some(c), None) => c,
            (None, Some(c)) => c,
            (None, None) => return Err(ManifestError::FileNotFound(format!(
                "'{}.yml' or '{}.yaml'", file, file
            ))),
        };

        let yaml_root: serde_yaml_ng::Value = serde_yaml_ng::from_str(&content)
            .map_err(|e| ManifestError::ParseError(format!("YAML parse error: {}", e)))?;

        let pm = parse(
            file,
            yaml_to_value(yaml_root),
            &mut self.dynamic,
            &mut self.keys,
            &mut self.values,
            &mut self.path_map,
            &mut self.children_map,
        ).map_err(|e| ManifestError::ParseError(e))?;

        self.files.insert(file.to_string(), pm);
        Ok(())
    }

    /// Returns the file_key_idx for a loaded file.
    pub fn file_key_idx(&self, file: &str) -> Option<u16> {
        self.files.get(file).map(|pm| pm.file_key_idx)
    }

    /// Looks up a key record index by dot-separated path within a file.
    /// Returns `None` if file is not loaded or path not found.
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

    /// Recursively walks the Trie to find the record matching `segments`.
    fn find_in(&self, segments: &[&str], candidates: &[u16]) -> Option<u16> {
        let target = segments[0];
        let rest   = &segments[1..];

        for &idx in candidates {
            let record = self.keys.get(idx as usize).copied()?;

            // skip meta keys
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

    /// Returns the direct field-key children indices of a record.
    fn children_of(&self, record: u64) -> Vec<u16> {
        let child_idx = fixed_bits::get(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD) as usize;
        if child_idx == 0 {
            return vec![];
        }
        let has_children = fixed_bits::get(record, fixed_bits::K_OFFSET_HAS_CHILDREN, fixed_bits::K_MASK_HAS_CHILDREN);
        if has_children == 1 {
            self.children_map.get(child_idx)
                .map(|s| s.to_vec())
                .unwrap_or_default()
        } else {
            vec![child_idx as u16]
        }
    }

    /// Returns meta record indices (_load/_store/_state) for a dot-path node.
    /// Collects from root to node; child overrides parent.
    pub fn get_meta(&self, file: &str, path: &str) -> MetaIndices {
        crate::fn_log!("Manifest", "get_meta", &format!("{}.{}", file, path));
        let file_idx = match self.files.get(file) {
            Some(pm) => pm.file_key_idx,
            None => return MetaIndices::default(),
        };

        let file_record = match self.keys.get(file_idx as usize).copied() {
            Some(r) => r,
            None => return MetaIndices::default(),
        };

        let segments: Vec<&str> = if path.is_empty() {
            vec![]
        } else {
            path.split('.').collect()
        };

        let mut meta = MetaIndices::default();

        // collect meta from file root level
        self.collect_meta(file_record, &mut meta);

        // walk path segments, collecting meta at each level
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

    /// Scans children of `record` for meta records and updates `meta`.
    fn collect_meta(&self, record: u64, meta: &mut MetaIndices) {
        let children = self.children_of(record);
        for &idx in &children {
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

    /// Returns indices of field-key leaf values for a node (meta keys and nulls excluded).
    /// Returns a vec of (dynamic_index, yaml_value_index) for each leaf child.
    pub fn get_value(&self, file: &str, path: &str) -> Vec<(u16, u16)> {
        let node_idx = match self.find(file, path) {
            Some(idx) => idx,
            None => return vec![],
        };

        let record = match self.keys.get(node_idx as usize).copied() {
            Some(r) => r,
            None => return vec![],
        };

        let mut result = Vec::new();
        let children = self.children_of(record);

        for &idx in &children {
            let child = match self.keys.get(idx as usize).copied() {
                Some(r) => r,
                None => continue,
            };
            // skip meta keys
            if fixed_bits::get(child, fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT) != fixed_bits::ROOT_NULL {
                continue;
            }
            // only leaf nodes with a value
            if fixed_bits::get(child, fixed_bits::K_OFFSET_IS_LEAF, fixed_bits::K_MASK_IS_LEAF) == 0 {
                continue;
            }
            let dyn_idx   = fixed_bits::get(child, fixed_bits::K_OFFSET_DYNAMIC, fixed_bits::K_MASK_DYNAMIC) as u16;
            let value_idx = fixed_bits::get(child, fixed_bits::K_OFFSET_CHILD,   fixed_bits::K_MASK_CHILD)   as u16;
            result.push((dyn_idx, value_idx));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m() -> Manifest {
        Manifest::new("./examples/manifest")
    }

    // --- load ---

    #[test]
    fn test_load_ok() {
        let mut m = m();
        assert!(m.load("cache").is_ok());
    }

    #[test]
    fn test_load_noop_on_second_call() {
        let mut m = m();
        assert!(m.load("cache").is_ok());
        assert!(m.load("cache").is_ok());
    }

    #[test]
    fn test_load_file_not_found() {
        let mut m = m();
        assert!(matches!(m.load("nonexistent"), Err(ManifestError::FileNotFound(_))));
    }

    // --- find ---

    #[test]
    fn test_find_top_level() {
        let mut m = m();
        m.load("cache").unwrap();
        assert!(m.find("cache", "user").is_some());
    }

    #[test]
    fn test_find_nested() {
        let mut m = m();
        m.load("cache").unwrap();
        assert!(m.find("cache", "user.id").is_some());
    }

    #[test]
    fn test_find_unknown() {
        let mut m = m();
        m.load("cache").unwrap();
        assert!(m.find("cache", "nonexistent").is_none());
    }

    #[test]
    fn test_find_not_loaded_returns_none() {
        let m = m();
        assert!(m.find("cache", "user").is_none());
    }

    #[test]
    fn test_find_unique_indices_across_files() {
        // parser guarantees uniqueness, but Manifest must load both correctly
        let mut m = m();
        m.load("cache").unwrap();
        m.load("connection").unwrap();
        let cache_idx = m.find("cache", "user").unwrap();
        let conn_idx  = m.find("connection", "common").unwrap();
        assert_ne!(cache_idx, conn_idx);
    }

    // --- get_meta ---

    #[test]
    fn test_get_meta_has_load_and_store() {
        // cache.user has both _load (Db) and _store (KVS)
        let mut m = m();
        m.load("cache").unwrap();
        let meta = m.get_meta("cache", "user");
        assert!(meta.load.is_some());
        assert!(meta.store.is_some());
    }

    #[test]
    fn test_get_meta_leaf_has_state() {
        // cache.user.id has _state
        let mut m = m();
        m.load("cache").unwrap();
        let meta = m.get_meta("cache", "user.id");
        assert!(meta.state.is_some());
    }

    #[test]
    fn test_get_meta_inheritance() {
        // cache.user.id has no _store of its own; inherits from cache.user (_store: KVS)
        let mut m = m();
        m.load("cache").unwrap();
        let parent_meta = m.get_meta("cache", "user");
        let child_meta  = m.get_meta("cache", "user.id");
        // both point to a _store record — child inherits parent's
        assert!(child_meta.store.is_some());
        assert_eq!(child_meta.store, parent_meta.store);
    }

    #[test]
    fn test_get_meta_child_overrides_parent_load() {
        // cache.user.tenant_id has its own _load (State), overriding user's _load (Db)
        let mut m = m();
        m.load("cache").unwrap();
        let parent_meta = m.get_meta("cache", "user");
        let child_meta  = m.get_meta("cache", "user.tenant_id");
        assert_ne!(child_meta.load, parent_meta.load);
    }

    #[test]
    fn test_get_meta_unknown_path_returns_default() {
        let mut m = m();
        m.load("cache").unwrap();
        let meta = m.get_meta("cache", "nonexistent");
        assert!(meta.load.is_none());
        assert!(meta.store.is_none());
        assert!(meta.state.is_none());
    }

    // --- get_value ---

    #[test]
    fn test_get_value_returns_static_leaves() {
        // connection.common has static leaf values: tag, driver, charset
        let mut m = m();
        m.load("connection").unwrap();
        let values = m.get_value("connection", "common");
        assert!(!values.is_empty());
    }

    #[test]
    fn test_get_value_excludes_meta_keys() {
        // cache.user has _store/_load but those must not appear in get_value
        let mut m = m();
        m.load("cache").unwrap();
        let values = m.get_value("cache", "user");
        // user has no static leaf children (id/org_id/tenant_id are non-leaf nodes)
        assert!(values.is_empty());
    }

    #[test]
    fn test_get_value_unknown_path_returns_empty() {
        let mut m = m();
        m.load("cache").unwrap();
        let values = m.get_value("cache", "nonexistent");
        assert!(values.is_empty());
    }
}

fn yaml_to_value(v: serde_yaml_ng::Value) -> Value {
    match v {
        serde_yaml_ng::Value::Mapping(m) => Value::Mapping(
            m.into_iter()
                .filter_map(|(k, v)| {
                    let key = match k {
                        serde_yaml_ng::Value::String(s) => s,
                        _ => return None,
                    };
                    Some((key, yaml_to_value(v)))
                })
                .collect(),
        ),
        serde_yaml_ng::Value::String(s) => Value::Scalar(s),
        serde_yaml_ng::Value::Number(n) => Value::Scalar(n.to_string()),
        serde_yaml_ng::Value::Bool(b)   => Value::Scalar(b.to_string()),
        serde_yaml_ng::Value::Null      => Value::Null,
        _                               => Value::Null,
    }
}
