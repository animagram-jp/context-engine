// State - 統一CRUD実装
//
// manifest の _state/_store/_load に従って状態を管理する。

use crate::load::Load;
use crate::ports::provided::{Manifest as ManifestTrait, State as StateTrait};
use crate::ports::required::{KVSClient, ProcessMemoryClient};
use crate::common::PlaceholderResolver;
use serde_json::Value;
use std::collections::HashMap;

/// State 実装
///
/// _state/_store/_load メタデータに基づいて状態を管理する。
/// placeholder 解決、再帰制御、store管理を全て担当。
pub struct State<'a> {
    manifest: &'a mut dyn ManifestTrait,
    load: Load<'a>,
    process_memory: Option<&'a mut dyn ProcessMemoryClient>,
    kvs_client: Option<&'a mut dyn KVSClient>,
    recursion_depth: usize,
    max_recursion: usize,
}

impl<'a> State<'a> {
    /// 新しい State を作成
    pub fn new(manifest: &'a mut dyn ManifestTrait, load: Load<'a>) -> Self {
        Self {
            manifest,
            load,
            process_memory: None,
            kvs_client: None,
            recursion_depth: 0,
            max_recursion: 10,
        }
    }

    /// ProcessMemoryClient を設定
    pub fn with_process_memory(mut self, client: &'a mut dyn ProcessMemoryClient) -> Self {
        self.process_memory = Some(client);
        self
    }

    /// KVSClient を設定
    pub fn with_kvs_client(mut self, client: &'a mut dyn KVSClient) -> Self {
        self.kvs_client = Some(client);
        self
    }

    /// placeholder を namespace ルールで解決
    ///
    /// 解決順序:
    /// 1. 親層参照: ${org_id} → {parent}.org_id
    /// 2. 絶対パス: ${org_id} → org_id
    ///
    /// placeholder内の文字列を一切気にせず、単純に parent + name と name で試行。
    fn resolve_placeholder(&mut self, name: &str, context_key: &str) -> Option<Value> {
        // 1. 親層参照
        if let Some(parent) = context_key.rsplit_once('.').map(|(p, _)| p) {
            let parent_key = format!("{}.{}", parent, name);
            if let Some(value) = self.get(&parent_key) {
                return Some(value);
            }
        }

        // 2. 絶対パス
        self.get(name)
    }

    /// load_config 内の placeholder を解決
    fn resolve_load_config(
        &mut self,
        context_key: &str,
        load_config: &HashMap<String, Value>,
    ) -> HashMap<String, Value> {
        // config を Value に変換
        let config_map: serde_json::Map<String, Value> = load_config
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let config_value = Value::Object(config_map);

        // placeholder resolver
        let mut resolver = |placeholder_name: &str| -> Option<Value> {
            self.resolve_placeholder(placeholder_name, context_key)
        };

        // PlaceholderResolver で型付き解決
        let resolved_value = PlaceholderResolver::resolve_typed(config_value, &mut resolver);

        // HashMap に戻す
        if let Value::Object(map) = resolved_value {
            map.into_iter().collect()
        } else {
            HashMap::new()
        }
    }

    /// _store 設定から値を取得
    fn get_from_store(&self, store_config: &HashMap<String, Value>) -> Option<Value> {
        let client = store_config.get("client")?.as_str()?;

        match client {
            "InMemory" => {
                let process_memory = self.process_memory.as_ref()?;
                let key = store_config.get("key")?.as_str()?;
                process_memory.get(key)
            }
            "KVS" => {
                let kvs_client = self.kvs_client.as_ref()?;
                let key = store_config.get("key")?.as_str()?;
                kvs_client.get(key)
            }
            _ => None,
        }
    }

