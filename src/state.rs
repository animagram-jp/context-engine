use serde_json::Value;
use std::collections::{HashMap, HashSet};
use crate::manifest::Manifest;
use crate::common::pool::{StateValueList, STATE_OFFSET_KEY, STATE_MASK_KEY};
use crate::common::bit;
use crate::store::Store;
use crate::load::Load;
use crate::ports::provided::StateError;

pub struct State<'a> {
    manifest: Manifest,
    state_values: StateValueList,
    store: Store<'a>,
    load: Load<'a>,
    max_recursion: usize,
    called_keys: HashSet<String>,
}

impl<'a> State<'a> {
    /// Creates a new State with the given manifest directory and load handler.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::State;
    /// use state_engine::load::Load;
    ///
    /// let state = State::new("./examples/manifest", Load::new());
    /// ```
    pub fn new(manifest_dir: &str, load: Load<'a>) -> Self {
        Self {
            manifest: Manifest::new(manifest_dir),
            state_values: StateValueList::new(),
            store: Store::new(),
            load,
            max_recursion: 20,
            called_keys: HashSet::new(),
        }
    }

    pub fn with_in_memory(mut self, client: &'a mut dyn crate::ports::required::InMemoryClient) -> Self {
        self.store = self.store.with_in_memory(client);
        self
    }

    pub fn with_kvs_client(mut self, client: &'a mut dyn crate::ports::required::KVSClient) -> Self {
        self.store = self.store.with_kvs_client(client);
        self
    }

