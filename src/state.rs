use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use crate::core::fixed_bits;
use crate::core::manifest::{Manifest, ConfigValue};
use crate::core::parser::{Value as ParseValue, parse};
use crate::ports::provided::{ManifestError, StateError, Value};
use crate::ports::required::FileClient;
use crate::store::Store;
use crate::load::Load;

use std::sync::Arc;

pub struct State {
    manifest_dir: PathBuf,
    manifest_file: Box<dyn FileClient>,
    manifest: Manifest,
    state_keys: Vec<u16>,
    state_vals: Vec<Value>,
    store: Store,
    load: Load,
    max_recursion: usize,
    called_keys: HashSet<String>,
}

impl State {
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
            manifest_dir: PathBuf::from(manifest_dir),
            manifest_file: Box::new(crate::ports::default::DefaultFileClient),
            manifest: Manifest::new(),
            state_keys: vec![0],
            state_vals: vec![Value::Null],
            store: Store::new(),
            load: Load::new(),
            max_recursion: 20,
            called_keys: HashSet::new(),
        }
    }

    pub fn with_in_memory(mut self, client: Arc<dyn crate::ports::required::InMemoryClient>) -> Self {
        self.store = self.store.with_in_memory(Arc::clone(&client));
        self.load = self.load.with_in_memory(client);
        self
    }

    pub fn with_kvs(mut self, client: Arc<dyn crate::ports::required::KVSClient>) -> Self {
        self.store = self.store.with_kvs(Arc::clone(&client));
        self.load = self.load.with_kvs(client);
        self
    }

    pub fn with_db(mut self, client: Arc<dyn crate::ports::required::DbClient>) -> Self {
        self.load = self.load.with_db(client);
        self
    }

    pub fn with_env(mut self, client: Arc<dyn crate::ports::required::EnvClient>) -> Self {
        self.load = self.load.with_env(client);
        self
    }

    pub fn with_http(mut self, client: Arc<dyn crate::ports::required::HttpClient>) -> Self {
        self.store = self.store.with_http(Arc::clone(&client));
        self.load = self.load.with_http(client);
        self
    }

    pub fn with_file(mut self, client: Arc<dyn crate::ports::required::FileClient>) -> Self {
        self.store = self.store.with_file(Arc::clone(&client));
        self.load = self.load.with_file(client);
        self
    }

    pub fn with_manifest_file(mut self, client: impl FileClient + 'static) -> Self {
        self.manifest_file = Box::new(client);
        self
    }

    fn load_manifest(&mut self, file: &str) -> Result<(), ManifestError> {
        crate::fn_log!("State", "load_manifest", file);
        if self.manifest.is_loaded(file) {
            return Ok(());
        }

        let yml_path  = self.manifest_dir.join(format!("{}.yml",  file));
        let yaml_path = self.manifest_dir.join(format!("{}.yaml", file));
        let yml_key   = yml_path.to_string_lossy();
        let yaml_key  = yaml_path.to_string_lossy();
        let yml_content  = self.manifest_file.get(&yml_key);
        let yaml_content = self.manifest_file.get(&yaml_key);

        let content = match (yml_content, yaml_content) {
            (Some(_), Some(_)) => return Err(ManifestError::AmbiguousFile(
                format!("both '{}.yml' and '{}.yaml' exist.", file, file)
            )),
            (Some(c), None) => c,
            (None, Some(c)) => c,
            (None, None) => return Err(ManifestError::FileNotFound(
                format!("'{}.yml' or '{}.yaml'", file, file)
            )),
        };

        let content_str = std::str::from_utf8(&content)
            .map_err(|e| ManifestError::ParseError(format!("UTF-8 error: {}", e)))?;

        let yaml_root: serde_yaml_ng::Value = serde_yaml_ng::from_str(content_str)
            .map_err(|e| ManifestError::ParseError(format!("YAML parse error: {}", e)))?;

        let pm = parse(
            file,
            yaml_to_parse_value(yaml_root),
            &mut self.manifest.dynamic,
            &mut self.manifest.keys,
            &mut self.manifest.values,
            &mut self.manifest.path_map,
            &mut self.manifest.children_map,
        ).map_err(|e| ManifestError::ParseError(e))?;

        self.manifest.insert(file.to_string(), pm);
        Ok(())
    }

    fn split_key<'k>(key: &'k str) -> (&'k str, &'k str) {
        match key.find('.') {
            Some(pos) => (&key[..pos], &key[pos + 1..]),
            None => (key, ""),
        }
    }

    fn find_state_value(&self, key_idx: u16) -> Option<usize> {
        self.state_keys.iter().skip(1).position(|&k| k == key_idx).map(|p| p + 1)
    }

    fn resolve_template(&mut self, template: &str) -> Result<Option<String>, StateError> {
        let mut result = String::new();
        let mut remaining = template;
        while let Some(start) = remaining.find("${") {
            result.push_str(&remaining[..start]);
            remaining = &remaining[start + 2..];
            let end = match remaining.find('}') {
                Some(e) => e,
                None => return Ok(None),
            };
            let path = &remaining[..end];
            remaining = &remaining[end + 1..];
            let resolved = match self.get(path)? {
                Some(Value::Scalar(b)) => String::from_utf8_lossy(&b).into_owned(),
                _ => return Ok(None),
            };
            result.push_str(&resolved);
        }
        result.push_str(remaining);
        Ok(Some(result))
    }

    fn resolve_config_value(&mut self, cv: ConfigValue) -> Result<Option<Value>, StateError> {
        match cv {
            ConfigValue::Client(c) => Ok(Some(Value::Scalar(c.to_le_bytes().to_vec()))),
            ConfigValue::Placeholder(path) => self.get(&path),
            ConfigValue::Str(s) if s.contains("${") => {
                Ok(self.resolve_template(&s)?.map(|s| Value::Scalar(s.into_bytes())))
            }
            ConfigValue::Str(s) => Ok(Some(Value::Scalar(s.into_bytes()))),
            ConfigValue::Map(pairs) => {
                Ok(Some(Value::Mapping(
                    pairs.into_iter()
                        .map(|(k, v)| (k.into_bytes(), Value::Scalar(v.into_bytes())))
                        .collect()
                )))
            }
        }
    }

    fn resolve_config(&mut self, meta_idx: u16) -> Result<Option<HashMap<String, Value>>, StateError> {
        let entries = match self.manifest.build_config(meta_idx) {
            Some(e) => e,
            None => return Ok(None),
        };

        let mut config = HashMap::new();
        for (key, cv) in entries {
            if key == "map" {
                if let ConfigValue::Map(pairs) = cv {
                    let yaml_keys = Value::Sequence(pairs.iter().map(|(k, _)| Value::Scalar(k.as_bytes().to_vec())).collect());
                    let ext_keys  = Value::Sequence(pairs.into_iter().map(|(_, v)| Value::Scalar(v.into_bytes())).collect());
                    config.insert("yaml_keys".into(), yaml_keys);
                    config.insert("ext_keys".into(),  ext_keys);
                }
            } else if let Some(v) = self.resolve_config_value(cv)? {
                config.insert(key, v);
            }
        }
        Ok(Some(config))
    }

    /// Returns the value for `key`, checking state cache → _store → _load in order.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::{State, Value};
    /// use state_engine::InMemoryClient;
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
    ///     .with_in_memory(std::sync::Arc::new(client));
    ///
    /// // set then get
    /// state.set("connection.common", Value::Scalar(b"test".to_vec()), None).unwrap();
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

        if let Err(e) = self.load_manifest(&file) {
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

        if let Some(sv_idx) = self.find_state_value(key_idx) {
            let val = self.state_vals.get(sv_idx).cloned();
            self.called_keys.remove(key);
            return Ok(val);
        }

        let meta = self.manifest.get_meta(&file, &path);

        let has_state_client = meta.load
            .map(|load_idx| self.manifest.get_client(load_idx) == fixed_bits::CLIENT_STATE)
            .unwrap_or(false);

        if !has_state_client {
            if let Some(store_idx) = meta.store {
                match self.resolve_config(store_idx) {
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

        // CLIENT_STATE: extract key path directly from build_config without resolving
        if has_state_client {
            if let Some(load_idx) = meta.load {
                let state_key = self.manifest.build_config(load_idx)
                    .and_then(|entries| entries.into_iter().find(|(k, _)| k == "key"))
                    .and_then(|(_, cv)| match cv {
                        ConfigValue::Placeholder(p) => Some(p),
                        ConfigValue::Str(s) => Some(s),
                        _ => None,
                    });
                let result = match state_key {
                    Some(k) => self.get(&k),
                    None => Ok(None),
                };
                self.called_keys.remove(key);
                return result;
            }
        }

        let result = if let Some(load_idx) = meta.load {
            match self.resolve_config(load_idx) {
                Ok(Some(mut config)) => {
                    if !config.contains_key("client") {
                        self.called_keys.remove(key);
                        return Ok(None);
                    }

                    // unqualify map keys for Load
                    if let Some(Value::Mapping(map_pairs)) = config.get("map").cloned() {
                        let unqualified: Vec<(Vec<u8>, Value)> = map_pairs.into_iter()
                            .map(|(qk, v)| {
                                let field = qk.iter().rposition(|&b| b == b'.')
                                    .map_or(qk.clone(), |p| qk[p+1..].to_vec());
                                (field, v)
                            })
                            .collect();
                        config.insert("map".to_string(), Value::Mapping(unqualified));
                    }

                    match self.load.handle(&config) {
                        Ok(loaded) => {
                            if let Some(store_idx) = meta.store {
                                match self.resolve_config(store_idx) {
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
                                    Err(_) => {}
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
    /// # use state_engine::{State, Value};
    /// # use state_engine::InMemoryClient;
    /// # struct MockInMemory { data: std::sync::Mutex<std::collections::HashMap<String, Value>> }
    /// # impl MockInMemory { fn new() -> Self { Self { data: Default::default() } } }
    /// # impl InMemoryClient for MockInMemory {
    /// #     fn get(&self, key: &str) -> Option<Value> { self.data.lock().unwrap().get(key).cloned() }
    /// #     fn set(&self, key: &str, value: Value) -> bool { self.data.lock().unwrap().insert(key.to_string(), value); true }
    /// #     fn delete(&self, key: &str) -> bool { self.data.lock().unwrap().remove(key).is_some() }
    /// # }
    /// let client = MockInMemory::new();
    /// let mut state = State::new("./examples/manifest")
    ///     .with_in_memory(std::sync::Arc::new(client));
    ///
    /// assert!(state.set("connection.common", Value::Scalar(b"data".to_vec()), None).unwrap());
    /// ```
    pub fn set(&mut self, key: &str, value: Value, ttl: Option<u64>) -> Result<bool, StateError> {
        crate::fn_log!("State", "set", key);
        let (file, path) = Self::split_key(key);
        let file = file.to_string();
        let path = path.to_string();

        if let Err(e) = self.load_manifest(&file) {
            return Err(StateError::ManifestLoadFailed(e.to_string()));
        }

        let key_idx = match self.manifest.find(&file, &path) {
            Some(idx) => idx,
            None => return Err(StateError::KeyNotFound(key.to_string())),
        };

        let meta = self.manifest.get_meta(&file, &path);

        if let Some(store_idx) = meta.store {
            let owner_idx = meta.store_owner;
            let is_leaf = owner_idx != key_idx;

            // For leaf keys: build updated owner Mapping via read-modify-write
            let store_value = if is_leaf {
                let field = path.rsplit('.').next().unwrap_or(&path).as_bytes().to_vec();

                // 1. state_vals から owner Mapping を取得、なければ store から read
                let owner_mapping = match self.find_state_value(owner_idx)
                    .and_then(|i| self.state_vals.get(i).cloned())
                {
                    Some(v @ Value::Mapping(_)) => Some(v),
                    _ => {
                        match self.resolve_config(store_idx)? {
                            Some(ref config) => self.store.get(config),
                            None => None,
                        }
                    }
                };

                // 2. Mapping にフィールドを差し込む
                let mut pairs = match owner_mapping {
                    Some(Value::Mapping(p)) => p,
                    _ => vec![],
                };
                if let Some(entry) = pairs.iter_mut().find(|(k, _)| *k == field) {
                    entry.1 = value.clone();
                } else {
                    pairs.push((field, value.clone()));
                }
                Value::Mapping(pairs)
            } else {
                value.clone()
            };

            match self.resolve_config(store_idx)? {
                Some(config) => {
                    return match self.store.set(&config, store_value.clone(), ttl) {
                        Ok(ok) => {
                            if ok {
                                // owner の state_vals を更新
                                if let Some(sv_idx) = self.find_state_value(owner_idx) {
                                    self.state_vals[sv_idx] = store_value;
                                } else {
                                    self.state_keys.push(owner_idx);
                                    self.state_vals.push(store_value);
                                }
                                // 葉キー自身も state_vals に記録
                                if is_leaf {
                                    if let Some(sv_idx) = self.find_state_value(key_idx) {
                                        self.state_vals[sv_idx] = value;
                                    } else {
                                        self.state_keys.push(key_idx);
                                        self.state_vals.push(value);
                                    }
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
    /// # use state_engine::{State, Value};
    /// # use state_engine::InMemoryClient;
    /// # struct MockInMemory { data: std::sync::Mutex<std::collections::HashMap<String, Value>> }
    /// # impl MockInMemory { fn new() -> Self { Self { data: Default::default() } } }
    /// # impl InMemoryClient for MockInMemory {
    /// #     fn get(&self, key: &str) -> Option<Value> { self.data.lock().unwrap().get(key).cloned() }
    /// #     fn set(&self, key: &str, value: Value) -> bool { self.data.lock().unwrap().insert(key.to_string(), value); true }
    /// #     fn delete(&self, key: &str) -> bool { self.data.lock().unwrap().remove(key).is_some() }
    /// # }
    /// let client = MockInMemory::new();
    /// let mut state = State::new("./examples/manifest")
    ///     .with_in_memory(std::sync::Arc::new(client));
    ///
    /// state.set("connection.common", Value::Scalar(b"data".to_vec()), None).unwrap();
    /// assert!(state.delete("connection.common").unwrap());
    /// // after delete, store has no data; _load is attempted but EnvClient is not configured here
    /// assert!(state.get("connection.common").is_err() || state.get("connection.common").unwrap().is_none());
    /// ```
    pub fn delete(&mut self, key: &str) -> Result<bool, StateError> {
        crate::fn_log!("State", "delete", key);
        let (file, path) = Self::split_key(key);
        let file = file.to_string();
        let path = path.to_string();

        if let Err(e) = self.load_manifest(&file) {
            return Err(StateError::ManifestLoadFailed(e.to_string()));
        }

        let key_idx = match self.manifest.find(&file, &path) {
            Some(idx) => idx,
            None => return Err(StateError::KeyNotFound(key.to_string())),
        };

        let meta = self.manifest.get_meta(&file, &path);

        if let Some(store_idx) = meta.store {
            match self.resolve_config(store_idx)? {
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
    /// # use state_engine::{State, Value};
    /// # use state_engine::InMemoryClient;
    /// # struct MockInMemory { data: std::sync::Mutex<std::collections::HashMap<String, Value>> }
    /// # impl MockInMemory { fn new() -> Self { Self { data: Default::default() } } }
    /// # impl InMemoryClient for MockInMemory {
    /// #     fn get(&self, key: &str) -> Option<Value> { self.data.lock().unwrap().get(key).cloned() }
    /// #     fn set(&self, key: &str, value: Value) -> bool { self.data.lock().unwrap().insert(key.to_string(), value); true }
    /// #     fn delete(&self, key: &str) -> bool { self.data.lock().unwrap().remove(key).is_some() }
    /// # }
    /// let client = MockInMemory::new();
    /// let mut state = State::new("./examples/manifest")
    ///     .with_in_memory(std::sync::Arc::new(client));
    ///
    /// assert!(!state.exists("connection.common").unwrap());
    /// state.set("connection.common", Value::Scalar(b"data".to_vec()), None).unwrap();
    /// assert!(state.exists("connection.common").unwrap());
    /// ```
    pub fn exists(&mut self, key: &str) -> Result<bool, StateError> {
        crate::fn_log!("State", "exists", key);
        let (file, path) = Self::split_key(key);
        let file = file.to_string();
        let path = path.to_string();

        if let Err(e) = self.load_manifest(&file) {
            return Err(StateError::ManifestLoadFailed(e.to_string()));
        }

        let key_idx = match self.manifest.find(&file, &path) {
            Some(idx) => idx,
            None => return Err(StateError::KeyNotFound(key.to_string())),
        };

        if let Some(sv_idx) = self.find_state_value(key_idx) {
            return Ok(!matches!(self.state_vals.get(sv_idx), Some(Value::Null) | None));
        }

        let meta = self.manifest.get_meta(&file, &path);
        if let Some(store_idx) = meta.store {
            if let Some(config) = self.resolve_config(store_idx)? {
                return Ok(self.store.get(&config).is_some());
            }
        }
        Ok(false)
    }
}

fn yaml_to_parse_value(v: serde_yaml_ng::Value) -> ParseValue {
    match v {
        serde_yaml_ng::Value::Mapping(m) => ParseValue::Mapping(
            m.into_iter()
                .filter_map(|(k, v)| {
                    let key = match k {
                        serde_yaml_ng::Value::String(s) => s.into_bytes(),
                        _ => return None,
                    };
                    Some((key, yaml_to_parse_value(v)))
                })
                .collect(),
        ),
        serde_yaml_ng::Value::Sequence(s) => ParseValue::Sequence(
            s.into_iter().map(yaml_to_parse_value).collect()
        ),
        serde_yaml_ng::Value::String(s) => ParseValue::Scalar(s.into_bytes()),
        serde_yaml_ng::Value::Number(n) => ParseValue::Scalar(n.to_string().into_bytes()),
        serde_yaml_ng::Value::Bool(b)   => ParseValue::Scalar(b.to_string().into_bytes()),
        serde_yaml_ng::Value::Null      => ParseValue::Null,
        _                               => ParseValue::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::required::{KVSClient, DbClient, EnvClient, FileClient};
    use std::sync::Arc;

    struct StubKVS;
    impl KVSClient for StubKVS {
        fn get(&self, _: &str) -> Option<Vec<u8>> { None }
        fn set(&self, _: &str, _: Vec<u8>, _: Option<u64>) -> bool { false }
        fn delete(&self, _: &str) -> bool { false }
    }

    struct StubDb;
    impl DbClient for StubDb {
        fn get(&self, _: &Value, _: &str, _: &[Vec<u8>], _: Option<&[u8]>) -> Option<Vec<Value>> { None }
        fn set(&self, _: &Value, _: &str, _: &[Vec<u8>], _: Option<&[u8]>) -> bool { false }
        fn delete(&self, _: &Value, _: &str, _: Option<&[u8]>) -> bool { false }
    }

    struct StubEnv;
    impl EnvClient for StubEnv {
        fn get(&self, _: &[Vec<u8>]) -> Option<Vec<Value>> { None }
        fn set(&self, _: &str, _: Vec<u8>) -> bool { false }
        fn delete(&self, _: &str) -> bool { false }
    }

    struct StubFile;
    impl FileClient for StubFile {
        fn get(&self, _: &str) -> Option<Vec<u8>> { None }
        fn set(&self, _: &str, _: Vec<u8>) -> bool { false }
        fn delete(&self, _: &str) -> bool { false }
    }

    struct StubHttp;
    impl crate::ports::required::HttpClient for StubHttp {
        fn get(&self, _: &str, _: &[Vec<u8>], _: Option<&[(Vec<u8>, Vec<u8>)]>) -> Option<Vec<Value>> { None }
        fn set(&self, _: &str, _: Value, _: Option<&[(Vec<u8>, Vec<u8>)]>) -> bool { false }
        fn delete(&self, _: &str, _: Option<&[(Vec<u8>, Vec<u8>)]>) -> bool { false }
    }

    #[test]
    fn test_with_clients_build() {
        let _ = State::new("./examples/manifest").with_kvs(Arc::new(StubKVS));
        let _ = State::new("./examples/manifest").with_db(Arc::new(StubDb));
        let _ = State::new("./examples/manifest").with_env(Arc::new(StubEnv));
        let _ = State::new("./examples/manifest").with_http(Arc::new(StubHttp));
        let _ = State::new("./examples/manifest").with_file(Arc::new(StubFile));
    }
}
