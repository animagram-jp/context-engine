// State impl
use crate::load::Load;
use crate::ports::provided::{Manifest as ManifestTrait, State as StateTrait};
use crate::ports::required::{KVSClient, InMemoryClient};
use crate::common::{DotMapAccessor, DotString, PlaceholderResolver};
use serde_json::Value;
use std::collections::HashMap;

/// State impl
///
/// depends on: Manifest YAML meta data(_state, _store, _load)
/// func: placeholder resolution, self calling, store CRUD
pub struct State<'a> {
    dot_accessor: DotMapAccessor,
    manifest: &'a mut dyn ManifestTrait,
    load: Load<'a>,
    in_memory: Option<&'a mut dyn InMemoryClient>,
    kvs_client: Option<&'a mut dyn KVSClient>,
    max_recursion: usize,
    called_keys: Vec<DotString>,  // call stack の DotString（recursion_depth は len() で代用）
    cache: Value,  // instance cache（single collection object）
}

impl<'a> State<'a> {
    /// create a new State instance
    pub fn new(manifest: &'a mut dyn ManifestTrait, load: Load<'a>) -> Self {
        Self {
            manifest,
            load,
            in_memory: None,
            kvs_client: None,
            max_recursion: 10,
            called_keys: Vec::new(),
            cache: Value::Object(serde_json::Map::new()),
            dot_accessor: DotMapAccessor::new(),
        }
    }

    /// move InMemoryClient ownership
    pub fn with_in_memory(mut self, client: &'a mut dyn InMemoryClient) -> Self {
        self.in_memory = Some(client);
        self
    }

    /// move KVSClient ownership
    pub fn with_kvs_client(mut self, client: &'a mut dyn KVSClient) -> Self {
        self.kvs_client = Some(client);
        self
    }

    /// storeから取得した値から、要求された子フィールドを抽出
    ///
    /// key="cache.user.org_id" で _store が "cache.user" レベルで定義されている場合、
    /// storeには user オブジェクト全体が保存されているため、"org_id" フィールドを抽出する。
    fn extract_field_from_value(key: &DotString, value: Value, meta: &HashMap<String, Value>) -> Value {
        // metaに _store が含まれている場合、直接マッチしたので値をそのまま返す
        // （cache.user を取得した場合）
        if let Some(store_meta) = meta.get("_store") {
            if let Some(_store_obj) = store_meta.as_object() {
                // _store の定義が継承でなく、このレベルで直接定義されているか確認
                // ここでは簡易的に、keyの最後の部分とvalueの構造で判断
                // cache.user → value全体を返す
                // cache.user.org_id → valueから org_id を抽出

                if key.len() < 2 {
                    return value;
                }

                // 最後の部分が value の中のフィールドとして存在するか確認
                let last_field = &key[key.len() - 1];
                if let Some(field_value) = value.get(last_field) {
                    // 子フィールドが存在する → 抽出して返す
                    return field_value.clone();
                }
            }
        }

        // それ以外は値をそのまま返す
        value
    }

    /// placeholder を解決
    ///
    /// Manifestが既に完全修飾パス(manifestDir path)を返すため、
    /// nameをそのままkeyとして使用
    fn resolve_placeholder(&mut self, name: &str) -> Option<Value> {
        // キャッシュを優先チェック（高速パス）
        let name_dot = DotString::new(name);
        if let Some(cached) = self.dot_accessor.get(&self.cache, &name_dot) {
            return Some(cached.clone());
        }

        // キャッシュミス時はフル処理
        self.get(name)
    }