    /// _store 設定に値を保存
    fn set_to_store(
        &mut self,
        store_config: &HashMap<String, Value>,
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
                    if let Some(key) = store_config.get("key").and_then(|v| v.as_str()) {
                        process_memory.set(key, value);
                        return true;
                    }
                }
                false
            }
            "KVS" => {
                if let Some(kvs_client) = self.kvs_client.as_mut() {
                    if let Some(key) = store_config.get("key").and_then(|v| v.as_str()) {
                        let final_ttl =
                            ttl.or_else(|| store_config.get("ttl").and_then(|v| v.as_u64()));
                        return kvs_client.set(key, value, final_ttl);
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// _store 設定から値を削除
    fn delete_from_store(&mut self, store_config: &HashMap<String, Value>) -> bool {
        let client = match store_config.get("client").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return false,
        };

        match client {
            "InMemory" => {
                if let Some(process_memory) = self.process_memory.as_mut() {
                    if let Some(key) = store_config.get("key").and_then(|v| v.as_str()) {
                        return process_memory.delete(key);
                    }
                }
                false
            }
            "KVS" => {
                if let Some(kvs_client) = self.kvs_client.as_mut() {
                    if let Some(key) = store_config.get("key").and_then(|v| v.as_str()) {
                        return kvs_client.delete(key);
                    }
                }
                false
            }
            _ => false,
        }
    }
}

impl<'a> StateTrait for State<'a> {
    fn get(&mut self, key: &str) -> Option<Value> {
        // 再帰深度チェック
        if self.recursion_depth >= self.max_recursion {
            eprintln!(
                "State::get: max recursion depth ({}) reached for key '{}'",
                self.max_recursion, key
            );
            return None;
        }

        self.recursion_depth += 1;

        // 1. メタデータ取得
        let meta = self.manifest.get_meta(key);
        if meta.is_empty() {
            self.recursion_depth -= 1;
            return None;
        }

        // 2. _store から値を取得
        if let Some(store_config_value) = meta.get("_store") {
            if let Some(store_config_obj) = store_config_value.as_object() {
                let store_config: HashMap<String, Value> =
                    store_config_obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

                // store_config 内の placeholder を解決
                let resolved_store_config = self.resolve_load_config(key, &store_config);

                if let Some(value) = self.get_from_store(&resolved_store_config) {
                    self.recursion_depth -= 1;
                    return Some(value);
                }
            }
        }

        // 3. miss時は自動ロード
        let result = if let Some(load_config_value) = meta.get("_load") {
            if let Some(load_config_obj) = load_config_value.as_object() {
                let load_config: HashMap<String, Value> =
                    load_config_obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

                // placeholder 解決
                let resolved_config = self.resolve_load_config(key, &load_config);

                // Load 実行
                if let Ok(loaded) = self.load.handle(&resolved_config) {
                    // ロード成功 → _store に保存
                    if let Some(store_config_value) = meta.get("_store") {
                        if let Some(store_config_obj) = store_config_value.as_object() {
                            let store_config: HashMap<String, Value> = store_config_obj
                                .iter()
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect();

                            // store_config の placeholder も解決
                            let resolved_store_config = self.resolve_load_config(key, &store_config);
                            self.set_to_store(&resolved_store_config, loaded.clone(), None);
                        }
                    }
                    Some(loaded)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        self.recursion_depth -= 1;
        result
    }

    fn set(&mut self, key: &str, value: Value, ttl: Option<u64>) -> bool {
        // メタデータ取得
        let meta = self.manifest.get_meta(key);
        if meta.is_empty() {
            eprintln!("State::set: meta is empty for key '{}'", key);
            return false;
        }

        // _store 設定取得
        let store_config = match meta.get("_store").and_then(|v| v.as_object()) {
            Some(config) => config,
            None => {
                eprintln!("State::set: no _store config for key '{}'", key);
                return false;
            }
        };

        let store_config_map: HashMap<String, Value> =
            store_config.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        // store_config 内の placeholder 解決
        let resolved_store_config = self.resolve_load_config(key, &store_config_map);

        // _store に保存
        self.set_to_store(&resolved_store_config, value, ttl)
    }

    fn delete(&mut self, key: &str) -> bool {
        // メタデータ取得
        let meta = self.manifest.get_meta(key);
        if meta.is_empty() {
            return false;
        }

        // _store 設定取得
        let store_config = match meta.get("_store").and_then(|v| v.as_object()) {
            Some(config) => config,
            None => return false,
        };

        let store_config_map: HashMap<String, Value> =
            store_config.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        // store_config 内の placeholder 解決
        let resolved_store_config = self.resolve_load_config(key, &store_config_map);

        // _store から削除
        self.delete_from_store(&resolved_store_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Manifest;
    use crate::ports::required::ProcessMemoryClient;

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

    #[test]
    fn test_state_set_and_get() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/manifest");
        let mut manifest = Manifest::new(manifest_path.to_str().unwrap());

        let load = Load::new();
        let mut process_memory = MockProcessMemory::new();

        let mut state = State::new(&mut manifest, load).with_process_memory(&mut process_memory);

        // connection.common は InMemory で placeholder なし
        let mut conn_value = serde_json::Map::new();
        conn_value.insert("host".to_string(), Value::String("localhost".to_string()));
        let value = Value::Object(conn_value);

        let result = state.set("connection.common", value.clone(), None);
        assert!(result, "set should succeed");

        // get
        let retrieved = state.get("connection.common");
        assert_eq!(retrieved, Some(value));
    }

    #[test]
    fn test_state_delete() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/manifest");
        let mut manifest = Manifest::new(manifest_path.to_str().unwrap());

        let load = Load::new();
        let mut process_memory = MockProcessMemory::new();

        let mut state = State::new(&mut manifest, load).with_process_memory(&mut process_memory);

        // connection.common でテスト
        let value = Value::String("test_value".to_string());
        state.set("connection.common", value, None);

        // delete
        let result = state.delete("connection.common");
        assert!(result, "delete should succeed");

        // get (should be None)
        let retrieved = state.get("connection.common");
        assert_eq!(retrieved, None);
    }
}
