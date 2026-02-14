// DotMapAccessor
// ドット記法での Map (Object) アクセスを提供
//
// - missingKeys追跡機能
// - 階層構造の自動作成（set時）
// - インスタンスメソッド（get, getMissingKeys, clearMissingKeys）
// - 静的メソッド（set, merge, unset）

use serde_json::Value;
use crate::common::DotString;

/// ドット記法でのデータアクセスを提供
pub struct DotMapAccessor {
    missing_keys: Vec<String>,
}

impl DotMapAccessor {

    pub fn new() -> Self {
        Self {
            missing_keys: Vec::new(),
        }
    }

    /// ドット記法で値を取得（missingKeys追跡付き）
    ///
    /// キーが見つからない場合はmissingKeysに記録してNoneを返す
    pub fn get<'a>(&mut self, data: &'a Value, dot_string: &DotString) -> Option<&'a Value> {
        let key = dot_string.as_str();

        // 単純なキーアクセス
        if dot_string.len() <= 1 {
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

        // ネストされたパスを辿る
        let mut current = data;
        for segment in dot_string.iter() {
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
    pub fn set(data: &mut Value, dot_string: &DotString, value: Value) {
        let key = dot_string.as_str();

        // 単純なキー設定
        if dot_string.len() <= 1 {
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

        // dataがObjectでない場合、新しいObjectを作成
        if !data.is_object() {
            *data = Value::Object(serde_json::Map::new());
        }

        let mut current = data;
        let last_idx = dot_string.len() - 1;

        for (i, segment) in dot_string.iter().enumerate() {
            if i == last_idx {
                // 最後のセグメント：値を設定
                if let Some(obj) = current.as_object_mut() {
                    obj.insert(segment.to_string(), value);
                }
                return;
            }

            // 中間パス：存在しないか、Objectでない場合は新規作成してから移動
            {
                let obj = current.as_object_mut().expect("current must be an object");
                if !obj.contains_key(segment) || !obj[segment].is_object() {
                    obj.insert(segment.to_string(), Value::Object(serde_json::Map::new()));
                }
            }

            // 次の階層へ移動
            current = current.get_mut(segment).expect("segment must exist");
        }
    }

    /// 値をマージ（静的メソッド）
    ///
    /// 例: merge(&mut data, "user.profile", json!({"age": 30}))
    ///
    /// 注意: マージ処理の各レベルで、既存値と新しい値の少なくとも一方がスカラー（非オブジェクト）である場合、上書き処理がされる。
    /// state object では、末尾のノードは値に null を持って、末尾の値と区別されている。
    /// このため、scalar と list の object は、自動的に上書き処理してよい。
    pub fn merge(data: &mut Value, dot_string: &DotString, value: Value) {
        let key = dot_string.as_str();

        // 単純なキー
        if dot_string.len() <= 1 {
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
                                    let k_dot = DotString::new(k);
                                    Self::merge(existing, &k_dot, v.clone());
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

        // ネストされたパス
        let first_segment = &dot_string[0];
        let remaining_key = dot_string[1..].iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(".");

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
                let remaining_dot = DotString::new(&remaining_key);
                Self::merge(next, &remaining_dot, value);
            }
        }
    }

    /// キーが存在するかチェック（静的メソッド）
    ///
    /// 例: has(&data, "user.profile.name")
    ///
    /// ドット記法でのパスを辿り、最後のキーまで存在するか確認する
    pub fn has(data: &Value, dot_string: &DotString) -> bool {
        let key = dot_string.as_str();

        // 単純なキー存在チェック
        if dot_string.len() <= 1 {
            if let Some(obj) = data.as_object() {
                return obj.contains_key(key);
            }
            return false;
        }

        // ネストされたパスを辿る
        let mut current = data;
        for segment in dot_string.iter() {
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
    pub fn unset(data: &mut Value, dot_string: &DotString) {
        let key = dot_string.as_str();

        // 単純な削除
        if dot_string.len() <= 1 {
            if let Some(obj) = data.as_object_mut() {
                obj.remove(key);
            }
            return;
        }

        // ネストされたパスを辿る
        let mut current = data;
        let last_idx = dot_string.len() - 1;

        for (i, segment) in dot_string.iter().enumerate() {
            if i == last_idx {
                // 最後のセグメント：削除
                if let Some(obj) = current.as_object_mut() {
                    obj.remove(segment);
                }
                return;
            }

            // 中間パス：次の階層へ移動
            if !current.is_object() {
                return;
            }

            let has_next = if let Some(obj) = current.as_object() {
                obj.contains_key(segment) && obj.get(segment).map_or(false, |v| v.is_object())
            } else {
                false
            };

            if !has_next {
                // パスが存在しない場合は何もしない
                return;
            }

            // 次の階層へ移動
            current = current.get_mut(segment).unwrap();
        }
    }
}

impl Default for DotMapAccessor {
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
        let mut accessor = DotMapAccessor::new();
        let data = json!({
            "name": "Alice"
        });

        let key = DotString::new("name");
        let result = accessor.get(&data, &key);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), &json!("Alice"));
        assert_eq!(accessor.get_missing_keys().len(), 0);
    }

    #[test]
    fn test_get_nested_key() {
        let mut accessor = DotMapAccessor::new();
        let data = json!({
            "user": {
                "profile": {
                    "name": "Alice"
                }
            }
        });

        let key = DotString::new("user.profile.name");
        let result = accessor.get(&data, &key);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), &json!("Alice"));
        assert_eq!(accessor.get_missing_keys().len(), 0);
    }

    #[test]
    fn test_get_missing_key_tracking() {
        let mut accessor = DotMapAccessor::new();
        let data = json!({
            "user": {
                "name": "Alice"
            }
        });

        let key = DotString::new("user.age");
        let result = accessor.get(&data, &key);
        assert!(result.is_none());
        assert_eq!(accessor.get_missing_keys(), vec!["user.age"]);

        // 2回目の失敗
        let key2 = DotString::new("user.email");
        let result2 = accessor.get(&data, &key2);
        assert!(result2.is_none());
        assert_eq!(accessor.get_missing_keys(), vec!["user.age", "user.email"]);

        // クリア
        accessor.clear_missing_keys();
        assert_eq!(accessor.get_missing_keys().len(), 0);
    }

    #[test]
    fn test_set_simple_key() {
        let mut data = json!({});
        let key = DotString::new("name");
        DotMapAccessor::set(&mut data, &key, json!("Alice"));

        assert_eq!(data, json!({"name": "Alice"}));
    }

    #[test]
    fn test_set_nested_key() {
        let mut data = json!({});
        let key = DotString::new("user.profile.name");
        DotMapAccessor::set(&mut data, &key, json!("Alice"));

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

        let key = DotString::new("user.name");
        DotMapAccessor::set(&mut data, &key, json!("Bob"));

        assert_eq!(data["user"]["name"], json!("Bob"));
    }

    #[test]
    fn test_unset_simple_key() {
        let mut data = json!({
            "name": "Alice",
            "age": 30
        });

        let key = DotString::new("name");
        DotMapAccessor::unset(&mut data, &key);

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

        let key = DotString::new("user.profile.name");
        DotMapAccessor::unset(&mut data, &key);

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
        let key1 = DotString::new("user.age");
        DotMapAccessor::unset(&mut data, &key1);
        let key2 = DotString::new("unknown.path");
        DotMapAccessor::unset(&mut data, &key2);

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

        let key = DotString::new("age");
        DotMapAccessor::merge(&mut data, &key, json!(30));

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

        let key = DotString::new("name");
        DotMapAccessor::merge(&mut data, &key, json!("Bob"));

        assert_eq!(data, json!({
            "name": "Bob"
        }));
    }

    #[test]
    fn test_merge_overwrites_list() {
        let mut data = json!({
            "tags": ["php", "web"]
        });

        let key = DotString::new("tags");
        DotMapAccessor::merge(&mut data, &key, json!(["api", "rest"]));

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

        let key = DotString::new("user");
        DotMapAccessor::merge(&mut data, &key, json!({
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

        let key = DotString::new("connection");
        DotMapAccessor::merge(&mut data, &key, json!({
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

        let key = DotString::new("user.profile.name");
        DotMapAccessor::merge(&mut data, &key, json!("Alice"));

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

        let key1 = DotString::new("name");
        assert!(DotMapAccessor::has(&data, &key1));
        let key2 = DotString::new("age");
        assert!(DotMapAccessor::has(&data, &key2));
        let key3 = DotString::new("email");
        assert!(!DotMapAccessor::has(&data, &key3));
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

        assert!(DotMapAccessor::has(&data, &DotString::new("user")));
        assert!(DotMapAccessor::has(&data, &DotString::new("user.profile")));
        assert!(DotMapAccessor::has(&data, &DotString::new("user.profile.name")));
        assert!(DotMapAccessor::has(&data, &DotString::new("user.profile.age")));
        assert!(!DotMapAccessor::has(&data, &DotString::new("user.profile.email")));
        assert!(!DotMapAccessor::has(&data, &DotString::new("user.settings")));
        assert!(!DotMapAccessor::has(&data, &DotString::new("unknown")));
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
        assert!(DotMapAccessor::has(&data, &DotString::new("user.deleted_at")));
        assert!(!DotMapAccessor::has(&data, &DotString::new("user.created_at")));
    }

    #[test]
    fn test_has_with_non_object_value() {
        let data = json!({
            "tags": ["php", "rust"],
            "count": 42
        });

        // スカラー値や配列も存在チェック可能
        assert!(DotMapAccessor::has(&data, &DotString::new("tags")));
        assert!(DotMapAccessor::has(&data, &DotString::new("count")));

        // 配列の要素にはドット記法でアクセスできない
        assert!(!DotMapAccessor::has(&data, &DotString::new("tags.0")));
    }

    #[test]
    fn test_has_empty_key() {
        let data = json!({
            "user": {
                "name": "Alice"
            }
        });

        // 空文字列は存在しない
        assert!(!DotMapAccessor::has(&data, &DotString::new("")));
    }
}
