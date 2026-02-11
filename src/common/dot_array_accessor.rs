// DotArrayAccessor
// ドット記法での配列アクセスを提供
//
// PHPのDotArrayAccessorを完全再現
// - missingKeys追跡機能
// - 階層構造の自動作成（set時）
// - インスタンスメソッド（get, getMissingKeys, clearMissingKeys）
// - 静的メソッド（set, merge, unset）

use serde_json::Value;

/// ドット記法でのデータアクセスを提供
pub struct DotArrayAccessor {
    missing_keys: Vec<String>,
}

impl DotArrayAccessor {
    /// 新しいインスタンスを作成
    pub fn new() -> Self {
        Self {
            missing_keys: Vec::new(),
        }
    }

    /// ドット記法で値を取得（missingKeys追跡付き）
    ///
    /// 例: get(&data, "user.profile.name")
    ///
    /// キーが見つからない場合はmissingKeysに記録してNoneを返す
    pub fn get<'a>(&mut self, data: &'a Value, key: &str) -> Option<&'a Value> {
        // ドットが無い場合は単純なキーアクセス
        if !key.contains('.') {
            if let Some(obj) = data.as_object() {
                if !obj.contains_key(key) {
                    self.missing_keys.push(key.to_string());
                    return None;
                }
                return obj.get(key);
            } else {
                self.missing_keys.push(key.to_string());
                return None;
            }
        }

        // ドット記法のパスを分解
        let segments: Vec<&str> = key.split('.').collect();
        let mut current = data;

        for segment in segments {
            match current.get(segment) {
                Some(next) => current = next,
                None => {
                    self.missing_keys.push(key.to_string());
                    return None;
                }
            }
        }

        Some(current)
    }

    /// 取得失敗したキーの一覧を返す
    pub fn get_missing_keys(&self) -> &[String] {
        &self.missing_keys
    }

    /// missingKeysをクリア
    pub fn clear_missing_keys(&mut self) {
        self.missing_keys.clear();
    }

    /// ドット記法で値を設定（静的メソッド）
    ///
    /// 例: set(&mut data, "user.profile.name", Value::String("Alice".to_string()))
    ///
    /// 存在しないパスは自動的にObjectとして作成される
    pub fn set(data: &mut Value, key: &str, value: Value) {
        // ドットが無い場合は単純な設定
        if !key.contains('.') {
            if let Some(obj) = data.as_object_mut() {
                obj.insert(key.to_string(), value);
            } else {
                // dataがObjectでない場合、新しいObjectを作成
                let mut new_obj = serde_json::Map::new();
                new_obj.insert(key.to_string(), value);
                *data = Value::Object(new_obj);
            }
            return;
        }

        // ドット記法のパスを分解
        let segments: Vec<&str> = key.split('.').collect();

        // dataがObjectでない場合、新しいObjectを作成
        if !data.is_object() {
            *data = Value::Object(serde_json::Map::new());
        }

        let mut current = data;

        let last_idx = segments.len() - 1;

        for (i, segment) in segments.iter().enumerate() {
            if i == last_idx {
                // 最後のセグメント：値を設定
                if let Some(obj) = current.as_object_mut() {
                    obj.insert(segment.to_string(), value);
                }
                return;
            }

            // 中間パス：存在しないか、Objectでない場合は新規作成してから移動
            // Borrowチェッカーを満たすため、明示的にスコープを分ける
            {
                let obj = current.as_object_mut().expect("current must be an object");
                if !obj.contains_key(*segment) || !obj[*segment].is_object() {
                    obj.insert(segment.to_string(), Value::Object(serde_json::Map::new()));
                }
            }

            // 次の階層へ移動（新しいスコープで借用）
            current = current.get_mut(*segment).expect("segment must exist");
        }
    }

    /// 値をマージ（静的メソッド）
    ///
    /// 例: merge(&mut data, "user.profile", json!({"age": 30}))
    ///
    /// 注意: マージ処理の各レベルで、既存値と新しい値の少なくとも一方がスカラー（非オブジェクト）である場合、上書き処理がされる。
    /// state object では、末尾のノードは値に null を持って、末尾の値と区別されている。
    /// このため、scalar と list の object は、自動的に上書き処理してよい。
    pub fn merge(data: &mut Value, key: &str, value: Value) {
        // ドットが無い場合
        if !key.contains('.') {
            // data[key] と value の両方がオブジェクトの場合は再帰マージ
            if let Some(obj) = data.as_object_mut() {
                let should_merge = if let Some(existing) = obj.get(key) {
                    existing.is_object() && value.is_object()
                } else {
                    false
                };

                if should_merge {
                    // 両方がオブジェクト → 再帰的にマージ
                    if let Some(value_obj) = value.as_object() {
                        for (k, v) in value_obj {
                            if let Some(existing) = obj.get_mut(key) {
                                let should_recurse = if let Some(existing_obj) = existing.as_object() {
                                    existing_obj.contains_key(k) && existing_obj[k].is_object() && v.is_object()
                                } else {
                                    false
                                };

                                if should_recurse {
                                    // 既存のキーがあり、両方がオブジェクト → 再帰呼び出し
                                    Self::merge(existing, k, v.clone());
                                } else {
                                    // それ以外は上書き
                                    if let Some(existing_obj_mut) = existing.as_object_mut() {
                                        existing_obj_mut.insert(k.clone(), v.clone());
                                    }
                                }
                            }
                        }
                    }
                    return;
                }
                // 既存値がない、またはどちらかがオブジェクトでない → 上書き
                obj.insert(key.to_string(), value);
            } else {
                // data がオブジェクトでない場合、新しいオブジェクトを作成
                let mut new_obj = serde_json::Map::new();
                new_obj.insert(key.to_string(), value);
                *data = Value::Object(new_obj);
            }
            return;
        }

        // ドット記法のパスを分解
        let segments: Vec<&str> = key.split('.').collect();
        let first_segment = segments[0];
        let remaining_key = segments[1..].join(".");

        // data がオブジェクトでない場合、新しいオブジェクトを作成
        if !data.is_object() {
            *data = Value::Object(serde_json::Map::new());
        }

        // 最初のセグメントが存在しない場合は作成
        if let Some(obj) = data.as_object_mut() {
            if !obj.contains_key(first_segment) {
                obj.insert(first_segment.to_string(), Value::Object(serde_json::Map::new()));
            }

            // 再帰的にマージ
            if let Some(next) = obj.get_mut(first_segment) {
                Self::merge(next, &remaining_key, value);
            }
        }
    }

    /// キーが存在するかチェック（静的メソッド）
    ///
    /// 例: has(&data, "user.profile.name")
    ///
    /// ドット記法でのパスを辿り、最後のキーまで存在するか確認する
    pub fn has(data: &Value, key: &str) -> bool {
        // ドットが無い場合は単純なキー存在チェック
        if !key.contains('.') {
            if let Some(obj) = data.as_object() {
                return obj.contains_key(key);
            }
            return false;
        }

        // ドット記法のパスを分解
        let segments: Vec<&str> = key.split('.').collect();
        let mut current = data;

        for segment in segments {
            match current.get(segment) {
                Some(next) => current = next,
                None => return false,
            }
        }

        true
    }

    /// 値を削除（静的メソッド）
    ///
    /// 例: unset(&mut data, "user.profile.name")
    pub fn unset(data: &mut Value, key: &str) {
        // ドットが無い場合は単純な削除
        if !key.contains('.') {
            if let Some(obj) = data.as_object_mut() {
                obj.remove(key);
            }
            return;
        }

        // ドット記法のパスを分解
        let segments: Vec<&str> = key.split('.').collect();
        let mut current = data;

        for (i, segment) in segments.iter().enumerate() {
            let is_last = i == segments.len() - 1;

            if is_last {
                // 最後のセグメント：削除
                if let Some(obj) = current.as_object_mut() {
                    obj.remove(*segment);
                }
                return;
            }

            // 中間パス：次の階層へ移動
            // Borrowチェック回避のため、存在チェックと取得を分離
            if !current.is_object() {
                return;
            }

            let has_next = if let Some(obj) = current.as_object() {
                obj.contains_key(*segment) && obj.get(*segment).map_or(false, |v| v.is_object())
            } else {
                false
            };

            if !has_next {
                // パスが存在しない場合は何もしない
                return;
            }

            // 次の階層へ移動
            current = current.get_mut(*segment).unwrap();
        }
    }
}

