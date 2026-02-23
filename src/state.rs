use crate::ports::provided::{Manifest as ManifestTrait, State as StateTrait};
use crate::ports::required::{KVSClient, InMemoryClient};
use crate::common::{DotString, DotMapAccessor, Placeholder};
use crate::store::Store;
use crate::load::Load;
use crate::fn_log;
use crate::warn_log;
use serde_json::Value;
use std::collections::HashMap;

pub struct State<'a> {
    dot_accessor: DotMapAccessor,
    manifest: &'a mut dyn ManifestTrait,
    load: Load<'a>,
    store: Store<'a>,
    max_recursion: usize,
    called_keys: Vec<DotString>,
    cache: Value,
}

impl<'a> State<'a> {
    /// # Examples
    ///
    /// ```
    /// use state_engine::{Manifest, State, Load};
    ///
    /// let mut manifest = Manifest::new("./examples/manifest");
    /// let load = Load::new();
    /// let mut state = State::new(&mut manifest, load);
    /// ```
    pub fn new(manifest: &'a mut dyn ManifestTrait, load: Load<'a>) -> Self {
        Self {
            manifest,
            load,
            store: Store::new(),
            max_recursion: 20,
            called_keys: Vec::new(),
            cache: Value::Object(serde_json::Map::new()),
            dot_accessor: DotMapAccessor::new(),
        }
    }

    pub fn with_in_memory(mut self, client: &'a mut dyn InMemoryClient) -> Self {
        self.store = self.store.with_in_memory(client);
        self
    }

    pub fn with_kvs_client(mut self, client: &'a mut dyn KVSClient) -> Self {
        self.store = self.store.with_kvs_client(client);
        self
    }

    /// Returns the owner path where `_load` or `_store` is defined in the manifest.
    ///
    /// Derives from the first qualified key in `_load.map`; falls back to `called_key`
    /// (is_load=true) or its parent path (is_load=false) when no map is present.
    fn get_owner_path(&self, meta: &HashMap<String, Value>, is_load: bool) -> DotString {
        meta.get("_load")
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get("map"))
            .and_then(|v| v.as_object())
            .and_then(|map| map.keys().next())
            .and_then(|qualified_key| {
                qualified_key.rfind('.').map(|pos| DotString::new(&qualified_key[..pos]))
            })
            .unwrap_or_else(|| {
                if let Some(called_key) = self.called_keys.last() {
                    if is_load {
                        DotString::new(called_key.as_str())
                    } else {
                        if called_key.len() <= 1 {
                            DotString::new("")
                        } else {
                            DotString::new(&called_key[..called_key.len() - 1].join("."))
                        }
                    }
                } else {
                    DotString::new("")
                }
            })
    }

    /// Resolves `${path}` placeholders in a config map using the instance cache and `self.get()`.
    fn resolve_config_placeholders(&mut self, config: &mut HashMap<String, Value>) {
        let placeholder_names: Vec<String> = Placeholder::collect(config);

        if placeholder_names.is_empty() {
            return;
        }

        let mut resolved_values: HashMap<String, Value> = HashMap::new();
        let mut pending_paths: Vec<String> = Vec::new();

        for name in placeholder_names {
            let name_dot = DotString::new(&name);
            if let Some(cached) = self.dot_accessor.get(&self.cache, &name_dot) {
                resolved_values.insert(name, cached.clone());
            } else {
                pending_paths.push(name);
            }
        }

        Placeholder::replace(config, &resolved_values);

        for path in pending_paths {
            if let Some(value) = self.get(&path) {
                resolved_values.insert(path, value);
            }
        }

        let final_missing = Placeholder::replace(config, &resolved_values);

        if !final_missing.is_empty() {
            let missing_list = final_missing.join(", ");
            warn_log!(
                "State",
                "resolve_config_placeholders",
                &format!("Unresolved placeholders: {}", missing_list)
            );
        }
    }
}

