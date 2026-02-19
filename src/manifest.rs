use crate::ports::provided;
use crate::common::{DotString, DotMapAccessor};
use crate::method_log;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct Manifest {
    manifest_dir: PathBuf,
    cache: HashMap<String, Value>,
    missing_keys: Vec<String>,
}

impl Manifest {
    pub fn new(manifest_dir: &str) -> Self {
        Self {
            manifest_dir: PathBuf::from(manifest_dir),
            cache: HashMap::new(),
            missing_keys: Vec::new(),
        }
    }

    /// # Examples
    ///
    /// ```
    /// use state_engine::Manifest;
    /// use serde_json::json;
    ///
    /// let mut manifest = Manifest::new("./examples/manifest");
    ///
    /// // field keys are returned, meta keys excluded
    /// let user = manifest.get("cache.user", None);
    /// assert!(user.get("id").is_some());
    /// assert!(user.get("_load").is_none());
    ///
    /// // leaf field key returns {} (presence indicator)
    /// let host = manifest.get("connection.common.host", None);
    /// assert_eq!(host, json!({}));
    ///
    /// // miss returns default
    /// let missing = manifest.get("cache.never", Some(json!("default")));
    /// assert_eq!(missing, json!("default"));
    /// ```
    pub fn get(&mut self, key: &str, default: Option<Value>) -> Value {
        method_log!("Manifest", "get", key);

        let mut parts = key.splitn(2, '.');
        let file = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("").to_string();

        if let Err(_e) = self.load_file(&file) {
            self.missing_keys.push(key.to_string());
            return default.unwrap_or(Value::Null);
        }

        let value = if path.is_empty() {
            self.cache.get(&file).cloned()
        } else {
            self.cache.get(&file).and_then(|data| {
                let mut accessor = DotMapAccessor::new();
                let path_dot = DotString::new(&path);
                accessor.get(data, &path_dot).cloned()
            })
        };

        match value {
            Some(v) => self.remove_meta(&v),
            None => {
                self.missing_keys.push(key.to_string());
                default.unwrap_or(Value::Null)
            }
        }
    }

    /// # Examples
    ///
    /// ```
    /// use state_engine::Manifest;
    ///
    /// let mut manifest = Manifest::new("./examples/manifest");
    ///
    /// // collects meta keys from root to node, child overrides parent
    /// let meta = manifest.get_meta("cache.user");
    /// assert!(meta.contains_key("_load"));
    /// assert!(meta.contains_key("_store"));
    ///
    /// // _load.map keys are qualified to absolute paths
    /// let load = meta["_load"].as_object().unwrap();
    /// let map = load["map"].as_object().unwrap();
    /// assert!(map.keys().next().unwrap().starts_with("cache.user."));
    /// ```
    pub fn get_meta(&mut self, key: &str) -> HashMap<String, Value> {
        method_log!("Manifest", "get_meta", key);

        use regex::Regex;

        let mut parts = key.splitn(2, '.');
        let file = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("").to_string();

        if self.load_file(&file).is_err() {
            self.missing_keys.push(key.to_string());
            return HashMap::new();
        }

        let Some(root) = self.cache.get(&file) else {
            return HashMap::new();
        };

        // ルートから指定Nodeまでのパス上のすべてのNodeを収集
        let mut nodes = vec![root.clone()];
        let dot_string = DotString::new(&path);

        if !path.is_empty() {
            let mut accessor = DotMapAccessor::new();
            let mut current = root;
            for segment in dot_string.iter() {
                let segment_dot = DotString::new(segment);
                current = match accessor.get(current, &segment_dot) {
                    Some(node) => node,
                    None => {
                        self.missing_keys.push(key.to_string());
                        return HashMap::new();
                    }
                };
                nodes.push(current.clone());
            }
        }

        let mut meta: HashMap<String, Value> = HashMap::new();

        // 完全修飾用のパス情報を記録
        let mut meta_paths: HashMap<String, String> = HashMap::new();

        // すべてのNodeから_始まりのキーを抽出してメタデータを構築
        for (depth, node) in nodes.iter().enumerate() {
            let Value::Object(obj) = node else {
                continue;
            };

            // この node のパスを構築
            let node_path = if depth == 0 {
                file.clone()
            } else if depth > dot_string.len() {
                file.clone()
            } else {
                let node_segments = &dot_string[..depth];
                let joined = node_segments.iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                if joined.is_empty() {
                    file.clone()
                } else {
                    format!("{}.{}", file, joined)
                }
            };

            // meta keys を収集
            for (k, v) in obj {
                if k.starts_with('_') {
                    // meta key のマージ/上書きルール
                    if let Some(existing_value) = meta.get(k).cloned() {
                        if existing_value.is_object() && v.is_object() {
                            if let (Value::Object(existing_obj), Value::Object(new_obj)) = (&existing_value, v) {
                                let mut merged = existing_obj.clone();
                                for (child_key, child_value) in new_obj {
                                    merged.insert(child_key.clone(), child_value.clone());
                                }
                                meta.insert(k.clone(), Value::Object(merged));
                            }
                        } else {
                            meta.insert(k.clone(), v.clone());
                        }
                    } else {
                        meta.insert(k.clone(), v.clone());
                    }

                    // パス情報を記録: _load.map のパスを記録
                    if k == "_load" {
                        if let Value::Object(load_obj) = v {
                            if load_obj.contains_key("map") {
                                meta_paths.insert("_load.map".to_string(), node_path.clone());
                            }
                        }
                    }
                }
            }
        }

        // _load.map のキーを完全修飾
        if let Some(map_parent) = meta_paths.get("_load.map") {
            if let Some(Value::Object(load_obj)) = meta.get_mut("_load") {
                if let Some(Value::Object(map_obj)) = load_obj.get("map").cloned() {
                    let mut qualified_map = serde_json::Map::new();
                    for (relative_key, db_column) in map_obj {
                        qualified_map.insert(format!("{}.{}", map_parent, relative_key), db_column);
                    }
                    load_obj.insert("map".to_string(), Value::Object(qualified_map));
                }
            }
        }

        // placeholder を完全修飾
        self.load_file(&file).ok();
        let re = Regex::new(r"\$\{([^}]+)\}").unwrap();
        let parent_path = path.rfind('.').map_or(String::new(), |pos| path[..pos].to_string());

        for (_meta_key, meta_value) in meta.iter_mut() {
            self.qualify_value(meta_value, &re, &file, &parent_path);
        }

        meta
    }