    /// load_config 内の placeholder を解決
    fn resolve_load_config(&mut self, load_config: &HashMap<String, Value>) -> HashMap<String, Value> {
        let config_value = Value::Object(load_config.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        let mut resolver = |placeholder_name: &str| -> Option<Value> {
            self.resolve_placeholder(placeholder_name)
        };
        let resolved_value = PlaceholderResolver::resolve_typed(config_value, &mut resolver);
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
                let in_memory = self.in_memory.as_ref()?;
                let key = store_config.get("key")?.as_str()?;
                in_memory.get(key)
            }
            "KVS" => {
                let kvs_client = self.kvs_client.as_ref()?;
                let key = store_config.get("key")?.as_str()?;
                let value_str = kvs_client.get(key)?;

                // deserialize処理
                // 全ての値はJSON形式で保存されている（型情報保持）
                // JSON parse → Number/String/Bool/Null/Array/Objectを正確に復元
                serde_json::from_str(&value_str).ok()
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
                if let Some(in_memory) = self.in_memory.as_mut() {
                    if let Some(key) = store_config.get("key").and_then(|v| v.as_str()) {
                        in_memory.set(key, value);
                        return true;
                    }
                }
                false
            }
            "KVS" => {
                if let Some(kvs_client) = self.kvs_client.as_mut() {
                    if let Some(key) = store_config.get("key").and_then(|v| v.as_str()) {
                        // serialize処理
                        // 全ての値をJSON形式で保存（型情報を保持）
                        // JSON内でNumber/String/Bool/Null/Array/Objectを区別
                        let serialized = match serde_json::to_string(&value) {
                            Ok(s) => s,
                            Err(_) => return false,
                        };

                        let final_ttl =
                            ttl.or_else(|| store_config.get("ttl").and_then(|v| v.as_u64()));
                        return kvs_client.set(key, serialized, final_ttl);
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
                if let Some(in_memory) = self.in_memory.as_mut() {
                    if let Some(key) = store_config.get("key").and_then(|v| v.as_str()) {
                        return in_memory.delete(key);
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

    /// 親キーを取得
    ///
    /// "cache.user.org_id" → "cache.user"
    /// "cache.user" → "cache"
    /// "cache" → ""
    fn get_parent_key(dot_string: &DotString) -> String {
        if dot_string.len() <= 1 {
            return String::new();
        }
        dot_string[..dot_string.len() - 1].join(".")
    }

    /// Manifest の静的値とマージ
    ///
    /// Load/Store から取得した値に Manifest の静的値をマージする。
    /// Load/Store の値が優先される（後から上書き）。
    ///
    /// # Arguments
    /// * `manifest_key` - Manifest から取得するキー
    /// * `data` - Load/Store から取得した値
    ///
    /// # Returns
    /// マージ後の値（data が Object でない場合はそのまま返す）
    fn merge_with_manifest_static_values(&mut self, manifest_key: &str, data: Value) -> Value {
        // data が Object でない場合はマージ不要
        let Value::Object(data_obj) = data else {
            return data;
        };

        // Manifest から静的値を取得（メタデータとnullを除外）
        let manifest_key_dotstring = DotString::new(manifest_key);
        let manifest_value = self.manifest.get_value(&manifest_key_dotstring);

        // Manifest の値が Object でない場合はマージ不要
        let manifest_obj = match manifest_value {
            Value::Object(obj) => obj,
            Value::Null => return Value::Object(data_obj),
            _ => return Value::Object(data_obj),
        };

        // Manifest の静的値を先に入れ、data で上書き（data が優先）
        let mut merged = manifest_obj.clone();
        for (key, value) in data_obj {
            merged.insert(key, value);
        }

        Value::Object(merged)
    }
}

impl<'a> StateTrait for State<'a> {
    fn get(&mut self, key: &str) -> Option<Value> {
        // 再帰深度チェック
        if self.called_keys.len() >= self.max_recursion {
            eprintln!(
                "State::get: max recursion depth ({}) reached for key '{}'",
                self.max_recursion, key
            );
            return None;
        }

        // DotString を生成して call stack に追加
        self.called_keys.push(DotString::new(key));

        // 1. インスタンスキャッシュをチェック（最優先）
        let current_key = self.called_keys.last().unwrap();
        if let Some(cached) = self.dot_accessor.get(&self.cache, current_key) {
            self.called_keys.pop();
            return Some(cached.clone());
        }

        // 2. メタデータ取得
        let meta = self.manifest.get_meta(key);
        if meta.is_empty() {
            self.called_keys.pop();
            return None;
        }

        // 3. _load.client: State の場合は _store をスキップ
        //    明示的なState参照は親の _store を使わない
        let has_state_client = meta.get("_load")
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get("client"))
            .and_then(|v| v.as_str())
            == Some("State");

        // 4. _store から値を取得（client: State でない場合のみ）
        if !has_state_client {
            if let Some(store_config_value) = meta.get("_store") {
                if let Some(store_config_obj) = store_config_value.as_object() {
                    let store_config: HashMap<String, Value> =
                        store_config_obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

                    // store_config 内の placeholder を解決
                    let resolved_store_config = self.resolve_load_config(&store_config);

                    if let Some(value) = self.get_from_store(&resolved_store_config) {
                        // map の最初のキーから owner_key を逆算
                        let owner_key = meta.get("_load")
                            .and_then(|v| v.as_object())
                            .and_then(|obj| obj.get("map"))
                            .and_then(|v| v.as_object())
                            .and_then(|map| map.keys().next())
                            .map(|first_key| Self::get_parent_key(&DotString::new(first_key)))
                            .unwrap_or_else(|| {
                                if let Some(current_key) = self.called_keys.last() {
                                    Self::get_parent_key(current_key)
                                } else {
                                    String::new()
                                }
                            });

                        // owner_key で Manifest の静的値とマージ
                        let merged_value = self.merge_with_manifest_static_values(&owner_key, value);

                        // owner_key で cache にマージ
                        let owner_key_dot = DotString::new(&owner_key);
                        DotMapAccessor::merge(&mut self.cache, &owner_key_dot, merged_value.clone());

                        // 要求されたフィールドを抽出
                        let extracted = if key == owner_key {
                            merged_value
                        } else {
                            let current_key = self.called_keys.last().unwrap();
                            Self::extract_field_from_value(current_key, merged_value, &meta)
                        };

                        self.called_keys.pop();
                        return Some(extracted);
                    }
                }
            }
        }

        // 5. miss時は自動ロード
        let result = if let Some(load_config_value) = meta.get("_load") {
            if let Some(load_config_obj) = load_config_value.as_object() {
                let load_config: HashMap<String, Value> =
                    load_config_obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

                // client が無い場合は自動ロードしない
                if !load_config.contains_key("client") {
                    self.called_keys.pop();
                    return None;
                }

                // placeholder 解決
                let mut resolved_config = self.resolve_load_config(&load_config);

                // client: State の場合は key の値を直接返す（State内参照）
                let client_value = resolved_config.get("client").and_then(|v| v.as_str());

                if client_value == Some("State") {
                    // _load.key の値を返す（placeholder 解決済みの値）
                    if let Some(key_value) = resolved_config.get("key") {
                        // key_value は既に resolve_load_config で値に解決されている
                        // インスタンスキャッシュに保存
                        let current_key = self.called_keys.last().unwrap();
                        DotMapAccessor::set(&mut self.cache, current_key, key_value.clone());

                        self.called_keys.pop();
                        return Some(key_value.clone());
                    }
                    self.called_keys.pop();
                    None
                } else {
                    // Load に渡す前に map を denormalize（絶対パス → 相対パス）
                    if let Some(map_value) = resolved_config.get("map") {
                        if let Value::Object(map_obj) = map_value {
                            let mut denormalized_map = serde_json::Map::new();
                            for (absolute_key, db_column) in map_obj {
                                // 絶対パスから最後のセグメントを抽出（相対フィールド名）
                                let dot_key = DotString::new(absolute_key);
                                if dot_key.len() > 0 {
                                    let relative_key = &dot_key[dot_key.len() - 1];
                                    denormalized_map.insert(relative_key.to_string(), db_column.clone());
                                }
                            }
                            resolved_config.insert("map".to_string(), Value::Object(denormalized_map));
                        }
                    }

                    // Load 実行
                    if let Ok(loaded) = self.load.handle(&resolved_config) {
                        // map の最初のキーから owner_key を逆算
                        // declare-e L120-130: map から所有者キーを算出
                        let owner_key = meta.get("_load")
                            .and_then(|v| v.as_object())
                            .and_then(|obj| obj.get("map"))
                            .and_then(|v| v.as_object())
                            .and_then(|map| map.keys().next())
                            .map(|first_key| Self::get_parent_key(&DotString::new(first_key)))
                            .unwrap_or_else(|| {
                                if let Some(current_key) = self.called_keys.last() {
                                    current_key.as_str().to_string()
                                } else {
                                    String::new()
                                }
                            });

                        // owner_key で Manifest の静的値とマージ
                        let merged_loaded = self.merge_with_manifest_static_values(&owner_key, loaded);

                        // ロード成功 → _store に保存（マージ後の値を保存）
                        if let Some(store_config_value) = meta.get("_store") {
                            if let Some(store_config_obj) = store_config_value.as_object() {
                                let store_config: HashMap<String, Value> = store_config_obj
                                    .iter()
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect();

                                // store_config の placeholder も解決
                                let resolved_store_config = self.resolve_load_config(&store_config);
                                self.set_to_store(&resolved_store_config, merged_loaded.clone(), None);
                            }
                        }

                        // owner_key で cache にマージ
                        let owner_key_dot = DotString::new(&owner_key);
                        DotMapAccessor::merge(&mut self.cache, &owner_key_dot, merged_loaded.clone());

                        // 要求されたフィールドを抽出して返す
                        if key == owner_key {
                            // 親辞書そのものを取得した場合
                            Some(merged_loaded)
                        } else {
                            // 子フィールドを取得した場合
                            let current_key = self.called_keys.last().unwrap();
                            Some(Self::extract_field_from_value(current_key, merged_loaded, &meta))
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

        let store_config_map: HashMap<String, Value> =
            store_config.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        // store_config 内の placeholder 解決
        let resolved_store_config = self.resolve_load_config(&store_config_map);

        // 1. owner_key を特定（_store が定義されているレベル）
        // map の最初のキーから owner_key を逆算
        // declare-engine L120-130: map から所有者キーを算出
        let owner_key = meta.get("_load")
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get("map"))
            .and_then(|v| v.as_object())
            .and_then(|map| map.keys().next())
            .map(|first_key| Self::get_parent_key(&DotString::new(first_key)))
            .unwrap_or_else(|| {
                if let Some(current_key) = self.called_keys.last() {
                    Self::get_parent_key(current_key)
                } else {
                    String::new()
                }
            });

        // 2. cache に owner_key の値がなければ、Store から取得して cache にロード
        let owner_key_dot = DotString::new(&owner_key);
        if self.dot_accessor.get(&self.cache, &owner_key_dot).is_none() {
            if let Some(store_value) = self.get_from_store(&resolved_store_config) {
                DotMapAccessor::merge(&mut self.cache, &owner_key_dot, store_value);
            }
        }

        // 3. cache 上で新しい値を設定（DotMapAccessor が全ての作業を行う）
        let current_key = self.called_keys.last().unwrap();
        DotMapAccessor::set(&mut self.cache, current_key, value);

        // 4. cache から owner_key の親オブジェクト全体を取得
        let store_value = self.dot_accessor.get(&self.cache, &owner_key_dot)
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

        // 5. 親オブジェクト全体を _store に保存
        let result = self.set_to_store(&resolved_store_config, store_value, ttl);

        self.called_keys.pop();
        result
    }

    fn delete(&mut self, key: &str) -> bool {
        // DotString を生成して call stack に追加
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

        let store_config_map: HashMap<String, Value> =
            store_config.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        // store_config 内の placeholder 解決
        let resolved_store_config = self.resolve_load_config(&store_config_map);

        // 1. owner_key を特定（_store が定義されているレベル）
        let owner_key = meta.get("_load")
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get("map"))
            .and_then(|v| v.as_object())
            .and_then(|map| map.keys().next())
            .map(|first_key| Self::get_parent_key(&DotString::new(first_key)))
            .unwrap_or_else(|| {
                if let Some(current_key) = self.called_keys.last() {
                    Self::get_parent_key(current_key)
                } else {
                    String::new()
                }
            });

        // 2. key が owner_key と同じ場合は親オブジェクト全体を削除
        let owner_key_dot = DotString::new(&owner_key);
        if key == owner_key {
            // 親オブジェクト全体を削除
            let result = self.delete_from_store(&resolved_store_config);
            if result {
                let current_key = self.called_keys.last().unwrap();
                DotMapAccessor::unset(&mut self.cache, current_key);
            }
            self.called_keys.pop();
            return result;
        }

        // 3. 子フィールドの削除: cache に owner_key の値がなければ、Store から取得して cache にロード
        if self.dot_accessor.get(&self.cache, &owner_key_dot).is_none() {
            if let Some(store_value) = self.get_from_store(&resolved_store_config) {
                DotMapAccessor::merge(&mut self.cache, &owner_key_dot, store_value);
            }
        }

        // 4. cache のバックアップ（ロールバック用）
        let cache_backup = self.cache.clone();

        // 5. cache 上で子フィールドを削除（DotMapAccessor が全ての作業を行う）
        let current_key = self.called_keys.last().unwrap();
        DotMapAccessor::unset(&mut self.cache, current_key);

        // 6. cache から owner_key の親オブジェクト全体を取得
        let store_value = self.dot_accessor.get(&self.cache, &owner_key_dot)
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

        // 7. 親オブジェクト全体を _store に保存
        let result = self.set_to_store(&resolved_store_config, store_value, None);

        // 8. 失敗時はロールバック
        if !result {
            self.cache = cache_backup;
        }

        // TODO: 空オブジェクトになった場合の処理（保留）
        // 全ての子フィールドを削除して空配列になった時に owner_key も削除する

        self.called_keys.pop();
        result
    }

    fn exists(&mut self, key: &str) -> bool {
        // DotString を生成して call stack に追加
        self.called_keys.push(DotString::new(key));

        // 1. インスタンスキャッシュをチェック（最優先・最速）
        let current_key = self.called_keys.last().unwrap();
        if self.dot_accessor.get(&self.cache, current_key).is_some() {
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

        let store_config_map: HashMap<String, Value> =
            store_config.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        // 4. store_config 内の placeholder 解決
        let resolved_store_config = self.resolve_load_config(&store_config_map);

        // 5. _store から値を取得（自動ロードなし）
        let result = self.get_from_store(&resolved_store_config).is_some();

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