impl<'a> StateTrait for State<'a> {
    /// # Examples
    ///
    /// ```
    /// use state_engine::{Manifest, State, Load, StateTrait, InMemoryClient};
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
    /// let mut manifest = Manifest::new("./examples/manifest");
    /// let mut client = MockInMemory::new();
    /// let mut state = State::new(&mut manifest, Load::new()).with_in_memory(&mut client);
    ///
    /// // store miss with no load client → None
    /// assert_eq!(state.get("connection.common"), None);
    ///
    /// // when set before get
    /// state.set("connection.common", json!({"host": "localhost"}), None);
    /// assert!(state.get("connection.common").is_some());
    /// ```
    fn get(&mut self, key: &str) -> Option<Value> {
        fn_log!("State", "get", key);

        if self.called_keys.len() >= self.max_recursion {
            eprintln!(
                "State::get: max recursion depth ({}) reached for key '{}'",
                self.max_recursion, key
            );
            return None;
        }

        self.called_keys.push(DotString::new(key));

        // 1. cache check
        let current_key = self.called_keys.last().unwrap();
        if DotMapAccessor::has(&self.cache, current_key) {
            let cached = self.dot_accessor.get(&self.cache, current_key).cloned();
            self.called_keys.pop();
            return cached;
        }

        let meta = self.manifest.get_meta(key);
        if meta.is_empty() {
            self.called_keys.pop();
            return None;
        }

        // Skip _store when _load.client is "State" (intra-State reference)
        let has_state_client = meta.get("_load")
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get("client"))
            .and_then(|v| v.as_str())
            == Some("State");

        // 2. Try _store
        if !has_state_client {
            if let Some(store_config_value) = meta.get("_store") {
                if let Some(store_config_obj) = store_config_value.as_object() {
                    let mut store_config: HashMap<String, Value> =
                        store_config_obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

                    self.resolve_config_placeholders(&mut store_config);

                    if let Some(value) = self.store.get(&store_config) {
                        let owner_path = self.get_owner_path(&meta, false);

                        self.manifest.clear_missing_keys();
                        let manifest_value = self.manifest.get_value(&owner_path);
                        if self.manifest.get_missing_keys().is_empty() {
                            DotMapAccessor::merge(&mut self.cache, &owner_path, manifest_value);
                        }

                        if value.is_object() {
                            DotMapAccessor::merge(&mut self.cache, &owner_path, value);
                        } else {
                            let called_key = self.called_keys.last().unwrap();
                            DotMapAccessor::set(&mut self.cache, called_key, value);
                        }

                        // Use has() before get() to avoid early-returning null nodes
                        let called_key = self.called_keys.last().unwrap();
                        if DotMapAccessor::has(&self.cache, called_key) {
                            let extracted = self.dot_accessor.get(&self.cache, called_key).cloned();
                            self.called_keys.pop();
                            return extracted;
                        }
                    }
                }
            }
        }