    /// Recursively qualifies relative placeholders in a Value to absolute paths.
    fn qualify_value(
        &mut self,
        value: &mut Value,
        re: &regex::Regex,
        owner_file: &str,
        parent_path: &str,
    ) {
        match value {
            Value::String(s) => {
                *s = re.replace_all(s, |caps: &regex::Captures| {
                    let placeholder = &caps[1];

                    // owner file内に存在するか
                    if let Some(owner_data) = self.cache.get(owner_file) {
                        let placeholder_dot = DotString::new(placeholder);
                        if DotMapAccessor::has(owner_data, &placeholder_dot) {
                            return caps[0].to_string();
                        }
                    }

                    // 別ファイル参照か
                    let mut ph_parts = placeholder.splitn(2, '.');
                    let ph_file = ph_parts.next().unwrap_or("").to_string();
                    let ph_path = ph_parts.next().unwrap_or("").to_string();
                    self.load_file(&ph_file).ok();

                    if let Some(ph_data) = self.cache.get(&ph_file) {
                        if let Some(obj) = ph_data.as_object() {
                            let ph_path_dot = DotString::new(&ph_path);
                            if !obj.is_empty() && (ph_path.is_empty() || DotMapAccessor::has(ph_data, &ph_path_dot)) {
                                return caps[0].to_string();
                            }
                        }
                    }

                    // 相対パス → 完全修飾
                    if parent_path.is_empty() {
                        caps[0].to_string()
                    } else {
                        format!("${{{}.{}.{}}}", owner_file, parent_path, placeholder)
                    }
                }).to_string();
            }
            Value::Object(obj) => {
                for (_k, v) in obj.iter_mut() {
                    self.qualify_value(v, re, owner_file, parent_path);
                }
            }
            Value::Array(arr) => {
                for v in arr.iter_mut() {
                    self.qualify_value(v, re, owner_file, parent_path);
                }
            }
            _ => {}
        }
    }

    /// Loads a YAML file into cache; returns error if both .yml and .yaml exist.
    fn load_file(&mut self, file: &str) -> Result<(), String> {
        if self.cache.contains_key(file) {
            return Ok(());
        }

        let yml_path = self.manifest_dir.join(format!("{}.yml", file));
        let yaml_path = self.manifest_dir.join(format!("{}.yaml", file));

        let yml_exists = yml_path.exists();
        let yaml_exists = yaml_path.exists();

        if yml_exists && yaml_exists {
            self.cache.insert(file.to_string(), Value::Object(serde_json::Map::new()));
            return Err(format!(
                "Ambiguous file: both '{}.yml' and '{}.yaml' exist. Please use only one extension.",
                file, file
            ));
        }

        let file_path = if yml_exists {
            yml_path
        } else if yaml_exists {
            yaml_path
        } else {
            self.cache.insert(file.to_string(), Value::Object(serde_json::Map::new()));
            return Err(format!("File not found: '{}.yml' or '{}.yaml'", file, file));
        };

        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&content)
            .map_err(|e| format!("Failed to parse YAML: {}", e))?;

        let json_value = serde_json::to_value(&yaml)
            .unwrap_or_else(|_| Value::Object(serde_json::Map::new()));
        self.cache.insert(file.to_string(), json_value);

