use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use crate::common::parser::{ParsedManifest, parse};
use crate::common::pool::{DynamicPool, PathMap, ChildrenMap, KeyList, YamlValueList};
use crate::common::bit;

/// Indices of meta records for a given node, collected from root to node (child overrides parent).
#[derive(Debug, Default)]
pub struct MetaIndices {
    pub load:  Option<u16>,
    pub store: Option<u16>,
    pub state: Option<u16>,
}

/// Manages parsed manifest files with all pools shared globally across files.
/// Each file is parsed on first access and appended to the shared pools.
///
/// # Examples
///
/// ```
/// use state_engine::Manifest;
///
/// let mut store = Manifest::new("./examples/manifest");
/// assert!(store.load("cache").is_ok());
/// assert!(store.load("nonexistent").is_err());
/// ```
pub struct Manifest {
    manifest_dir: PathBuf,
    files: HashMap<String, ParsedManifest>,
    // shared pools across all loaded files
    pub dynamic: DynamicPool,
    pub path_map: PathMap,
    pub children_map: ChildrenMap,
    pub keys: KeyList,
    pub values: YamlValueList,
}

impl Manifest {
    pub fn new(manifest_dir: &str) -> Self {
        Self {
            manifest_dir: PathBuf::from(manifest_dir),
            files: HashMap::new(),
            dynamic: DynamicPool::new(),
            path_map: PathMap::new(),
            children_map: ChildrenMap::new(),
            keys: KeyList::new(),
            values: YamlValueList::new(),
        }
    }

    /// Loads and parses a manifest file by name (without extension) if not cached.
    /// Second call for the same file is a no-op (cached).
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::Manifest;
    ///
    /// let mut store = Manifest::new("./examples/manifest");
    ///
    /// // first load parses and caches
    /// assert!(store.load("cache").is_ok());
    ///
    /// // second load is a no-op
    /// assert!(store.load("cache").is_ok());
    ///
    /// // nonexistent file returns Err
    /// assert!(store.load("nonexistent").is_err());
    ///
    /// // after load, keys are globally unique across files
    /// assert!(store.load("session").is_ok());
    /// let cache_idx  = store.find("cache",   "user").unwrap();
    /// let session_idx = store.find("session", "sso_user_id").unwrap();
    /// assert_ne!(cache_idx, session_idx);
    /// ```
    pub fn load(&mut self, file: &str) -> Result<(), String> {
        crate::fn_log!("Manifest", "load", file);
        if self.files.contains_key(file) {
            return Ok(());
        }

        let yml_path  = self.manifest_dir.join(format!("{}.yml", file));
        let yaml_path = self.manifest_dir.join(format!("{}.yaml", file));
        let yml_exists  = yml_path.exists();
        let yaml_exists = yaml_path.exists();

        if yml_exists && yaml_exists {
            return Err(format!(
                "Ambiguous file: both '{}.yml' and '{}.yaml' exist.",
                file, file
            ));
        }

        let path = if yml_exists {
            yml_path
        } else if yaml_exists {
            yaml_path
        } else {
            return Err(format!("File not found: '{}.yml' or '{}.yaml'", file, file));
        };

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let pm = parse(
            file,
            &content,
            &mut self.dynamic,
            &mut self.path_map,
            &mut self.children_map,
            &mut self.keys,
            &mut self.values,
        )?;

        self.files.insert(file.to_string(), pm);
        Ok(())
    }

    /// Returns the file_key_idx for a loaded file.
    pub fn file_key_idx(&self, file: &str) -> Option<u16> {
        self.files.get(file).map(|pm| pm.file_key_idx)
    }

    /// Looks up a key record index by dot-separated path within a file.
    /// Returns `None` if file is not loaded or path not found.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::Manifest;
    ///
    /// let mut store = Manifest::new("./examples/manifest");
    /// store.load("cache").unwrap();
    ///
    /// // "user" exists
    /// assert!(store.find("cache", "user").is_some());
    ///
    /// // "user.id" exists
    /// assert!(store.find("cache", "user.id").is_some());
    ///
    /// // unknown path returns None
    /// assert!(store.find("cache", "never").is_none());
    /// ```
    pub fn find(&self, file: &str, path: &str) -> Option<u16> {
        let file_idx = self.files.get(file)?.file_key_idx;
        let file_record = self.keys.get(file_idx)?;

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
            let record = self.keys.get(idx)?;

            // skip meta keys
            if bit::get(record, bit::OFFSET_ROOT, bit::MASK_ROOT) != bit::ROOT_NULL {
                continue;
            }

            let dyn_idx = bit::get(record, bit::OFFSET_DYNAMIC, bit::MASK_DYNAMIC) as u16;
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
        let child_idx = bit::get(record, bit::OFFSET_CHILD, bit::MASK_CHILD) as u16;
        if child_idx == 0 {
            return vec![];
        }
        let has_children = bit::get(record, bit::OFFSET_HAS_CHILDREN, bit::MASK_HAS_CHILDREN);
        if has_children == 1 {
            self.children_map.get(child_idx)
                .map(|s| s.to_vec())
                .unwrap_or_default()
        } else {
            vec![child_idx]
        }
    }