        // 3. Auto-load on miss
        let result = if let Some(load_config_value) = meta.get("_load") {
            if let Some(load_config_obj) = load_config_value.as_object() {
                let mut load_config: HashMap<String, Value> =
                    load_config_obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

                if !load_config.contains_key("client") {
                    self.called_keys.pop();
                    return None;
                }

                self.resolve_config_placeholders(&mut load_config);

                let client_value = load_config.get("client").and_then(|v| v.as_str());

                if client_value == Some("State") {
                    if let Some(key_value) = load_config.get("key") {
                        let current_key = self.called_keys.last().unwrap();
                        DotMapAccessor::set(&mut self.cache, current_key, key_value.clone());

                        self.called_keys.pop();
                        return Some(key_value.clone());
                    }
                    self.called_keys.pop();
                    None
                } else {
                    // Unqualify map keys (qualified path → relative field name) before passing to Load
                    if let Some(map_value) = load_config.get("map") {
                        if let Value::Object(map_obj) = map_value {
                            let mut unqualified_map = serde_json::Map::new();
                            for (qualified_key, db_column) in map_obj {
                                if let Some(pos) = qualified_key.rfind('.') {
                                    let field_name = &qualified_key[pos + 1..];
                                    unqualified_map.insert(field_name.to_string(), db_column.clone());
                                } else {
                                    unqualified_map.insert(qualified_key.clone(), db_column.clone());
                                }
                            }
                            load_config.insert("map".to_string(), Value::Object(unqualified_map));
                        }
                    }

                    if let Ok(loaded) = self.load.handle(&load_config) {
                        let owner_path = self.get_owner_path(&meta, true);

                        self.manifest.clear_missing_keys();
                        let manifest_value = self.manifest.get_value(&owner_path);
                        if self.manifest.get_missing_keys().is_empty() {
                            DotMapAccessor::merge(&mut self.cache, &owner_path, manifest_value);
                        }

                        DotMapAccessor::merge(&mut self.cache, &owner_path, loaded.clone());

                        // On load success, persist merged cache value to _store
                        if let Some(store_config_value) = meta.get("_store") {
                            if let Some(store_config_obj) = store_config_value.as_object() {
                                let mut store_config: HashMap<String, Value> = store_config_obj
                                    .iter()
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect();

                                self.resolve_config_placeholders(&mut store_config);

                                if let Some(cache_value) = self.dot_accessor.get(&self.cache, &owner_path) {
                                    self.store.set(&store_config, cache_value.clone(), None);
                                }
                            }
                        }

                        // Use has() before get() to avoid early-returning null nodes
                        let called_key = self.called_keys.last().unwrap();
                        if DotMapAccessor::has(&self.cache, called_key) {
                            self.dot_accessor.get(&self.cache, called_key).cloned()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        self.called_keys.pop();
        result
    }

    /// # Examples
    ///
    /// ```
    /// # use state_engine::{Manifest, State, Load, StateTrait, InMemoryClient};
    /// # use serde_json::{json, Value};
    /// # struct MockInMemory { data: std::collections::HashMap<String, Value> }
    /// # impl MockInMemory { fn new() -> Self { Self { data: Default::default() } } }
    /// # impl InMemoryClient for MockInMemory {
    /// #     fn get(&self, key: &str) -> Option<Value> { self.data.get(key).cloned() }
    /// #     fn set(&mut self, key: &str, value: Value) { self.data.insert(key.to_string(), value); }
    /// #     fn delete(&mut self, key: &str) -> bool { self.data.remove(key).is_some() }
    /// # }
    /// let mut manifest = Manifest::new("./examples/manifest");
    /// let mut client = MockInMemory::new();
    /// let mut state = State::new(&mut manifest, Load::new()).with_in_memory(&mut client);
    ///
    /// let ok = state.set("connection.common", json!({"host": "localhost"}), None);
    /// assert!(ok);
    /// ```
    fn set(&mut self, key: &str, value: Value, ttl: Option<u64>) -> bool {
        fn_log!("State", "set", key);

        self.called_keys.push(DotString::new(key));

        let meta = self.manifest.get_meta(key);
        if meta.is_empty() {
            eprintln!("State::set: meta is empty for key '{}'", key);
            self.called_keys.pop();
            return false;
        }

        let store_config = match meta.get("_store").and_then(|v| v.as_object()) {
            Some(config) => config,
            None => {
                eprintln!("State::set: no _store config for key '{}'", key);
                self.called_keys.pop();
                return false;
            }
        };

        let mut store_config_map: HashMap<String, Value> =
            store_config.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        self.resolve_config_placeholders(&mut store_config_map);

        let owner_path = self.get_owner_path(&meta, false);

        // Pre-load owner object from store into cache to preserve sibling fields
        if self.dot_accessor.get(&self.cache, &owner_path).is_none() {
            if let Some(store_value) = self.store.get(&store_config_map) {
                DotMapAccessor::merge(&mut self.cache, &owner_path, store_value);
            }
        }

        let called_key = self.called_keys.last().unwrap();
        DotMapAccessor::set(&mut self.cache, called_key, value);

        let store_value = self.dot_accessor.get(&self.cache, &owner_path)
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

        let result = self.store.set(&store_config_map, store_value, ttl);

        self.called_keys.pop();
        result
    }

    /// # Examples
    ///
    /// ```
    /// # use state_engine::{Manifest, State, Load, StateTrait, InMemoryClient};
    /// # use serde_json::{json, Value};
    /// # struct MockInMemory { data: std::collections::HashMap<String, Value> }
    /// # impl MockInMemory { fn new() -> Self { Self { data: Default::default() } } }
    /// # impl InMemoryClient for MockInMemory {
    /// #     fn get(&self, key: &str) -> Option<Value> { self.data.get(key).cloned() }
    /// #     fn set(&mut self, key: &str, value: Value) { self.data.insert(key.to_string(), value); }
    /// #     fn delete(&mut self, key: &str) -> bool { self.data.remove(key).is_some() }
    /// # }
    /// let mut manifest = Manifest::new("./examples/manifest");
    /// let mut client = MockInMemory::new();
    /// let mut state = State::new(&mut manifest, Load::new()).with_in_memory(&mut client);
    ///
    /// state.set("connection.common", json!({"host": "localhost"}), None);
    /// assert!(state.delete("connection.common"));
    /// assert_eq!(state.get("connection.common"), None);
    /// ```
    fn delete(&mut self, key: &str) -> bool {
        fn_log!("State", "delete", key);

        self.called_keys.push(DotString::new(key));

        let meta = self.manifest.get_meta(key);
        if meta.is_empty() {
            self.called_keys.pop();
            return false;
        }

        let store_config = match meta.get("_store").and_then(|v| v.as_object()) {
            Some(config) => config,
            None => {
                self.called_keys.pop();
                return false;
            }
        };

        let mut store_config_map: HashMap<String, Value> =
            store_config.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        self.resolve_config_placeholders(&mut store_config_map);

        let owner_path = self.get_owner_path(&meta, false);

        // When key == owner_path, delete the entire owner object from store
        if key == owner_path.as_str() {
            let result = self.store.delete(&store_config_map);
            if result {
                let called_key = self.called_keys.last().unwrap();
                DotMapAccessor::unset(&mut self.cache, called_key);
            }
            self.called_keys.pop();
            return result;
        }

        // Pre-load owner object from store into cache to preserve sibling fields
        if self.dot_accessor.get(&self.cache, &owner_path).is_none() {
            if let Some(store_value) = self.store.get(&store_config_map) {
                DotMapAccessor::merge(&mut self.cache, &owner_path, store_value);
            }
        }

        let cache_backup = self.cache.clone();

        let called_key = self.called_keys.last().unwrap();
        DotMapAccessor::unset(&mut self.cache, called_key);

        let store_value = self.dot_accessor.get(&self.cache, &owner_path)
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

        let result = self.store.set(&store_config_map, store_value, None);

        if !result {
            self.cache = cache_backup;
        }

        // Github issue #22

        self.called_keys.pop();
        result
    }

    /// # Examples
    ///
    /// ```
    /// # use state_engine::{Manifest, State, Load, StateTrait, InMemoryClient};
    /// # use serde_json::{json, Value};
    /// # struct MockInMemory { data: std::collections::HashMap<String, Value> }
    /// # impl MockInMemory { fn new() -> Self { Self { data: Default::default() } } }
    /// # impl InMemoryClient for MockInMemory {
    /// #     fn get(&self, key: &str) -> Option<Value> { self.data.get(key).cloned() }
    /// #     fn set(&mut self, key: &str, value: Value) { self.data.insert(key.to_string(), value); }
    /// #     fn delete(&mut self, key: &str) -> bool { self.data.remove(key).is_some() }
    /// # }
    /// let mut manifest = Manifest::new("./examples/manifest");
    /// let mut client = MockInMemory::new();
    /// let mut state = State::new(&mut manifest, Load::new()).with_in_memory(&mut client);
    ///
    /// assert!(!state.exists("connection.common"));
    /// state.set("connection.common", json!({"host": "localhost"}), None);
    /// assert!(state.exists("connection.common"));
    /// ```
    fn exists(&mut self, key: &str) -> bool {
        fn_log!("State", "exists", key);

        self.called_keys.push(DotString::new(key));

        // 1. cache check (fastest path, no I/O)
        let current_key = self.called_keys.last().unwrap();
        if DotMapAccessor::has(&self.cache, current_key) {
            self.called_keys.pop();
            return true;
        }

        let meta = self.manifest.get_meta(key);
        if meta.is_empty() {
            self.called_keys.pop();
            return false;
        }

        let store_config = match meta.get("_store").and_then(|v| v.as_object()) {
            Some(config) => config,
            None => {
                self.called_keys.pop();
                return false;
            }
        };

        let mut store_config_map: HashMap<String, Value> =
            store_config.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        self.resolve_config_placeholders(&mut store_config_map);

        // Check store only — no auto-load (Github issue #4)
        let result = self.store.get(&store_config_map).is_some();

        self.called_keys.pop();
        result
    }
}