        Ok(())
    }

    /// Recursively strips meta keys from a Value, preserving empty objects (field key presence).
    fn remove_meta(&self, value: &Value) -> Value {
        match value {
            Value::Object(obj) => {
                let filtered: serde_json::Map<String, Value> = obj
                    .iter()
                    .filter(|(k, _)| !k.starts_with('_'))
                    .map(|(k, v)| (k.clone(), self.remove_meta(v)))
                    .collect();
                Value::Object(filtered)
            }
            Value::Array(arr) => {
                let filtered: Vec<Value> = arr.iter().map(|v| self.remove_meta(v)).collect();
                // Empty array (all elements were meta keys or became null) should be treated as null
                if filtered.is_empty() {
                    Value::Null
                } else {
                    Value::Array(filtered)
                }
            }
            _ => value.clone(),
        }
    }

    /// # Examples
    ///
    /// ```
    /// use state_engine::Manifest;
    ///
    /// let mut manifest = Manifest::new("./nonexistent");
    /// manifest.get("missing.key", None);
    /// assert_eq!(manifest.get_missing_keys(), &["missing.key"]);
    /// ```
    pub fn get_missing_keys(&self) -> &[String] {
        &self.missing_keys
    }

    /// # Examples
    ///
    /// ```
    /// use state_engine::Manifest;
    ///
    /// let mut manifest = Manifest::new("./nonexistent");
    /// manifest.get("missing.key", None);
    /// manifest.clear_missing_keys();
    /// assert!(manifest.get_missing_keys().is_empty());
    /// ```
    pub fn clear_missing_keys(&mut self) {
        self.missing_keys.clear();
    }

    /// # Examples
    ///
    /// ```
    /// use state_engine::Manifest;
    /// use state_engine::common::DotString;
    /// use serde_json::json;
    ///
    /// let mut manifest = Manifest::new("./examples/manifest");
    ///
    /// // static field values returned, meta keys and nulls excluded
    /// let key = DotString::new("connection.common");
    /// let value = manifest.get_value(&key);
    /// assert_eq!(value.get("tag"), Some(&json!("common")));
    ///
    /// // miss records to missing_keys
    /// manifest.clear_missing_keys();
    /// let key = DotString::new("cache.never");
    /// manifest.get_value(&key);
    /// assert!(!manifest.get_missing_keys().is_empty());
    /// ```
    pub fn get_value(&mut self, key: &DotString) -> Value {
        method_log!("Manifest", "get_value", key.as_str());
        let key_str = key.as_str();
        let mut parts = key_str.splitn(2, '.');
        let file = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("").to_string();

        if let Err(_e) = self.load_file(&file) {
            self.missing_keys.push(key_str.to_string());
            return Value::Null;
        }

        let value = if path.is_empty() {
            self.cache.get(&file).cloned()
        } else {
            self.cache.get(&file).and_then(|data| {
                let mut accessor = DotMapAccessor::new();
                let path_dot = DotString::new(&path);
                accessor.get(data, &path_dot).cloned()
            })
        };

        match value {
            Some(v) => self.remove_meta_and_nulls(&v),
            None => {
                self.missing_keys.push(key_str.to_string());
                Value::Null
            }
        }
    }

    /// Recursively strips meta keys and nulls; empty objects collapse to Null.
    fn remove_meta_and_nulls(&self, value: &Value) -> Value {
        match value {
            Value::Object(obj) => {
                let filtered: serde_json::Map<String, Value> = obj
                    .iter()
                    .filter(|(k, _)| !k.starts_with('_'))
                    .filter_map(|(k, v)| {
                        let cleaned = self.remove_meta_and_nulls(v);
                        if cleaned.is_null() {
                            None
                        } else {
                            Some((k.clone(), cleaned))
                        }
                    })
                    .collect();

                if filtered.is_empty() {
                    Value::Null
                } else {
                    Value::Object(filtered)
                }
            }
            Value::Array(arr) => {
                let filtered: Vec<Value> = arr
                    .iter()
                    .map(|v| self.remove_meta_and_nulls(v))
                    .filter(|v| !v.is_null())
                    .collect();

                if filtered.is_empty() {
                    Value::Null
                } else {
                    Value::Array(filtered)
                }
            }
            _ => value.clone(),
        }
    }
}

// Provided::Manifest trait の実装
impl provided::Manifest for Manifest {
    fn get(&mut self, key: &str, default: Option<Value>) -> Value {
        Manifest::get(self, key, default)
    }

    fn get_meta(&mut self, key: &str) -> HashMap<String, Value> {
        Manifest::get_meta(self, key)
    }

    fn get_missing_keys(&self) -> &[String] {
        Manifest::get_missing_keys(self)
    }

    fn clear_missing_keys(&mut self) {
        Manifest::clear_missing_keys(self)
    }

    fn get_value(&mut self, key: &DotString) -> Value {
        Manifest::get_value(self, key)
    }
}