    /// Returns meta record indices (_load/_store/_state) for a dot-path node.
    /// Collects from root to node; child overrides parent.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::Manifest;
    ///
    /// let mut store = Manifest::new("./examples/manifest");
    /// store.load("cache").unwrap();
    ///
    /// let meta = store.get_meta("cache", "user");
    /// assert!(meta.load.is_some());
    /// assert!(meta.store.is_some());
    ///
    /// // leaf node has _state
    /// let meta = store.get_meta("cache", "user.id");
    /// assert!(meta.state.is_some());
    /// ```
    pub fn get_meta(&self, file: &str, path: &str) -> MetaIndices {
        crate::fn_log!("Manifest", "get_meta", &format!("{}.{}", file, path));
        let file_idx = match self.files.get(file) {
            Some(pm) => pm.file_key_idx,
            None => return MetaIndices::default(),
        };

        let file_record = match self.keys.get(file_idx) {
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
                let record = match self.keys.get(idx) {
                    Some(r) => r,
                    None => continue,
                };
                if bit::get(record, bit::OFFSET_ROOT, bit::MASK_ROOT) != bit::ROOT_NULL {
                    continue;
                }
                let dyn_idx = bit::get(record, bit::OFFSET_DYNAMIC, bit::MASK_DYNAMIC) as u16;
                if self.dynamic.get(dyn_idx) == Some(segment) {
                    self.collect_meta(record, &mut meta);
                    found_idx = Some(idx);
                    break;
                }
            }
            match found_idx {
                Some(idx) => {
                    let record = self.keys.get(idx).unwrap();
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
            let child = match self.keys.get(idx) {
                Some(r) => r,
                None => continue,
            };
            let root = bit::get(child, bit::OFFSET_ROOT, bit::MASK_ROOT);
            match root {
                bit::ROOT_LOAD  => meta.load  = Some(idx),
                bit::ROOT_STORE => meta.store = Some(idx),
                bit::ROOT_STATE => meta.state = Some(idx),
                _ => {}
            }
        }
    }

    /// Returns indices of field-key leaf values for a node (meta keys and nulls excluded).
    /// Returns a vec of (dynamic_index, yaml_value_index) for each leaf child.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::Manifest;
    ///
    /// let mut store = Manifest::new("./examples/manifest");
    /// store.load("connection").unwrap();
    ///
    /// // "tag", "driver", "charset" are static leaf values
    /// let values = store.get_value("connection", "common");
    /// assert!(!values.is_empty());
    /// ```
    pub fn get_value(&self, file: &str, path: &str) -> Vec<(u16, u16)> {
        let node_idx = match self.find(file, path) {
            Some(idx) => idx,
            None => return vec![],
        };

        let record = match self.keys.get(node_idx) {
            Some(r) => r,
            None => return vec![],
        };

        let mut result = Vec::new();
        let children = self.children_of(record);

        for &idx in &children {
            let child = match self.keys.get(idx) {
                Some(r) => r,
                None => continue,
            };
            // skip meta keys
            if bit::get(child, bit::OFFSET_ROOT, bit::MASK_ROOT) != bit::ROOT_NULL {
                continue;
            }
            // only leaf nodes with a value
            if bit::get(child, bit::OFFSET_IS_LEAF, bit::MASK_IS_LEAF) == 0 {
                continue;
            }
            let dyn_idx   = bit::get(child, bit::OFFSET_DYNAMIC, bit::MASK_DYNAMIC) as u16;
            let value_idx = bit::get(child, bit::OFFSET_CHILD,   bit::MASK_CHILD)   as u16;
            result.push((dyn_idx, value_idx));
        }

        result
    }
}
