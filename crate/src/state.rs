use serde_json::Value;
use std::collections::{HashMap, HashSet};
use crate::manifest::Manifest;
use crate::core::fixed_bits;
use crate::core::codec;
use crate::store::Store;
use crate::load::Load;
use crate::ports::provided::StateError;

// state_values layout: Vec<(key_idx: u16, value: Value)>
// index 0 is reserved as null slot
pub struct State<'a> {
    manifest: Manifest,
    state_keys: Vec<u16>,
    state_vals: Vec<Value>,
    store: Store<'a>,
    load: Load<'a>,
    max_recursion: usize,
    called_keys: HashSet<String>,
}

impl<'a> State<'a> {
    /// Creates a new State with the given manifest directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::State;
    ///
    /// let state = State::new("./examples/manifest");
    /// ```
    pub fn new(manifest_dir: &str) -> Self {
        Self {
            manifest: Manifest::new(manifest_dir),
            state_keys: vec![0],
            state_vals: vec![Value::Null],
            store: Store::new(),
            load: Load::new(),
            max_recursion: 20,
            called_keys: HashSet::new(),
        }
    }

    pub fn with_in_memory(mut self, client: &'a dyn crate::ports::required::InMemoryClient) -> Self {
        self.store = self.store.with_in_memory(client);
        self.load = self.load.with_in_memory(client);
        self
    }

    pub fn with_kvs(mut self, client: &'a dyn crate::ports::required::KVSClient) -> Self {
        self.store = self.store.with_kvs(client);
        self.load = self.load.with_kvs(client);
        self
    }

    pub fn with_db(mut self, client: &'a dyn crate::ports::required::DbClient) -> Self {
        self.load = self.load.with_db(client);
        self
    }

    pub fn with_env(mut self, client: &'a dyn crate::ports::required::EnvClient) -> Self {
        self.load = self.load.with_env(client);
        self
    }

    pub fn with_http(mut self, client: &'a dyn crate::ports::required::HttpClient) -> Self {
        self.store = self.store.with_http(client);
        self.load = self.load.with_http(client);
        self
    }

    pub fn with_file(mut self, client: impl crate::ports::required::FileClient + 'static) -> Self {
        self.manifest = self.manifest.with_file(client);
        self
    }

    pub fn with_file_client(mut self, client: &'a dyn crate::ports::required::FileClient) -> Self {
        self.store = self.store.with_file(client);
        self.load = self.load.with_file(client);
        self
    }


