use serde_json::Value;
use std::collections::HashMap;
use crate::common::manifest::ManifestStore;
use crate::common::pool::{StateValueList, STATE_OFFSET_KEY, STATE_MASK_KEY};
use crate::common::bit;
use crate::store::Store;
use crate::load::Load;

pub struct State<'a> {
    manifest: ManifestStore,
    state_values: StateValueList,
    store: Store<'a>,
    load: Load<'a>,
    max_recursion: usize,
    called_keys: Vec<String>,
}

impl<'a> State<'a> {
    /// Creates a new State with the given manifest directory and load handler.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::common::state::State;
    /// use state_engine::load::Load;
    ///
    /// let state = State::new("./examples/manifest", Load::new());
    /// ```
    pub fn new(manifest_dir: &str, load: Load<'a>) -> Self {
        Self {
            manifest: ManifestStore::new(manifest_dir),
            state_values: StateValueList::new(),
            store: Store::new(),
            load,
            max_recursion: 20,
            called_keys: Vec::new(),
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
    /// Follows path references via state values if needed.
    fn resolve_value_to_string(&mut self, file: &str, value_idx: u16) -> Option<String> {
        crate::fn_log!("State", "resolve_value_to_string", file, &value_idx.to_string());
        let pm = self.manifest.get_file(file)?;
        let vo = pm.values.get(value_idx)?;

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
                // resolve path via state
                let path_segments = {
                    let pm = self.manifest.get_file(file)?;
                    pm.path_map.get(dyn_idx)?.to_vec()
                };
                let qualified: Vec<String> = {
                    let pm = self.manifest.get_file(file)?;
                    path_segments.iter()
                        .filter_map(|&seg_idx| pm.dynamic.get(seg_idx).map(|s| s.to_string()))
                        .collect()
                };
                let path_key = qualified.join(".");
                crate::fn_log!("State", "resolve/get", &path_key);
                let resolved = self.get(&path_key);
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
                let pm = self.manifest.get_file(file)?;
                let s = pm.dynamic.get(dyn_idx)?.to_string();
                result.push_str(&s);
            }

            if !is_template {
                break;
            }
        }

        Some(result)
    }

    /// Builds a store/load config HashMap from a meta record index.
    fn build_config(&mut self, file: &str, meta_idx: u16) -> Option<HashMap<String, Value>> {
        crate::fn_log!("State", "build_config", file, &meta_idx.to_string());
        let children = {
            let pm = self.manifest.get_file(file)?;
            let record = pm.keys.get(meta_idx)?;
            let child_idx = bit::get(record, bit::OFFSET_CHILD, bit::MASK_CHILD) as u16;
            if child_idx == 0 { return None; }
            let has_children = bit::get(record, bit::OFFSET_HAS_CHILDREN, bit::MASK_HAS_CHILDREN);
            if has_children == 1 {
                pm.children_map.get(child_idx)?.to_vec()
            } else {
                vec![child_idx]
            }
        };

        let mut config = HashMap::new();

        for &child_idx in &children {
            let (prop, client, value_idx) = {
                let pm = self.manifest.get_file(file)?;
                let record = pm.keys.get(child_idx)?;
                let prop   = bit::get(record, bit::OFFSET_PROP,   bit::MASK_PROP)   as u8;
                let client = bit::get(record, bit::OFFSET_CLIENT, bit::MASK_CLIENT) as u8;
                let is_leaf = bit::get(record, bit::OFFSET_IS_LEAF, bit::MASK_IS_LEAF) == 1;
                let value_idx = if is_leaf {
                    bit::get(record, bit::OFFSET_CHILD, bit::MASK_CHILD) as u16
                } else { 0 };
                (prop, client, value_idx)
            };

            // client prop
            if client != 0 {
                let client_str = match client as u64 {
                    bit::CLIENT_STATE    => "State",
                    bit::CLIENT_IN_MEMORY => "InMemory",
                    bit::CLIENT_ENV      => "Env",
                    bit::CLIENT_KVS      => "KVS",
                    bit::CLIENT_DB       => "Db",
                    bit::CLIENT_API      => "API",
                    bit::CLIENT_FILE     => "File",
                    _ => continue,
                };
                config.insert("client".to_string(), Value::String(client_str.to_string()));
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
                // build map object from children
                let map_val = self.build_map_config(file, child_idx)?;
                config.insert("map".to_string(), map_val);
            } else if value_idx != 0 {
                if let Some(s) = self.resolve_value_to_string(file, value_idx) {
                    config.insert(prop_name.to_string(), Value::String(s));
                }
            }
        }

        Some(config)
    }

