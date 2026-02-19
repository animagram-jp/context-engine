use crate::ports::provided::{Manifest as ManifestTrait, State as StateTrait};
use crate::ports::required::{KVSClient, InMemoryClient};
use crate::common::{DotString, DotMapAccessor, Placeholder};
use crate::store::Store;
use crate::load::Load;
use crate::method_log;
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

    /// owner_pathを算出（YAMLで_loadが定義されているnodeのqualified path）
    ///
    /// _load.map の最初のキーから逆算する。
    /// mapがない場合の挙動は is_load によって異なる:
    /// - is_load = true (Load処理): called_key をそのまま返す
    /// - is_load = false (Store処理): called_key の親パスを返す
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
                        // Load処理: called_key そのまま
                        DotString::new(called_key.as_str())
                    } else {
                        // Store処理: called_key の親パス
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

    fn resolve_config_placeholders(&mut self, config: &mut HashMap<String, Value>) {
        // Phase 1: placeholder名を収集（unique list）
        let placeholder_names: Vec<String> = Placeholder::collect(config);

        if placeholder_names.is_empty() {
            return; // placeholderがなければ何もしない
        }

        // Phase 2: cache hit分を resolved_values に収集、miss分を pending_paths に保持
        let mut resolved_values: HashMap<String, Value> = HashMap::new();
        let mut pending_paths: Vec<String> = Vec::new();

        for name in placeholder_names {
            let name_dot = DotString::new(&name);
            if let Some(cached) = self.dot_accessor.get(&self.cache, &name_dot) {
                // cache hit
                resolved_values.insert(name, cached.clone());
            } else {
                // cache miss
                pending_paths.push(name);
            }
        }

        // Phase 3: cache hit分を一括置換
        let _missing_after_cache = Placeholder::replace(config, &resolved_values);

        // Phase 4: cache miss分を順次 self.get() で解決
        // TODO: 呼び出し順序の最適化（依存関係を考慮）
        for path in pending_paths {
            if let Some(value) = self.get(&path) {
                resolved_values.insert(path, value);
            }
            // else: 解決できなかった（missing_keysとして残る）
        }

        // Phase 5: 新たに解決した値で再度置換
        let final_missing = Placeholder::replace(config, &resolved_values);

        // 未解決のplaceholderがあれば警告ログ出力
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
    fn get(&mut self, key: &str) -> Option<Value> {
        method_log!("State", "get", key);

        if self.called_keys.len() >= self.max_recursion {
            eprintln!(
                "State::get: max recursion depth ({}) reached for key '{}'",
                self.max_recursion, key
            );
            return None;
        }

        self.called_keys.push(DotString::new(key));

        // 1. check State.cache
        let current_key = self.called_keys.last().unwrap();
        if DotMapAccessor::has(&self.cache, current_key) {
            let cached = self.dot_accessor.get(&self.cache, current_key).cloned();
            self.called_keys.pop();
            return cached;
        }

        // 2. メタデータ取得
        let meta = self.manifest.get_meta(key);
        if meta.is_empty() {
            self.called_keys.pop();
            return None;
        }

        // 3. _load.client: State の場合は _store をスキップ
        let has_state_client = meta.get("_load")
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get("client"))
            .and_then(|v| v.as_str())
            == Some("State");

        // 4. _store から値を取得（client: State でない場合のみ）
        if !has_state_client {
            if let Some(store_config_value) = meta.get("_store") {
                if let Some(store_config_obj) = store_config_value.as_object() {
                    let mut store_config: HashMap<String, Value> =
                        store_config_obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

                    // store_config 内の placeholder 名を収集
                    let _placeholder_names = Placeholder::collect(&store_config);

                    // store_config 内の placeholder を解決
                    self.resolve_config_placeholders(&mut store_config);

                    if let Some(value) = self.store.get(&store_config) {
                        let owner_path = self.get_owner_path(&meta, false);

                        // owner_path で Manifest の静的値を cache にマージ
                        self.manifest.clear_missing_keys();
                        let manifest_value = self.manifest.get_value(&owner_path);
                        if self.manifest.get_missing_keys().is_empty() {
                            DotMapAccessor::merge(&mut self.cache, &owner_path, manifest_value);
                        }

                        // Store値のマージ方法を値の型によって変える
                        if value.is_object() {
                            // Object の場合: owner_path にマージ（上書き）
                            DotMapAccessor::merge(&mut self.cache, &owner_path, value);
                        } else {
                            // Scalar の場合: called_key に直接セット
                            let called_key = self.called_keys.last().unwrap();
                            DotMapAccessor::set(&mut self.cache, called_key, value);
                        }

                        // 要求されたフィールドを抽出
                        // has() でキーの存在を確認してから取得する（nullノードのキーを早期returnしない）
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

        // 5. miss時は自動ロード
        let result = if let Some(load_config_value) = meta.get("_load") {
            if let Some(load_config_obj) = load_config_value.as_object() {
                let mut load_config: HashMap<String, Value> =
                    load_config_obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

                // client が無い場合は自動ロードしない
                if !load_config.contains_key("client") {
                    self.called_keys.pop();
                    return None;
                }

                // load_config 内の placeholder 名を収集
                let _placeholder_names = Placeholder::collect(&load_config);

                // placeholder 解決
                self.resolve_config_placeholders(&mut load_config);

                // client: State の場合は key の値を直接返す（State内参照）
                let client_value = load_config.get("client").and_then(|v| v.as_str());

                if client_value == Some("State") {
                    // _load.key の値を返す（placeholder 解決済みの値）
                    if let Some(key_value) = load_config.get("key") {
                        // key_value は既にplaceholder解決済み
                        // インスタンスキャッシュに保存
                        let current_key = self.called_keys.last().unwrap();
                        DotMapAccessor::set(&mut self.cache, current_key, key_value.clone());

                        self.called_keys.pop();
                        return Some(key_value.clone());
                    }
                    self.called_keys.pop();
                    None
                } else {
                    // Load に渡す前に map を unqualify（qualified path → relative field name）
                    if let Some(map_value) = load_config.get("map") {
                        if let Value::Object(map_obj) = map_value {
                            let mut unqualified_map = serde_json::Map::new();
                            for (qualified_key, db_column) in map_obj {
                                // qualified pathから最後のセグメントを抽出（相対フィールド名）
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

                    // Load 実行
                    if let Ok(loaded) = self.load.handle(&load_config) {
                        let owner_path = self.get_owner_path(&meta, true);

                        // owner_path で Manifest の静的値を cache にマージ
                        self.manifest.clear_missing_keys();
                        let manifest_value = self.manifest.get_value(&owner_path);
                        if self.manifest.get_missing_keys().is_empty() {
                            DotMapAccessor::merge(&mut self.cache, &owner_path, manifest_value);
                        }

                        // owner_path で Load値を cache にマージ (上書き)
                        DotMapAccessor::merge(&mut self.cache, &owner_path, loaded.clone());

                        // ロード成功 → _store に保存（cache上のマージ済み値を保存）
                        if let Some(store_config_value) = meta.get("_store") {
                            if let Some(store_config_obj) = store_config_value.as_object() {
                                let mut store_config: HashMap<String, Value> = store_config_obj
                                    .iter()
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect();

                                // store_config 内の placeholder 名を収集
                                let _placeholder_names = Placeholder::collect(&store_config);

                                // store_config の placeholder も解決
                                self.resolve_config_placeholders(&mut store_config);

                                // cacheから保存する値を取得
                                if let Some(cache_value) = self.dot_accessor.get(&self.cache, &owner_path) {
                                    self.store.set(&store_config, cache_value.clone(), None);
                                }
                            }
                        }

                        // 要求されたフィールドを抽出して返す
                        // has() でキーの存在を確認してから取得する（nullノードのキーを早期returnしない）
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

    fn set(&mut self, key: &str, value: Value, ttl: Option<u64>) -> bool {
        method_log!("State", "set", key);

        // DotString を生成して call stack に追加
        self.called_keys.push(DotString::new(key));

        // メタデータ取得
        let meta = self.manifest.get_meta(key);
        if meta.is_empty() {
            eprintln!("State::set: meta is empty for key '{}'", key);
            self.called_keys.pop();
            return false;
        }

        // _store 設定取得
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

        // store_config 内の placeholder 名を収集
        let _placeholder_names = Placeholder::collect(&store_config_map);

        // store_config 内の placeholder 解決
        self.resolve_config_placeholders(&mut store_config_map);

        // 1. owner_path を算出
        let owner_path = self.get_owner_path(&meta, false);

        // 2. cache に owner_path の値がなければ、Store から取得して cache にロード
        if self.dot_accessor.get(&self.cache, &owner_path).is_none() {
            if let Some(store_value) = self.store.get(&store_config_map) {
                DotMapAccessor::merge(&mut self.cache, &owner_path, store_value);
            }
        }

        // 3. cache 上で新しい値を設定（DotMapAccessor が全ての作業を行う）
        let called_key = self.called_keys.last().unwrap();
        DotMapAccessor::set(&mut self.cache, called_key, value);

        // 4. cache から owner_path の親オブジェクト全体を取得
        let store_value = self.dot_accessor.get(&self.cache, &owner_path)
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

        // 5. 親オブジェクト全体を _store に保存
        let result = self.store.set(&store_config_map, store_value, ttl);

        self.called_keys.pop();
        result
    }

    fn delete(&mut self, key: &str) -> bool {
        method_log!("State", "delete", key);

        self.called_keys.push(DotString::new(key));

        // メタデータ取得
        let meta = self.manifest.get_meta(key);
        if meta.is_empty() {
            self.called_keys.pop();
            return false;
        }

        // _store 設定取得
        let store_config = match meta.get("_store").and_then(|v| v.as_object()) {
            Some(config) => config,
            None => {
                self.called_keys.pop();
                return false;
            }
        };

        let mut store_config_map: HashMap<String, Value> =
            store_config.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        // store_config 内の placeholder 名を収集
        let _placeholder_names = Placeholder::collect(&store_config_map);

        // store_config 内の placeholder 解決
        self.resolve_config_placeholders(&mut store_config_map);

        // 1. owner_path を算出
        let owner_path = self.get_owner_path(&meta, false);

        // 2. key が owner_path と同じ場合は親オブジェクト全体を削除
        if key == owner_path.as_str() {
            // 親オブジェクト全体を削除
            let result = self.store.delete(&store_config_map);
            if result {
                let called_key = self.called_keys.last().unwrap();
                DotMapAccessor::unset(&mut self.cache, called_key);
            }
            self.called_keys.pop();
            return result;
        }

        // 3. 子フィールドの削除: cache に owner_path の値がなければ、Store から取得して cache にロード
        if self.dot_accessor.get(&self.cache, &owner_path).is_none() {
            if let Some(store_value) = self.store.get(&store_config_map) {
                DotMapAccessor::merge(&mut self.cache, &owner_path, store_value);
            }
        }

        // 4. cache のバックアップ（ロールバック用）
        let cache_backup = self.cache.clone();

        // 5. cache 上で子フィールドを削除（DotMapAccessor が全ての作業を行う）
        let called_key = self.called_keys.last().unwrap();
        DotMapAccessor::unset(&mut self.cache, called_key);

        // 6. cache から owner_path の親オブジェクト全体を取得
        let store_value = self.dot_accessor.get(&self.cache, &owner_path)
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

        // 7. 親オブジェクト全体を _store に保存
        let result = self.store.set(&store_config_map, store_value, None);

        // 8. 失敗時はロールバック
        if !result {
            self.cache = cache_backup;
        }

        // TODO: 空オブジェクトになった場合の処理（保留）
        // 全ての子フィールドを削除して空配列になった時に owner_path も削除する

        self.called_keys.pop();
        result
    }

    fn exists(&mut self, key: &str) -> bool {
        method_log!("State", "exists", key);

        // DotString を生成して call stack に追加
        self.called_keys.push(DotString::new(key));

        // 1. インスタンスキャッシュをチェック（最優先・最速）
        let current_key = self.called_keys.last().unwrap();
        if DotMapAccessor::has(&self.cache, current_key) {
            self.called_keys.pop();
            return true;
        }

        // 2. メタデータ取得
        let meta = self.manifest.get_meta(key);
        if meta.is_empty() {
            self.called_keys.pop();
            return false;
        }

        // 3. _store 設定取得
        let store_config = match meta.get("_store").and_then(|v| v.as_object()) {
            Some(config) => config,
            None => {
                self.called_keys.pop();
                return false;
            }
        };

        let mut store_config_map: HashMap<String, Value> =
            store_config.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        // 4. store_config 内の placeholder 名を収集
        let _placeholder_names = Placeholder::collect(&store_config_map);

        // 5. store_config 内の placeholder 解決
        self.resolve_config_placeholders(&mut store_config_map);

        // 6. _store から値を取得（自動ロードなし）
        let result = self.store.get(&store_config_map).is_some();

        self.called_keys.pop();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Manifest;
    use crate::ports::required::InMemoryClient;

    // Mock InMemoryClient
    struct MockInMemory {
        data: HashMap<String, Value>,
    }

    impl MockInMemory {
        fn new() -> Self {
            Self {
                data: HashMap::new(),
            }
        }
    }

    impl InMemoryClient for MockInMemory {
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
            .join("examples/manifest");
        let mut manifest = Manifest::new(manifest_path.to_str().unwrap());

        let load = Load::new();
        let mut in_memory = MockInMemory::new();

        let mut state = State::new(&mut manifest, load).with_in_memory(&mut in_memory);

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
            .join("examples/manifest");
        let mut manifest = Manifest::new(manifest_path.to_str().unwrap());

        let load = Load::new();
        let mut in_memory = MockInMemory::new();

        let mut state = State::new(&mut manifest, load).with_in_memory(&mut in_memory);

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