    /// Splits "file.path" into ("file", "path").
    fn split_key<'k>(key: &'k str) -> (&'k str, &'k str) {
        match key.find('.') {
            Some(pos) => (&key[..pos], &key[pos + 1..]),
            None => (key, ""),
        }
    }

    /// Resolves a yaml value record to a Value (any type, for connection etc.).
    /// For non-template single placeholder: returns the resolved Value as-is (including Object).
    /// For string-compatible values: delegates to resolve_value_to_string.
    fn resolve_value(&mut self, value_idx: u16) -> Result<Option<Value>, StateError> {
        crate::fn_log!("State", "resolve_value", &value_idx.to_string());
        let vo = match self.manifest.values.get(value_idx as usize).copied() {
            Some(v) => v,
            None => return Ok(None),
        };
        let is_template = fixed_bits::get(vo[0], fixed_bits::V_OFFSET_IS_TEMPLATE, fixed_bits::V_MASK_IS_TEMPLATE) == 1;
        let is_path = fixed_bits::get(vo[0], fixed_bits::V_OFFSET_T0_IS_PATH, fixed_bits::V_MASK_IS_PATH) == 1;
        let dyn_idx = fixed_bits::get(vo[0], fixed_bits::V_OFFSET_T0_DYNAMIC, fixed_bits::V_MASK_DYNAMIC) as u16;

        if is_path && dyn_idx != 0 && !is_template {
            let path_segments = match self.manifest.path_map.get(dyn_idx as usize) {
                Some(s) => s.to_vec(),
                None => return Ok(None),
            };
            let path_key: String = path_segments.iter()
                .filter_map(|&seg_idx| self.manifest.dynamic.get(seg_idx).map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join(".");
            return self.get(&path_key);
        }

        Ok(self.resolve_value_to_string(value_idx)?.map(Value::String))
    }

    /// Resolves a yaml value record to a String (for use in store/load config keys).
    fn resolve_value_to_string(&mut self, value_idx: u16) -> Result<Option<String>, StateError> {
        crate::fn_log!("State", "resolve_value_to_string", &value_idx.to_string());
        let vo = match self.manifest.values.get(value_idx as usize).copied() {
            Some(v) => v,
            None => return Ok(None),
        };

        let is_template = fixed_bits::get(vo[0], fixed_bits::V_OFFSET_IS_TEMPLATE, fixed_bits::V_MASK_IS_TEMPLATE) == 1;

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

            if dyn_idx == 0 {
                break;
            }

            if is_path {
                let path_segments = match self.manifest.path_map.get(dyn_idx as usize) {
                    Some(s) => s.to_vec(),
                    None => return Ok(None),
                };
                let path_key: String = path_segments.iter()
                    .filter_map(|&seg_idx| self.manifest.dynamic.get(seg_idx).map(|s| s.to_string()))
                    .collect::<Vec<_>>()
                    .join(".");
                crate::fn_log!("State", "resolve/get", &path_key);
                let resolved = self.get(&path_key)?;
                crate::fn_log!("State", "resolve/got", if resolved.is_some() { "Some" } else { "None" });
                let resolved = match resolved {
                    Some(v) => v,
                    None => return Ok(None),
                };
                let s = match &resolved {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => return Ok(None),
                };
                result.push_str(&s);
            } else {
                let s = match self.manifest.dynamic.get(dyn_idx) {
                    Some(s) => s.to_string(),
                    None => return Ok(None),
                };
                result.push_str(&s);
            }

            if !is_template {
                break;
            }
        }

        Ok(Some(result))
    }

    /// Builds a store/load config HashMap from a meta record index.
    fn build_config(&mut self, meta_idx: u16) -> Result<Option<HashMap<String, Value>>, StateError> {
        crate::fn_log!("State", "build_config", &meta_idx.to_string());
        let record = match self.manifest.keys.get(meta_idx as usize).copied() {
            Some(r) => r,
            None => return Ok(None),
        };
        let child_idx = fixed_bits::get(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD) as usize;
        if child_idx == 0 { return Ok(None); }
        let has_children = fixed_bits::get(record, fixed_bits::K_OFFSET_HAS_CHILDREN, fixed_bits::K_MASK_HAS_CHILDREN);
        let children = if has_children == 1 {
            match self.manifest.children_map.get(child_idx) {
                Some(c) => c.to_vec(),
                None => return Ok(None),
            }
        } else {
            vec![child_idx as u16]
        };

        let mut config = HashMap::new();

        for &child_idx in &children {
            let record = match self.manifest.keys.get(child_idx as usize).copied() {
                Some(r) => r,
                None => continue,
            };
            let prop   = fixed_bits::get(record, fixed_bits::K_OFFSET_PROP,   fixed_bits::K_MASK_PROP)   as u8;
            let client = fixed_bits::get(record, fixed_bits::K_OFFSET_CLIENT, fixed_bits::K_MASK_CLIENT) as u8;
            let is_leaf = fixed_bits::get(record, fixed_bits::K_OFFSET_IS_LEAF, fixed_bits::K_MASK_IS_LEAF) == 1;
            let value_idx = if is_leaf {
                fixed_bits::get(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD) as u16
            } else { 0 };

            if client != 0 {
                config.insert("client".to_string(), Value::Number(client.into()));
                continue;
            }

            let prop_name = match codec::prop_decode(prop as u64) {
                Some(name) => name,
                None => continue,
            };

            if prop_name == "map" {
                if let Some(map_val) = self.build_map_config(child_idx) {
                    config.insert("map".to_string(), map_val);
                }
            } else if prop_name == "connection" {
                if value_idx != 0 {
                    if let Some(v) = self.resolve_value(value_idx)? {
                        config.insert("connection".to_string(), v);
                    }
                }
            } else if value_idx != 0 {
                if let Some(s) = self.resolve_value_to_string(value_idx)? {
                    config.insert(prop_name.to_string(), Value::String(s));
                }
            }
        }

        Ok(Some(config))
    }

    /// Builds a map config object from a map prop record's children.
    fn build_map_config(&self, map_idx: u16) -> Option<Value> {
        let record = self.manifest.keys.get(map_idx as usize).copied()?;
        let child_idx = fixed_bits::get(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD) as usize;
        if child_idx == 0 { return Some(Value::Object(serde_json::Map::new())); }

        let has_children = fixed_bits::get(record, fixed_bits::K_OFFSET_HAS_CHILDREN, fixed_bits::K_MASK_HAS_CHILDREN);
        let children = if has_children == 1 {
            self.manifest.children_map.get(child_idx)?.to_vec()
        } else {
            vec![child_idx as u16]
        };

        let mut map = serde_json::Map::new();
        for &c in &children {
            let child = self.manifest.keys.get(c as usize).copied()?;
            let dyn_idx   = fixed_bits::get(child, fixed_bits::K_OFFSET_DYNAMIC, fixed_bits::K_MASK_DYNAMIC) as u16;
            let value_idx = fixed_bits::get(child, fixed_bits::K_OFFSET_CHILD,   fixed_bits::K_MASK_CHILD)   as usize;
            let is_path   = fixed_bits::get(child, fixed_bits::K_OFFSET_IS_PATH, fixed_bits::K_MASK_IS_PATH) == 1;

            let key_str = if is_path {
                let segs = self.manifest.path_map.get(dyn_idx as usize)?;
                segs.iter()
                    .filter_map(|&s| self.manifest.dynamic.get(s).map(|x| x.to_string()))
                    .collect::<Vec<_>>()
                    .join(".")
            } else {
                self.manifest.dynamic.get(dyn_idx)?.to_string()
            };

            let val_vo = self.manifest.values.get(value_idx).copied()?;
            let col_dyn = fixed_bits::get(val_vo[0], fixed_bits::V_OFFSET_T0_DYNAMIC, fixed_bits::V_MASK_DYNAMIC) as u16;
            let col_str = self.manifest.dynamic.get(col_dyn)?.to_string();

            map.insert(key_str, Value::String(col_str));
        }

        Some(Value::Object(map))
    }

    /// Finds a state_vals index by key_index (skips null slot at 0).
    fn find_state_value(&self, key_idx: u16) -> Option<usize> {
        self.state_keys.iter().skip(1).position(|&k| k == key_idx).map(|p| p + 1)
    }

    /// Returns the value for `key`, checking state cache → _store → _load in order.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::State;
    /// use state_engine::InMemoryClient;
    /// use serde_json::{json, Value};
    ///
    /// struct MockInMemory { data: std::sync::Mutex<std::collections::HashMap<String, Value>> }
    /// impl MockInMemory { fn new() -> Self { Self { data: Default::default() } } }
    /// impl InMemoryClient for MockInMemory {
    ///     fn get(&self, key: &str) -> Option<Value> { self.data.lock().unwrap().get(key).cloned() }
    ///     fn set(&self, key: &str, value: Value) -> bool { self.data.lock().unwrap().insert(key.to_string(), value); true }
    ///     fn delete(&self, key: &str) -> bool { self.data.lock().unwrap().remove(key).is_some() }
    /// }
    ///
    /// let client = MockInMemory::new();
    /// let mut state = State::new("./examples/manifest")
    ///     .with_in_memory(&client);
    ///
    /// // set then get
    /// state.set("connection.common", json!({"host": "localhost"}), None).unwrap();
    /// assert!(state.get("connection.common").unwrap().is_some());
    /// ```
    pub fn get(&mut self, key: &str) -> Result<Option<Value>, StateError> {
        crate::fn_log!("State", "get", key);
        if self.called_keys.len() >= self.max_recursion {
            return Err(StateError::RecursionLimitExceeded);
        }
        if self.called_keys.contains(&key.to_string()) {
            return Err(StateError::RecursionLimitExceeded);
        }

        self.called_keys.insert(key.to_string());

        let (file, path) = Self::split_key(key);
        let file = file.to_string();
        let path = path.to_string();

        if let Err(e) = self.manifest.load(&file) {
            self.called_keys.remove(key);
            return Err(StateError::ManifestLoadFailed(e.to_string()));
        }

        let key_idx = match self.manifest.find(&file, &path) {
            Some(idx) => idx,
            None => {
                self.called_keys.remove(key);
                return Err(StateError::KeyNotFound(key.to_string()));
            }
        };

        // check state cache
        if let Some(sv_idx) = self.find_state_value(key_idx) {
            let val = self.state_vals.get(sv_idx).cloned();
            self.called_keys.remove(key);
            return Ok(val);
        }

        let meta = self.manifest.get_meta(&file, &path);

        // check if _load client is State (load-only, no store read)
        let has_state_client = meta.load.and_then(|load_idx| {
            self.manifest.keys.get(load_idx as usize).copied()
                .map(|r| fixed_bits::get(r, fixed_bits::K_OFFSET_CLIENT, fixed_bits::K_MASK_CLIENT) == fixed_bits::CLIENT_STATE)
        }).unwrap_or(false);

        if !has_state_client {
            if let Some(store_idx) = meta.store {
                match self.build_config(store_idx) {
                    Ok(Some(config)) => {
                        if let Some(value) = self.store.get(&config) {
                            self.state_keys.push(key_idx);
                            self.state_vals.push(value.clone());
                            self.called_keys.remove(key);
                            return Ok(Some(value));
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        self.called_keys.remove(key);
                        return Err(e);
                    }
                }
            }
        }

        // try _load
        let result = if let Some(load_idx) = meta.load {
            match self.build_config(load_idx) {
                Ok(Some(mut config)) => {
                    if !config.contains_key("client") {
                        self.called_keys.remove(key);
                        return Ok(None);
                    }

                    // unqualify map keys for Load
                    if let Some(Value::Object(map_obj)) = config.get("map").cloned() {
                        let mut unqualified = serde_json::Map::new();
                        for (qk, v) in map_obj {
                            let field = qk.rfind('.').map_or(qk.as_str(), |p| &qk[p+1..]);
                            unqualified.insert(field.to_string(), v);
                        }
                        config.insert("map".to_string(), Value::Object(unqualified));
                    }

                    match self.load.handle(&config) {
                        Ok(loaded) => {
                            if let Some(store_idx) = meta.store {
                                match self.build_config(store_idx) {
                                    Ok(Some(store_config)) => {
                                        if self.store.set(&store_config, loaded.clone(), None).unwrap_or(false) {
                                            self.state_keys.push(key_idx);
                                            self.state_vals.push(loaded.clone());
                                        }
                                    }
                                    Ok(None) => {
                                        self.state_keys.push(key_idx);
                                        self.state_vals.push(loaded.clone());
                                    }
                                    Err(_) => {
                                        // write-through cache failure is non-fatal
                                    }
                                }
                            } else {
                                self.state_keys.push(key_idx);
                                self.state_vals.push(loaded.clone());
                            }
                            Ok(Some(loaded))
                        }
                        Err(e) => Err(StateError::LoadFailed(e)),
                    }
                }
                Ok(None) => Ok(None),
                Err(e) => Err(e),
            }
        } else { Ok(None) };

        self.called_keys.remove(key);
        result
    }

    /// Writes `value` to the _store backend for `key`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use state_engine::State;
    /// # use state_engine::InMemoryClient;
    /// # use serde_json::{json, Value};
    /// # struct MockInMemory { data: std::sync::Mutex<std::collections::HashMap<String, Value>> }
    /// # impl MockInMemory { fn new() -> Self { Self { data: Default::default() } } }
    /// # impl InMemoryClient for MockInMemory {
    /// #     fn get(&self, key: &str) -> Option<Value> { self.data.lock().unwrap().get(key).cloned() }
    /// #     fn set(&self, key: &str, value: Value) -> bool { self.data.lock().unwrap().insert(key.to_string(), value); true }
    /// #     fn delete(&self, key: &str) -> bool { self.data.lock().unwrap().remove(key).is_some() }
    /// # }
    /// let client = MockInMemory::new();
    /// let mut state = State::new("./examples/manifest")
    ///     .with_in_memory(&client);
    ///
    /// assert!(state.set("connection.common", json!({"host": "localhost"}), None).unwrap());
    /// ```
    pub fn set(&mut self, key: &str, value: Value, ttl: Option<u64>) -> Result<bool, StateError> {
        crate::fn_log!("State", "set", key);
        let (file, path) = Self::split_key(key);
        let file = file.to_string();
        let path = path.to_string();

        if let Err(e) = self.manifest.load(&file) {
            return Err(StateError::ManifestLoadFailed(e.to_string()));
        }

        let key_idx = match self.manifest.find(&file, &path) {
            Some(idx) => idx,
            None => return Err(StateError::KeyNotFound(key.to_string())),
        };

        let meta = self.manifest.get_meta(&file, &path);

        if let Some(store_idx) = meta.store {
            match self.build_config(store_idx)? {
                Some(config) => {
                    return match self.store.set(&config, value.clone(), ttl) {
                        Ok(ok) => {
                            if ok {
                                if let Some(sv_idx) = self.find_state_value(key_idx) {
                                    self.state_vals[sv_idx] = value;
                                } else {
                                    self.state_keys.push(key_idx);
                                    self.state_vals.push(value);
                                }
                            }
                            Ok(ok)
                        }
                        Err(e) => Err(StateError::StoreFailed(e)),
                    };
                }
                None => {}
            }
        }
        Ok(false)
    }

    /// Removes the value for `key` from the _store backend.
    ///
    /// # Examples
    ///
    /// ```
    /// # use state_engine::State;
    /// # use state_engine::InMemoryClient;
    /// # use serde_json::{json, Value};
    /// # struct MockInMemory { data: std::sync::Mutex<std::collections::HashMap<String, Value>> }
    /// # impl MockInMemory { fn new() -> Self { Self { data: Default::default() } } }
    /// # impl InMemoryClient for MockInMemory {
    /// #     fn get(&self, key: &str) -> Option<Value> { self.data.lock().unwrap().get(key).cloned() }
    /// #     fn set(&self, key: &str, value: Value) -> bool { self.data.lock().unwrap().insert(key.to_string(), value); true }
    /// #     fn delete(&self, key: &str) -> bool { self.data.lock().unwrap().remove(key).is_some() }
    /// # }
    /// let client = MockInMemory::new();
    /// let mut state = State::new("./examples/manifest")
    ///     .with_in_memory(&client);
    ///
    /// state.set("connection.common", json!({"host": "localhost"}), None).unwrap();
    /// assert!(state.delete("connection.common").unwrap());
    /// // after delete, store has no data; _load is attempted but EnvClient is not configured here
    /// assert!(state.get("connection.common").is_err() || state.get("connection.common").unwrap().is_none());
    /// ```
    pub fn delete(&mut self, key: &str) -> Result<bool, StateError> {
        crate::fn_log!("State", "delete", key);
        let (file, path) = Self::split_key(key);
        let file = file.to_string();
        let path = path.to_string();

        if let Err(e) = self.manifest.load(&file) {
            return Err(StateError::ManifestLoadFailed(e.to_string()));
        }

        let key_idx = match self.manifest.find(&file, &path) {
            Some(idx) => idx,
            None => return Err(StateError::KeyNotFound(key.to_string())),
        };

        let meta = self.manifest.get_meta(&file, &path);

        if let Some(store_idx) = meta.store {
            match self.build_config(store_idx)? {
                Some(config) => {
                    return match self.store.delete(&config) {
                        Ok(ok) => {
                            if ok {
                                if let Some(sv_idx) = self.find_state_value(key_idx) {
                                    self.state_keys[sv_idx] = 0;
                                    self.state_vals[sv_idx] = Value::Null;
                                }
                            }
                            Ok(ok)
                        }
                        Err(e) => Err(StateError::StoreFailed(e)),
                    };
                }
                None => {}
            }
        }
        Ok(false)
    }

    /// Returns `true` if a value exists for `key` in state cache or _store.
    /// Does not trigger _load.
    ///
    /// # Examples
    ///
    /// ```
    /// # use state_engine::State;
    /// # use state_engine::InMemoryClient;
    /// # use serde_json::{json, Value};
    /// # struct MockInMemory { data: std::sync::Mutex<std::collections::HashMap<String, Value>> }
    /// # impl MockInMemory { fn new() -> Self { Self { data: Default::default() } } }
    /// # impl InMemoryClient for MockInMemory {
    /// #     fn get(&self, key: &str) -> Option<Value> { self.data.lock().unwrap().get(key).cloned() }
    /// #     fn set(&self, key: &str, value: Value) -> bool { self.data.lock().unwrap().insert(key.to_string(), value); true }
    /// #     fn delete(&self, key: &str) -> bool { self.data.lock().unwrap().remove(key).is_some() }
    /// # }
    /// let client = MockInMemory::new();
    /// let mut state = State::new("./examples/manifest")
    ///     .with_in_memory(&client);
    ///
    /// assert!(!state.exists("connection.common").unwrap());
    /// state.set("connection.common", json!({"host": "localhost"}), None).unwrap();
    /// assert!(state.exists("connection.common").unwrap());
    /// ```
    pub fn exists(&mut self, key: &str) -> Result<bool, StateError> {
        crate::fn_log!("State", "exists", key);
        let (file, path) = Self::split_key(key);
        let file = file.to_string();
        let path = path.to_string();

        if let Err(e) = self.manifest.load(&file) {
            return Err(StateError::ManifestLoadFailed(e.to_string()));
        }

        let key_idx = match self.manifest.find(&file, &path) {
            Some(idx) => idx,
            None => return Err(StateError::KeyNotFound(key.to_string())),
        };

        if let Some(sv_idx) = self.find_state_value(key_idx) {
            return Ok(!self.state_vals.get(sv_idx).map_or(true, |v| v.is_null()));
        }

        let meta = self.manifest.get_meta(&file, &path);
        if let Some(store_idx) = meta.store {
            if let Some(config) = self.build_config(store_idx)? {
                return Ok(self.store.get(&config).is_some());
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::required::{KVSClient, DbClient, EnvClient, FileClient};
    use serde_json::Value;
    use std::collections::HashMap;

    struct StubKVS;
    impl KVSClient for StubKVS {
        fn get(&self, _: &str) -> Option<String> { None }
        fn set(&self, _: &str, _: String, _: Option<u64>) -> bool { false }
        fn delete(&self, _: &str) -> bool { false }
    }

    struct StubDb;
    impl DbClient for StubDb {
        fn get(&self, _: &Value, _: &str, _: &[&str], _: Option<&str>) -> Option<Vec<HashMap<String, Value>>> { None }
        fn set(&self, _: &Value, _: &str, _: &HashMap<String, Value>, _: Option<&str>) -> bool { false }
        fn delete(&self, _: &Value, _: &str, _: Option<&str>) -> bool { false }
    }

    struct StubEnv;
    impl EnvClient for StubEnv {
        fn get(&self, _: &str) -> Option<String> { None }
        fn set(&self, _: &str, _: String) -> bool { false }
        fn delete(&self, _: &str) -> bool { false }
    }

    struct StubFile;
    impl FileClient for StubFile {
        fn get(&self, _: &str) -> Option<String> { None }
        fn set(&self, _: &str, _: String) -> bool { false }
        fn delete(&self, _: &str) -> bool { false }
    }

    struct StubHttp;
    impl crate::ports::required::HttpClient for StubHttp {
        fn get(&self, _: &str, _: Option<&HashMap<String, String>>) -> Option<Value> { None }
        fn set(&self, _: &str, _: Value, _: Option<&HashMap<String, String>>) -> bool { false }
        fn delete(&self, _: &str, _: Option<&HashMap<String, String>>) -> bool { false }
    }

    #[test]
    fn test_with_clients_build() {
        let kvs  = StubKVS;
        let db   = StubDb;
        let env  = StubEnv;
        let http = StubHttp;

        // each builder returns Self without panic — wiring is correct
        let _ = State::new("./examples/manifest").with_kvs(&kvs);
        let _ = State::new("./examples/manifest").with_db(&db);
        let _ = State::new("./examples/manifest").with_env(&env);
        let _ = State::new("./examples/manifest").with_http(&http);
        let _ = State::new("./examples/manifest").with_file(StubFile);
    }
}
