// State - 統一CRUD実装
//
// manifest の _state/_store/_load に従って状態を管理する。

use crate::load::Load;
use crate::ports::provided::{Manifest as ManifestTrait, State as StateTrait};
use crate::ports::required::{KVSClient, ProcessMemoryClient};
use crate::common::PlaceholderResolver;
use serde_json::Value;
use std::collections::HashMap;

/// State実装
///
/// _state/_store/_load メタデータに基づいて状態を管理する。
pub struct StateManager<'a> {
    manifest: &'a mut dyn ManifestTrait,
    load: Load<'a>,
    process_memory: Option<&'a mut dyn ProcessMemoryClient>,
    kvs_client: Option<&'a mut dyn KVSClient>,
}

impl<'a> StateManager<'a> {
    /// 新しいStateManagerを作成
    pub fn new(manifest: &'a mut dyn ManifestTrait, load: Load<'a>) -> Self {
        Self {
            manifest,
            load,
            process_memory: None,
            kvs_client: None,
        }
    }

    /// ProcessMemoryClientを設定
    pub fn with_process_memory(mut self, client: &'a mut dyn ProcessMemoryClient) -> Self {
        self.process_memory = Some(client);
        self
    }

    /// KVSClientを設定
    pub fn with_kvs_client(mut self, client: &'a mut dyn KVSClient) -> Self {
        self.kvs_client = Some(client);
        self
    }

    /// _store設定から値を取得
    fn get_from_store(
        &self,
        store_config: &HashMap<String, Value>,
        params: &HashMap<String, String>,
    ) -> Option<Value> {
        let client = store_config.get("client")?.as_str()?;

        match client {
            "InMemory" => {
                let process_memory = self.process_memory.as_ref()?;
                let key_template = store_config.get("key")?.as_str()?;
                let key = PlaceholderResolver::replace(key_template, params);
                process_memory.get(&key)
            }
            "KVS" => {
                let kvs_client = self.kvs_client.as_ref()?;
                let key_template = store_config.get("key")?.as_str()?;
                let key = PlaceholderResolver::replace(key_template, params);
                kvs_client.get(&key)
            }
            _ => None,
        }
    }

    /// _store設定に値を保存
    fn set_to_store(
        &mut self,
        store_config: &HashMap<String, Value>,
        params: &HashMap<String, String>,
        value: Value,
        ttl: Option<u64>,
    ) -> bool {
        let client = match store_config.get("client").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return false,
        };