    /// Builds a map config object from a map prop record's children.
    fn build_map_config(&self, file: &str, map_idx: u16) -> Option<Value> {
        let pm = self.manifest.get_file(file)?;
        let record = pm.keys.get(map_idx)?;
        let child_idx = bit::get(record, bit::OFFSET_CHILD, bit::MASK_CHILD) as u16;
        if child_idx == 0 { return Some(Value::Object(serde_json::Map::new())); }

        let has_children = bit::get(record, bit::OFFSET_HAS_CHILDREN, bit::MASK_HAS_CHILDREN);
        let children = if has_children == 1 {
            pm.children_map.get(child_idx)?.to_vec()
        } else {
            vec![child_idx]
        };

        let mut map = serde_json::Map::new();
        for &c in &children {
            let child = pm.keys.get(c)?;
            let dyn_idx   = bit::get(child, bit::OFFSET_DYNAMIC, bit::MASK_DYNAMIC) as u16;
            let value_idx = bit::get(child, bit::OFFSET_CHILD,   bit::MASK_CHILD)   as u16;
            let is_path   = bit::get(child, bit::OFFSET_IS_PATH, bit::MASK_IS_PATH) == 1;

            // map key: qualified path string
            let key_str = if is_path {
                let segs = pm.path_map.get(dyn_idx)?;
                segs.iter()
                    .filter_map(|&s| pm.dynamic.get(s).map(|x| x.to_string()))
                    .collect::<Vec<_>>()
                    .join(".")
            } else {
                pm.dynamic.get(dyn_idx)?.to_string()
            };

            // map value: db column name (static string)
            let val_vo = pm.values.get(value_idx)?;
            let col_dyn = bit::get(val_vo[0], bit::VO_OFFSET_T0_DYNAMIC, bit::VO_MASK_DYNAMIC) as u16;
            let col_str = pm.dynamic.get(col_dyn)?.to_string();

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
    /// Returns `None` on miss, unknown key, or missing manifest.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::common::state::State;
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
    /// // store miss with no load client configured → None
    /// assert_eq!(state.get("connection.common"), None);
    ///
    /// // set then get
    /// state.set("connection.common", json!({"host": "localhost"}), None);
    /// assert!(state.get("connection.common").is_some());
    /// ```
    pub fn get(&mut self, key: &str) -> Option<Value> {
        crate::fn_log!("State", "get", key);
        if self.called_keys.len() >= self.max_recursion {
            return None;
        }
        if self.called_keys.contains(&key.to_string()) {
            return None;
        }

        self.called_keys.push(key.to_string());

        let (file, path) = Self::split_key(key);
        let file = file.to_string();
        let path = path.to_string();

        if self.manifest.load(&file).is_err() {
            self.called_keys.pop();
            return None;
        }

        // find key record index
        let key_idx = match self.manifest.find(&file, &path) {
            Some(idx) => idx,
            None => { self.called_keys.pop(); return None; }
        };

        // check state_values cache
        if let Some(sv_idx) = self.find_state_value(key_idx) {
            let val = self.state_values.get_value(sv_idx).cloned();
            self.called_keys.pop();
            return val;
        }

        let meta = self.manifest.get_meta(&file, &path);

        // try _store
        let has_state_client = meta.load.and_then(|load_idx| {
            self.manifest.get_file(&file)
                .and_then(|pm| pm.keys.get(load_idx))
                .map(|r| bit::get(r, bit::OFFSET_CLIENT, bit::MASK_CLIENT) == bit::CLIENT_STATE)
        }).unwrap_or(false);

        if !has_state_client {
            if let Some(store_idx) = meta.store {
                if let Some(config) = self.build_config(&file, store_idx) {
                    if let Some(value) = self.store.get(&config) {
                        // cache into state_values
                        self.state_values.push(key_idx, value.clone());
                        self.called_keys.pop();
                        return Some(value);
                    }
                }
            }
        }

        // try _load
        let result = if let Some(load_idx) = meta.load {
            if let Some(mut config) = self.build_config(&file, load_idx) {
                if !config.contains_key("client") {
                    self.called_keys.pop();
                    return None;
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
                        // store loaded value
                        if let Some(store_idx) = meta.store {
                            if let Some(store_config) = self.build_config(&file, store_idx) {
                                let _ = self.store.set(&store_config, loaded.clone(), None);
                            }
                        }
                        self.state_values.push(key_idx, loaded.clone());
                        Some(loaded)
                    }
                    Err(_) => None,
                }
            } else { None }
        } else { None };

        self.called_keys.pop();
        result
    }

    /// Writes `value` to the _store backend for `key`.
    /// Returns `true` on success.
    ///
    /// # Examples
    ///
    /// ```
    /// # use state_engine::common::state::State;
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
    /// assert!(state.set("connection.common", json!({"host": "localhost"}), None));
    /// ```
    pub fn set(&mut self, key: &str, value: Value, ttl: Option<u64>) -> bool {
        crate::fn_log!("State", "set", key);
        let (file, path) = Self::split_key(key);
        let file = file.to_string();
        let path = path.to_string();

        if self.manifest.load(&file).is_err() {
            return false;
        }

        let key_idx = match self.manifest.find(&file, &path) {
            Some(idx) => idx,
            None => return false,
        };

        let meta = self.manifest.get_meta(&file, &path);

        if let Some(store_idx) = meta.store {
            if let Some(config) = self.build_config(&file, store_idx) {
                crate::fn_log!("State", "set/store.set",
                    config.get("client").and_then(|v| v.as_str()).unwrap_or(""),
                    config.get("key").and_then(|v| v.as_str()).unwrap_or(""));
                let ok = self.store.set(&config, value.clone(), ttl);
                if ok {
                    // update state_values
                    if let Some(sv_idx) = self.find_state_value(key_idx) {
                        self.state_values.update(sv_idx, value);
                    } else {
                        self.state_values.push(key_idx, value);
                    }
                }
                return ok;
            }
        }
        false
    }

    /// Removes the value for `key` from the _store backend.
    /// Returns `true` on success.
    ///
    /// # Examples
    ///
    /// ```
    /// # use state_engine::common::state::State;
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
    /// state.set("connection.common", json!({"host": "localhost"}), None);
    /// assert!(state.delete("connection.common"));
    /// assert_eq!(state.get("connection.common"), None);
    /// ```
    pub fn delete(&mut self, key: &str) -> bool {
        crate::fn_log!("State", "delete", key);
        let (file, path) = Self::split_key(key);
        let file = file.to_string();
        let path = path.to_string();

        if self.manifest.load(&file).is_err() {
            return false;
        }

        let key_idx = match self.manifest.find(&file, &path) {
            Some(idx) => idx,
            None => return false,
        };

        let meta = self.manifest.get_meta(&file, &path);

        if let Some(store_idx) = meta.store {
            if let Some(config) = self.build_config(&file, store_idx) {
                crate::fn_log!("State", "delete/store.delete",
                    config.get("client").and_then(|v| v.as_str()).unwrap_or(""),
                    config.get("key").and_then(|v| v.as_str()).unwrap_or(""));
                let ok = self.store.delete(&config);
                if ok {
                    if let Some(sv_idx) = self.find_state_value(key_idx) {
                        self.state_values.remove(sv_idx);
                    }
                }
                return ok;
            }
        }
        false
    }

    /// Returns `true` if a value exists for `key` in state cache or _store.
    /// Does not trigger _load.
    ///
    /// # Examples
    ///
    /// ```
    /// # use state_engine::common::state::State;
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
    /// assert!(!state.exists("connection.common"));
    /// state.set("connection.common", json!({"host": "localhost"}), None);
    /// assert!(state.exists("connection.common"));
    /// ```
    pub fn exists(&mut self, key: &str) -> bool {
        crate::fn_log!("State", "exists", key);
        let (file, path) = Self::split_key(key);
        let file = file.to_string();
        let path = path.to_string();

        if self.manifest.load(&file).is_err() {
            return false;
        }

        let key_idx = match self.manifest.find(&file, &path) {
            Some(idx) => idx,
            None => return false,
        };

        // check state_values cache first
        if let Some(sv_idx) = self.find_state_value(key_idx) {
            return !self.state_values.get_value(sv_idx)
                .map_or(true, |v| v.is_null());
        }

        // check store
        let meta = self.manifest.get_meta(&file, &path);
        if let Some(store_idx) = meta.store {
            if let Some(config) = self.build_config(&file, store_idx) {
                return self.store.get(&config).is_some();
            }
        }
        false
    }
}