    /// Splits "file.path" into ("file", "path").
    fn split_key<'k>(key: &'k str) -> (&'k str, &'k str) {
        match key.find('.') {
            Some(pos) => (&key[..pos], &key[pos + 1..]),
            None => (key, ""),
        }
    }

    /// Resolves a yaml value record to a String (for use in store/load config keys).
    fn resolve_value_to_string(&mut self, value_idx: u16) -> Option<String> {
        crate::fn_log!("State", "resolve_value_to_string", &value_idx.to_string());
        let vo = self.manifest.values.get(value_idx)?;

        let is_template = bit::get(vo[0], bit::VO_OFFSET_IS_TEMPLATE, bit::VO_MASK_IS_TEMPLATE) == 1;

        const TOKEN_OFFSETS: [(u32, u32); 6] = [
            (bit::VO_OFFSET_T0_IS_PATH, bit::VO_OFFSET_T0_DYNAMIC),
            (bit::VO_OFFSET_T1_IS_PATH, bit::VO_OFFSET_T1_DYNAMIC),
            (bit::VO_OFFSET_T2_IS_PATH, bit::VO_OFFSET_T2_DYNAMIC),
            (bit::VO_OFFSET_T3_IS_PATH, bit::VO_OFFSET_T3_DYNAMIC),
            (bit::VO_OFFSET_T4_IS_PATH, bit::VO_OFFSET_T4_DYNAMIC),
            (bit::VO_OFFSET_T5_IS_PATH, bit::VO_OFFSET_T5_DYNAMIC),
        ];

        let mut result = String::new();

        for (i, (off_is_path, off_dynamic)) in TOKEN_OFFSETS.iter().enumerate() {
            let word = if i < 3 { 0 } else { 1 };
            let is_path = bit::get(vo[word], *off_is_path, bit::VO_MASK_IS_PATH) == 1;
            let dyn_idx = bit::get(vo[word], *off_dynamic, bit::VO_MASK_DYNAMIC) as u16;

            if dyn_idx == 0 {
                break;
            }

            if is_path {
                let path_segments = self.manifest.path_map.get(dyn_idx)?.to_vec();
                let path_key: String = path_segments.iter()
                    .filter_map(|&seg_idx| self.manifest.dynamic.get(seg_idx).map(|s| s.to_string()))
                    .collect::<Vec<_>>()
                    .join(".");
                crate::fn_log!("State", "resolve/get", &path_key);
                let resolved = self.get(&path_key).ok().flatten();
                crate::fn_log!("State", "resolve/got", if resolved.is_some() { "Some" } else { "None" });
                let resolved = resolved?;
                let s = match &resolved {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => return None,
                };
                result.push_str(&s);
            } else {
                let s = self.manifest.dynamic.get(dyn_idx)?.to_string();
                result.push_str(&s);
            }

            if !is_template {
                break;
            }
        }

        Some(result)
    }

    /// Builds a store/load config HashMap from a meta record index.
    fn build_config(&mut self, meta_idx: u16) -> Option<HashMap<String, Value>> {
        crate::fn_log!("State", "build_config", &meta_idx.to_string());
        let record = self.manifest.keys.get(meta_idx)?;
        let child_idx = bit::get(record, bit::OFFSET_CHILD, bit::MASK_CHILD) as u16;
        if child_idx == 0 { return None; }
        let has_children = bit::get(record, bit::OFFSET_HAS_CHILDREN, bit::MASK_HAS_CHILDREN);
        let children = if has_children == 1 {
            self.manifest.children_map.get(child_idx)?.to_vec()
        } else {
            vec![child_idx]
        };

        let mut config = HashMap::new();

        for &child_idx in &children {
            let record = match self.manifest.keys.get(child_idx) {
                Some(r) => r,
                None => continue,
            };
            let prop   = bit::get(record, bit::OFFSET_PROP,   bit::MASK_PROP)   as u8;
            let client = bit::get(record, bit::OFFSET_CLIENT, bit::MASK_CLIENT) as u8;
            let is_leaf = bit::get(record, bit::OFFSET_IS_LEAF, bit::MASK_IS_LEAF) == 1;
            let value_idx = if is_leaf {
                bit::get(record, bit::OFFSET_CHILD, bit::MASK_CHILD) as u16
            } else { 0 };

            if client != 0 {
                config.insert("client".to_string(), Value::Number(client.into()));
                continue;
            }

            let prop_name = match prop as u64 {
                bit::PROP_KEY        => "key",
                bit::PROP_CONNECTION => "connection",
                bit::PROP_MAP        => "map",
                bit::PROP_TTL        => "ttl",
                bit::PROP_TABLE      => "table",
                bit::PROP_WHERE      => "where",
                _ => continue,
            };

            if prop_name == "map" {
                if let Some(map_val) = self.build_map_config(child_idx) {
                    config.insert("map".to_string(), map_val);
                }
            } else if value_idx != 0 {
                if let Some(s) = self.resolve_value_to_string(value_idx) {
                    config.insert(prop_name.to_string(), Value::String(s));
                }
            }
        }

        Some(config)
    }

    /// Builds a map config object from a map prop record's children.
    fn build_map_config(&self, map_idx: u16) -> Option<Value> {
        let record = self.manifest.keys.get(map_idx)?;
        let child_idx = bit::get(record, bit::OFFSET_CHILD, bit::MASK_CHILD) as u16;
        if child_idx == 0 { return Some(Value::Object(serde_json::Map::new())); }

        let has_children = bit::get(record, bit::OFFSET_HAS_CHILDREN, bit::MASK_HAS_CHILDREN);
        let children = if has_children == 1 {
            self.manifest.children_map.get(child_idx)?.to_vec()
        } else {
            vec![child_idx]
        };

        let mut map = serde_json::Map::new();
        for &c in &children {
            let child = self.manifest.keys.get(c)?;
            let dyn_idx   = bit::get(child, bit::OFFSET_DYNAMIC, bit::MASK_DYNAMIC) as u16;
            let value_idx = bit::get(child, bit::OFFSET_CHILD,   bit::MASK_CHILD)   as u16;
            let is_path   = bit::get(child, bit::OFFSET_IS_PATH, bit::MASK_IS_PATH) == 1;

            let key_str = if is_path {
                let segs = self.manifest.path_map.get(dyn_idx)?;
                segs.iter()
                    .filter_map(|&s| self.manifest.dynamic.get(s).map(|x| x.to_string()))
                    .collect::<Vec<_>>()
                    .join(".")
            } else {
                self.manifest.dynamic.get(dyn_idx)?.to_string()
            };

            let val_vo = self.manifest.values.get(value_idx)?;
            let col_dyn = bit::get(val_vo[0], bit::VO_OFFSET_T0_DYNAMIC, bit::VO_MASK_DYNAMIC) as u16;
            let col_str = self.manifest.dynamic.get(col_dyn)?.to_string();

            map.insert(key_str, Value::String(col_str));
        }

        Some(Value::Object(map))
    }

    /// Finds a state_values record index by key_index.
    fn find_state_value(&self, key_idx: u16) -> Option<u16> {
        let mut i = 1u16;
        loop {
            let record = self.state_values.get_record(i)?;
            if record == 0 { i += 1; continue; }
            let k = ((record >> STATE_OFFSET_KEY) & (STATE_MASK_KEY as u32)) as u16;
            if k == key_idx { return Some(i); }
            i += 1;
        }
    }

    /// Returns the value for `key`, checking state cache → _store → _load in order.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::State;
    /// use state_engine::load::Load;
    /// use state_engine::InMemoryClient;
    /// use serde_json::{json, Value};
    ///
    /// struct MockInMemory { data: std::collections::HashMap<String, Value> }
    /// impl MockInMemory { fn new() -> Self { Self { data: Default::default() } } }
    /// impl InMemoryClient for MockInMemory {
    ///     fn get(&self, key: &str) -> Option<Value> { self.data.get(key).cloned() }
    ///     fn set(&mut self, key: &str, value: Value) { self.data.insert(key.to_string(), value); }
    ///     fn delete(&mut self, key: &str) -> bool { self.data.remove(key).is_some() }
    /// }
    ///
    /// let mut client = MockInMemory::new();
    /// let mut state = State::new("./examples/manifest", Load::new())
    ///     .with_in_memory(&mut client);
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

        // check state_values cache
        if let Some(sv_idx) = self.find_state_value(key_idx) {
            let val = self.state_values.get_value(sv_idx).cloned();
            self.called_keys.remove(key);
            return Ok(val);
        }

        let meta = self.manifest.get_meta(&file, &path);

        // check if _load client is State (load-only, no store read)
        let has_state_client = meta.load.and_then(|load_idx| {
            self.manifest.keys.get(load_idx)
                .map(|r| bit::get(r, bit::OFFSET_CLIENT, bit::MASK_CLIENT) == bit::CLIENT_STATE)
        }).unwrap_or(false);

        if !has_state_client {
            if let Some(store_idx) = meta.store {
                if let Some(config) = self.build_config(store_idx) {
                    if let Some(value) = self.store.get(&config) {
                        self.state_values.push(key_idx, value.clone());
                        self.called_keys.remove(key);
                        return Ok(Some(value));
                    }
                }
            }
        }

        // try _load
        let result = if let Some(load_idx) = meta.load {
            if let Some(mut config) = self.build_config(load_idx) {
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
                            if let Some(store_config) = self.build_config(store_idx) {
                                let _ = self.store.set(&store_config, loaded.clone(), None);
                            }
                        }
                        self.state_values.push(key_idx, loaded.clone());
                        Ok(Some(loaded))
                    }
                    Err(e) => Err(StateError::LoadFailed(e)),
                }
            } else { Ok(None) }
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
    /// # use state_engine::load::Load;
    /// # use state_engine::InMemoryClient;
    /// # use serde_json::{json, Value};
    /// # struct MockInMemory { data: std::collections::HashMap<String, Value> }
    /// # impl MockInMemory { fn new() -> Self { Self { data: Default::default() } } }
    /// # impl InMemoryClient for MockInMemory {
    /// #     fn get(&self, key: &str) -> Option<Value> { self.data.get(key).cloned() }
    /// #     fn set(&mut self, key: &str, value: Value) { self.data.insert(key.to_string(), value); }
    /// #     fn delete(&mut self, key: &str) -> bool { self.data.remove(key).is_some() }
    /// # }
    /// let mut client = MockInMemory::new();
    /// let mut state = State::new("./examples/manifest", Load::new())
    ///     .with_in_memory(&mut client);
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
            if let Some(config) = self.build_config(store_idx) {
                let ok = self.store.set(&config, value.clone(), ttl);
                if ok {
                    if let Some(sv_idx) = self.find_state_value(key_idx) {
                        self.state_values.update(sv_idx, value);
                    } else {
                        self.state_values.push(key_idx, value);
                    }
                }
                return Ok(ok);
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
    /// # use state_engine::load::Load;
    /// # use state_engine::InMemoryClient;
    /// # use serde_json::{json, Value};
    /// # struct MockInMemory { data: std::collections::HashMap<String, Value> }
    /// # impl MockInMemory { fn new() -> Self { Self { data: Default::default() } } }
    /// # impl InMemoryClient for MockInMemory {
    /// #     fn get(&self, key: &str) -> Option<Value> { self.data.get(key).cloned() }
    /// #     fn set(&mut self, key: &str, value: Value) { self.data.insert(key.to_string(), value); }
    /// #     fn delete(&mut self, key: &str) -> bool { self.data.remove(key).is_some() }
    /// # }
    /// let mut client = MockInMemory::new();
    /// let mut state = State::new("./examples/manifest", Load::new())
    ///     .with_in_memory(&mut client);
    ///
    /// state.set("connection.common", json!({"host": "localhost"}), None).unwrap();
    /// assert!(state.delete("connection.common").unwrap());
    /// // delete後はstoreにデータがなく、_loadも試みるが今回はEnvClientなしのため値なし
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
            if let Some(config) = self.build_config(store_idx) {
                let ok = self.store.delete(&config);
                if ok {
                    if let Some(sv_idx) = self.find_state_value(key_idx) {
                        self.state_values.remove(sv_idx);
                    }
                }
                return Ok(ok);
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
    /// # use state_engine::load::Load;
    /// # use state_engine::InMemoryClient;
    /// # use serde_json::{json, Value};
    /// # struct MockInMemory { data: std::collections::HashMap<String, Value> }
    /// # impl MockInMemory { fn new() -> Self { Self { data: Default::default() } } }
    /// # impl InMemoryClient for MockInMemory {
    /// #     fn get(&self, key: &str) -> Option<Value> { self.data.get(key).cloned() }
    /// #     fn set(&mut self, key: &str, value: Value) { self.data.insert(key.to_string(), value); }
    /// #     fn delete(&mut self, key: &str) -> bool { self.data.remove(key).is_some() }
    /// # }
    /// let mut client = MockInMemory::new();
    /// let mut state = State::new("./examples/manifest", Load::new())
    ///     .with_in_memory(&mut client);
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
            return Ok(!self.state_values.get_value(sv_idx)
                .map_or(true, |v| v.is_null()));
        }

        let meta = self.manifest.get_meta(&file, &path);
        if let Some(store_idx) = meta.store {
            if let Some(config) = self.build_config(store_idx) {
                return Ok(self.store.get(&config).is_some());
            }
        }
        Ok(false)
    }
}
