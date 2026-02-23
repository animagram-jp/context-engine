use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use super::parser::ParsedManifest;
use super::bit;

/// Manages parsed manifest files, keyed by filename (without extension).
/// Each file is parsed on first access and cached as a `ParsedManifest`.
pub struct ManifestStore {
    manifest_dir: PathBuf,
    files: HashMap<String, ParsedManifest>,
    missing_keys: Vec<String>,
}

impl ManifestStore {
    pub fn new(manifest_dir: &str) -> Self {
        Self {
            manifest_dir: PathBuf::from(manifest_dir),
            files: HashMap::new(),
            missing_keys: Vec::new(),
        }
    }

    /// Loads and parses a manifest file by name (without extension) if not cached.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::common::manifest::ManifestStore;
    ///
    /// let mut store = ManifestStore::new("./examples/manifest");
    /// assert!(store.load("cache").is_ok());
    /// assert!(store.load("nonexistent").is_err());
    /// ```
    pub fn load(&mut self, file: &str) -> Result<(), String> {
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

        let mut pm = ParsedManifest::new();
        pm.parse(file, &content)?;

        self.files.insert(file.to_string(), pm);
        Ok(())
    }

    /// Returns a reference to the parsed manifest for the given file.
    pub fn get_file(&self, file: &str) -> Option<&ParsedManifest> {
        self.files.get(file)
    }

    /// Looks up a key record index by dot-separated path within a file.
    /// Returns `None` if file is not loaded or path not found.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::common::manifest::ManifestStore;
    ///
    /// let mut store = ManifestStore::new("./examples/manifest");
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
        let pm = self.files.get(file)?;
        let segments: Vec<&str> = if path.is_empty() {
            vec![]
        } else {
            path.split('.').collect()
        };

        // index 1 is always the filename root record
        let file_record = pm.keys.get(1)?;
        if path.is_empty() {
            return Some(1);
        }

        // expand file record's children as the search starting set
        let top_level = self.children_of(pm, file_record);
        self.find_in(pm, &segments, &top_level)
    }

    /// Recursively walks the Trie to find the record matching `segments`.
    fn find_in(&self, pm: &ParsedManifest, segments: &[&str], candidates: &[u16]) -> Option<u16> {
        let target = segments[0];
        let rest   = &segments[1..];

        for &idx in candidates {
            let record = pm.keys.get(idx)?;

            // skip meta keys
            if bit::get(record, bit::OFFSET_ROOT, bit::MASK_ROOT) != bit::ROOT_NULL {
                continue;
            }

            let dyn_idx = bit::get(record, bit::OFFSET_DYNAMIC, bit::MASK_DYNAMIC) as u16;
            if pm.dynamic.get(dyn_idx)? != target {
                continue;
            }

            // matched
            if rest.is_empty() {
                return Some(idx);
            }

            // descend
            let next = self.children_of(pm, record);
            if next.is_empty() {
                return None;
            }
            return self.find_in(pm, rest, &next);
        }

        None
    }

    /// Returns the direct field-key children indices of a record.
    fn children_of(&self, pm: &ParsedManifest, record: u64) -> Vec<u16> {
        let child_idx = bit::get(record, bit::OFFSET_CHILD, bit::MASK_CHILD) as u16;
        if child_idx == 0 {
            return vec![];
        }
        let has_children = bit::get(record, bit::OFFSET_HAS_CHILDREN, bit::MASK_HAS_CHILDREN);
        if has_children == 1 {
            pm.children_map.get(child_idx)
                .map(|s| s.to_vec())
                .unwrap_or_default()
        } else {
            vec![child_idx]
        }
    }

    pub fn get_missing_keys(&self) -> &[String] {
        &self.missing_keys
    }

    pub fn clear_missing_keys(&mut self) {
        self.missing_keys.clear();
    }
}