impl Default for DotArrayAccessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_get_simple_key() {
        let mut accessor = DotArrayAccessor::new();
        let data = json!({
            "name": "Alice"
        });

        let result = accessor.get(&data, "name");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), &json!("Alice"));
        assert_eq!(accessor.get_missing_keys().len(), 0);
    }

    #[test]
    fn test_get_nested_key() {
        let mut accessor = DotArrayAccessor::new();
        let data = json!({
            "user": {
                "profile": {
                    "name": "Alice"
                }
            }
        });

        let result = accessor.get(&data, "user.profile.name");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), &json!("Alice"));
        assert_eq!(accessor.get_missing_keys().len(), 0);
    }

    #[test]
    fn test_get_missing_key_tracking() {
        let mut accessor = DotArrayAccessor::new();
        let data = json!({
            "user": {
                "name": "Alice"
            }
        });

        let result = accessor.get(&data, "user.age");
        assert!(result.is_none());
        assert_eq!(accessor.get_missing_keys(), vec!["user.age"]);

        // 2回目の失敗
        let result2 = accessor.get(&data, "user.email");
        assert!(result2.is_none());
        assert_eq!(accessor.get_missing_keys(), vec!["user.age", "user.email"]);

        // クリア
        accessor.clear_missing_keys();
        assert_eq!(accessor.get_missing_keys().len(), 0);
    }

    #[test]
    fn test_set_simple_key() {
        let mut data = json!({});
        DotArrayAccessor::set(&mut data, "name", json!("Alice"));

        assert_eq!(data, json!({"name": "Alice"}));
    }

    #[test]
    fn test_set_nested_key() {
        let mut data = json!({});
        DotArrayAccessor::set(&mut data, "user.profile.name", json!("Alice"));

        assert_eq!(data, json!({
            "user": {
                "profile": {
                    "name": "Alice"
                }
            }
        }));
    }

    #[test]
    fn test_set_overwrites_existing() {
        let mut data = json!({
            "user": {
                "name": "Alice"
            }
        });

        DotArrayAccessor::set(&mut data, "user.name", json!("Bob"));

        assert_eq!(data["user"]["name"], json!("Bob"));
    }

    #[test]
    fn test_unset_simple_key() {
        let mut data = json!({
            "name": "Alice",
            "age": 30
        });

        DotArrayAccessor::unset(&mut data, "name");

        assert_eq!(data, json!({"age": 30}));
    }

    #[test]
    fn test_unset_nested_key() {
        let mut data = json!({
            "user": {
                "profile": {
                    "name": "Alice",
                    "age": 30
                }
            }
        });

        DotArrayAccessor::unset(&mut data, "user.profile.name");

        assert_eq!(data, json!({
            "user": {
                "profile": {
                    "age": 30
                }
            }
        }));
    }

    #[test]
    fn test_unset_nonexistent() {
        let mut data = json!({
            "user": {
                "name": "Alice"
            }
        });

        // 存在しないキーの削除は何もしない
        DotArrayAccessor::unset(&mut data, "user.age");
        DotArrayAccessor::unset(&mut data, "unknown.path");

        assert_eq!(data, json!({
            "user": {
                "name": "Alice"
            }
        }));
    }

    #[test]
    fn test_merge_simple_key() {
        let mut data = json!({
            "name": "Alice"
        });

        DotArrayAccessor::merge(&mut data, "age", json!(30));

        assert_eq!(data, json!({
            "name": "Alice",
            "age": 30
        }));
    }

    #[test]
    fn test_merge_overwrites_scalar() {
        let mut data = json!({
            "name": "Alice"
        });

        DotArrayAccessor::merge(&mut data, "name", json!("Bob"));

        assert_eq!(data, json!({
            "name": "Bob"
        }));
    }

    #[test]
    fn test_merge_overwrites_list() {
        let mut data = json!({
            "tags": ["php", "web"]
        });

        DotArrayAccessor::merge(&mut data, "tags", json!(["api", "rest"]));

        assert_eq!(data, json!({
            "tags": ["api", "rest"]
        }));
    }

    #[test]
    fn test_merge_nested_objects() {
        let mut data = json!({
            "user": {
                "name": "Alice",
                "profile": {
                    "age": 25
                }
            }
        });

        DotArrayAccessor::merge(&mut data, "user", json!({
            "email": "alice@example.com",
            "profile": {
                "age": 30,
                "city": "Tokyo"
            }
        }));

        assert_eq!(data, json!({
            "user": {
                "name": "Alice",
                "email": "alice@example.com",
                "profile": {
                    "age": 30,
                    "city": "Tokyo"
                }
            }
        }));
    }

    #[test]
    fn test_merge_with_dot_notation() {
        let mut data = json!({
            "connection": {
                "driver": "postgres",
                "charset": "UTF8"
            }
        });

        DotArrayAccessor::merge(&mut data, "connection", json!({
            "host": "localhost",
            "port": 5432
        }));

        assert_eq!(data, json!({
            "connection": {
                "driver": "postgres",
                "charset": "UTF8",
                "host": "localhost",
                "port": 5432
            }
        }));
    }

    #[test]
    fn test_merge_creates_path() {
        let mut data = json!({});

        DotArrayAccessor::merge(&mut data, "user.profile.name", json!("Alice"));

        assert_eq!(data, json!({
            "user": {
                "profile": {
                    "name": "Alice"
                }
            }
        }));
    }

    #[test]
    fn test_has_simple_key() {
        let data = json!({
            "name": "Alice",
            "age": 30
        });

        assert!(DotArrayAccessor::has(&data, "name"));
        assert!(DotArrayAccessor::has(&data, "age"));
        assert!(!DotArrayAccessor::has(&data, "email"));
    }

    #[test]
    fn test_has_nested_key() {
        let data = json!({
            "user": {
                "profile": {
                    "name": "Alice",
                    "age": 30
                }
            }
        });

        assert!(DotArrayAccessor::has(&data, "user"));
        assert!(DotArrayAccessor::has(&data, "user.profile"));
        assert!(DotArrayAccessor::has(&data, "user.profile.name"));
        assert!(DotArrayAccessor::has(&data, "user.profile.age"));
        assert!(!DotArrayAccessor::has(&data, "user.profile.email"));
        assert!(!DotArrayAccessor::has(&data, "user.settings"));
        assert!(!DotArrayAccessor::has(&data, "unknown"));
    }

    #[test]
    fn test_has_with_null_value() {
        let data = json!({
            "user": {
                "name": "Alice",
                "deleted_at": null
            }
        });

        // null値でもキーは存在する
        assert!(DotArrayAccessor::has(&data, "user.deleted_at"));
        assert!(!DotArrayAccessor::has(&data, "user.created_at"));
    }

    #[test]
    fn test_has_with_non_object_value() {
        let data = json!({
            "tags": ["php", "rust"],
            "count": 42
        });

        // スカラー値や配列も存在チェック可能
        assert!(DotArrayAccessor::has(&data, "tags"));
        assert!(DotArrayAccessor::has(&data, "count"));

        // 配列の要素にはドット記法でアクセスできない
        assert!(!DotArrayAccessor::has(&data, "tags.0"));
    }

    #[test]
    fn test_has_empty_key() {
        let data = json!({
            "user": {
                "name": "Alice"
            }
        });

        // 空文字列は存在しない
        assert!(!DotArrayAccessor::has(&data, ""));
    }
}