        match client {
            "InMemory" => {
                if let Some(process_memory) = self.process_memory.as_mut() {
                    if let Some(key_template) = store_config.get("key").and_then(|v| v.as_str()) {
                        let key = PlaceholderResolver::replace(key_template, params);
                        process_memory.set(&key, value);
                        return true;
                    }
                }
                false
            }
            "KVS" => {
                if let Some(kvs_client) = self.kvs_client.as_mut() {
                    if let Some(key_template) = store_config.get("key").and_then(|v| v.as_str()) {
                        let key = PlaceholderResolver::replace(key_template, params);

                        // ttl優先順位: 引数 > YAML設定
                        let final_ttl = ttl.or_else(|| {
                            store_config.get("ttl").and_then(|v| v.as_u64())
                        });

                        return kvs_client.set(&key, value, final_ttl);
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// _store設定から値を削除
    fn delete_from_store(
        &mut self,
        store_config: &HashMap<String, Value>,
        params: &HashMap<String, String>,
    ) -> bool {
        let client = match store_config.get("client").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return false,
        };

        match client {
            "InMemory" => {
                if let Some(process_memory) = self.process_memory.as_mut() {
                    if let Some(key_template) = store_config.get("key").and_then(|v| v.as_str()) {
                        let key = PlaceholderResolver::replace(key_template, params);
                        return process_memory.delete(&key);
                    }
                }
                false
            }
            "KVS" => {
                if let Some(kvs_client) = self.kvs_client.as_mut() {
                    if let Some(key_template) = store_config.get("key").and_then(|v| v.as_str()) {
                        let key = PlaceholderResolver::replace(key_template, params);
                        return kvs_client.delete(&key);
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// manifest keyからパラメータを構築
    ///
    /// TODO: 現在はダミー実装。本来はUserKey等から取得
    fn build_params(&self, _key: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        // ダミーパラメータ
        params.insert("sso_user_id".to_string(), "user001".to_string());
        params.insert("tenant_id".to_string(), "1".to_string());
        params.insert("org_id".to_string(), "100".to_string());
        params
    }
}

impl<'a> StateTrait for StateManager<'a> {
    fn get(&mut self, key: &str) -> Option<Value> {
        // 1. メタデータ取得
        let meta = self.manifest.get_meta(key);
        if meta.is_empty() {
            return None;
        }

        // 2. パラメータ構築
        let params = self.build_params(key);

        // 3. _store設定取得
        let store_config = meta.get("_store").and_then(|v| v.as_object())?;
        let store_config_map: HashMap<String, Value> = store_config
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // 4. _storeから値を取得
        if let Some(value) = self.get_from_store(&store_config_map, &params) {
            return Some(value);
        }

        // 5. miss時は自動ロード
        if let Some(load_config_value) = meta.get("_load") {
            if let Some(load_config_obj) = load_config_value.as_object() {
                let load_config: HashMap<String, Value> = load_config_obj
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                if let Ok(loaded) = self.load.handle(&load_config, &params) {
                    // ロード成功 → _storeに保存
                    self.set_to_store(&store_config_map, &params, loaded.clone(), None);
                    return Some(loaded);
                }
            }
        }

        None
    }

    fn set(&mut self, key: &str, value: Value, ttl: Option<u64>) -> bool {
        // 1. メタデータ取得
        let meta = self.manifest.get_meta(key);
        if meta.is_empty() {
            return false;
        }

        // 2. パラメータ構築
        let params = self.build_params(key);

        // 3. _store設定取得
        let store_config = match meta.get("_store").and_then(|v| v.as_object()) {
            Some(config) => config,
            None => return false,
        };

        let store_config_map: HashMap<String, Value> = store_config
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // 4. _storeに保存
        self.set_to_store(&store_config_map, &params, value, ttl)
    }

    fn delete(&mut self, key: &str) -> bool {
        // 1. メタデータ取得
        let meta = self.manifest.get_meta(key);
        if meta.is_empty() {
            return false;
        }

        // 2. パラメータ構築
        let params = self.build_params(key);

        // 3. _store設定取得
        let store_config = match meta.get("_store").and_then(|v| v.as_object()) {
            Some(config) => config,
            None => return false,
        };

        let store_config_map: HashMap<String, Value> = store_config
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // 4. _storeから削除
        self.delete_from_store(&store_config_map, &params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Manifest as ManifestImpl;

    // Mock ProcessMemoryClient
    struct MockProcessMemory {
        data: HashMap<String, Value>,
    }

    impl MockProcessMemory {
        fn new() -> Self {
            Self {
                data: HashMap::new(),
            }
        }
    }

    impl ProcessMemoryClient for MockProcessMemory {
        fn get(&self, key: &str) -> Option<Value> {
            self.data.get(key).cloned()
        }

        fn set(&mut self, key: &str, value: Value) {
            self.data.insert(key.to_string(), value);
        }

        fn delete(&mut self, key: &str) -> bool {
            self.data.remove(key).is_some()
        }
    }

    // Mock KVSClient
    struct MockKVSClient {
        data: HashMap<String, Value>,
    }

    impl MockKVSClient {
        fn new() -> Self {
            Self {
                data: HashMap::new(),
            }
        }
    }

    impl KVSClient for MockKVSClient {
        fn get(&self, key: &str) -> Option<Value> {
            self.data.get(key).cloned()
        }

        fn set(&mut self, key: &str, value: Value, _ttl: Option<u64>) -> bool {
            self.data.insert(key.to_string(), value);
            true
        }

        fn delete(&mut self, key: &str) -> bool {
            self.data.remove(key).is_some()
        }

        fn exists(&self, key: &str) -> bool {
            self.data.contains_key(key)
        }
    }

    #[test]
    fn test_state_set_and_get() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/manifest");
        let mut manifest = ManifestImpl::new(manifest_path.to_str().unwrap());

        let load = Load::new();
        let mut process_memory = MockProcessMemory::new();

        let mut state = StateManager::new(&mut manifest, load)
            .with_process_memory(&mut process_memory);

        // connection.common.host に値をset
        let result = state.set(
            "connection.common.host",
            Value::String("test-host".to_string()),
            None,
        );
        assert!(result);

        // 値を取得
        let value = state.get("connection.common.host");
        assert_eq!(value, Some(Value::String("test-host".to_string())));
    }

    #[test]
    fn test_state_delete() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/manifest");
        let mut manifest = ManifestImpl::new(manifest_path.to_str().unwrap());

        let load = Load::new();
        let mut process_memory = MockProcessMemory::new();

        let mut state = StateManager::new(&mut manifest, load)
            .with_process_memory(&mut process_memory);

        // 値をset
        state.set(
            "connection.common.host",
            Value::String("test-host".to_string()),
            None,
        );

        // 削除
        let result = state.delete("connection.common.host");
        assert!(result);

        // 削除後はNoneが返る
        let value = state.get("connection.common.host");
        assert_eq!(value, None);
    }
}
