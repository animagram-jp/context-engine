use serde_json::Value;
use crate::common::DotString;

/// # Examples
///
/// ```
/// use serde_json::json;
/// use state_engine::common::DotString;
/// use state_engine::common::DotMapAccessor;
///
/// let mut accessor = DotMapAccessor::new();
/// let map = json!({
///     "1-1-key": {
///         "2-1-key": 1,
///         "2-2-key": 0.1,
///         "2-3-key": "string",
///         "2-4-key": [24, 2.4, "二十四", [31, 3.1, "三十一"], {"3-2-key": 32}]
///     }
/// });
///
/// let key = DotString::new("1-1-key.2-4-key");
/// let result = accessor.get(&map, &key);
/// assert_eq!(result, Some(&json!([24, 2.4, "二十四", [31, 3.1, "三十一"], {"3-2-key": 32}])));
///
/// let missing = DotString::new("never-found-key");
/// assert_eq!(accessor.get(&map, &missing), None);
/// assert_eq!(accessor.get_missing_keys(), &["never-found-key"]);
///
/// accessor.clear_missing_keys();
/// assert_eq!(accessor.get_missing_keys().len(), 0);
/// ```
pub struct DotMapAccessor {
    missing_keys: Vec<String>,
}

impl DotMapAccessor {

    pub fn new() -> Self {
        Self {
            missing_keys: Vec::new(),
        }
    }

    /// record miss key into self.missing_keys
    pub fn get<'a>(&mut self, data: &'a Value, dot_string: &DotString) -> Option<&'a Value> {
        let key = dot_string.as_str();

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

    /// show self.missing_keys
    pub fn get_missing_keys(&self) -> &[String] {
        &self.missing_keys
    }

    /// clear self.missing_keys
    pub fn clear_missing_keys(&mut self) {
        self.missing_keys.clear();
    }

    /// set Value
    /// static method
    pub fn set(data: &mut Value, dot_string: &DotString, value: Value) {
        let key = dot_string.as_str();

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

    /// insert or overwrite Value
    /// static method
    ///
    /// dev note:
    /// マージ処理の各レベルで、既存値と新しい値の少なくとも一方がスカラー（非オブジェクト）である場合、上書き処理がされる。
    /// state object では、末尾のノードは値に null を持って、末尾の値と区別されている。
    /// このため、scalar と list の object は、自動的に上書き処理してよい。
    pub fn merge(data: &mut Value, dot_string: &DotString, value: Value) {
        let key = dot_string.as_str();

        if dot_string.len() <= 1 {
            if let Some(obj) = data.as_object_mut() {
                let should_merge = if let Some(existing) = obj.get(key) {
                    existing.is_object() && value.is_object()
                } else {
                    false
                };

                if should_merge {
                    if let Some(value_obj) = value.as_object() {
                        for (k, v) in value_obj {
                            if let Some(existing) = obj.get_mut(key) {
                                let should_recurse = if let Some(existing_obj) = existing.as_object() {
                                    existing_obj.contains_key(k) && existing_obj[k].is_object() && v.is_object()
                                } else {
                                    false
                                };

                                if should_recurse {
                                    let k_dot = DotString::new(k);
                                    Self::merge(existing, &k_dot, v.clone());
                                } else {
                                    if let Some(existing_obj_mut) = existing.as_object_mut() {
                                        existing_obj_mut.insert(k.clone(), v.clone());
                                    }
                                }
                            }
                        }
                    }
                    return;
                }
                obj.insert(key.to_string(), value);
            } else {
                let mut new_obj = serde_json::Map::new();
                new_obj.insert(key.to_string(), value);
                *data = Value::Object(new_obj);
            }
            return;
        }

        let first_segment = &dot_string[0];
        let remaining_key = dot_string[1..].iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(".");

        if !data.is_object() {
            *data = Value::Object(serde_json::Map::new());
        }

        if let Some(obj) = data.as_object_mut() {
            if !obj.contains_key(first_segment) {
                obj.insert(first_segment.to_string(), Value::Object(serde_json::Map::new()));
            }

            if let Some(next) = obj.get_mut(first_segment) {
                let remaining_dot = DotString::new(&remaining_key);
                Self::merge(next, &remaining_dot, value);
            }
        }
    }

    /// check hit/miss key
    /// static method
    pub fn has(data: &Value, dot_string: &DotString) -> bool {
        let key = dot_string.as_str();

        if dot_string.len() <= 1 {
            if let Some(obj) = data.as_object() {
                return obj.contains_key(key);
            }
            return false;
        }
        let mut current = data;
        for segment in dot_string.iter() {
            match current.get(segment) {
                Some(next) => current = next,
                None => return false,
            }
        }

        true
    }

    /// unset {Key: Value}
    /// static method
    /// no reaction when miss hit
    pub fn unset(data: &mut Value, dot_string: &DotString) {
        let key = dot_string.as_str();

        if dot_string.len() <= 1 {
            if let Some(obj) = data.as_object_mut() {
                obj.remove(key);
            }
            return;
        }

        let mut current = data;
        let last_idx = dot_string.len() - 1;

        for (i, segment) in dot_string.iter().enumerate() {
            if i == last_idx {
                if let Some(obj) = current.as_object_mut() {
                    obj.remove(segment);
                }
                return;
            }

            if !current.is_object() {
                return;
            }

            let has_next = if let Some(obj) = current.as_object() {
                obj.contains_key(segment) && obj.get(segment).map_or(false, |v| v.is_object())
            } else {
                false
            };

            if !has_next {
                return;
            }

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
    fn test_set_overwrites_existing() {
        let mut map = json!({
            "1-1-key": {"2-1-key": "old-value"}
        });

        let key = DotString::new("1-1-key.2-1-key");
        DotMapAccessor::set(&mut map, &key, json!("new-value"));

        assert_eq!(map["1-1-key"]["2-1-key"], json!("new-value"));
    }

    #[test]
    fn test_unset_nonexistent() {
        let mut map = json!({
            "1-1-key": {"2-1-key": "value"}
        });

        // 存在しないキーの削除は何もしない
        let key1 = DotString::new("1-1-key.not-found");
        DotMapAccessor::unset(&mut map, &key1);
        let key2 = DotString::new("unknown.path");
        DotMapAccessor::unset(&mut map, &key2);

        assert_eq!(map, json!({
            "1-1-key": {"2-1-key": "value"}
        }));
    }

    #[test]
    fn test_merge_overwrites_scalar() {
        let mut map = json!({"1-1-key": "old-value"});

        let key = DotString::new("1-1-key");
        DotMapAccessor::merge(&mut map, &key, json!("new-value"));

        assert_eq!(map, json!({"1-1-key": "new-value"}));
    }

    #[test]
    fn test_merge_overwrites_list() {
        let mut map = json!({"1-1-key": [1, 2, 3]});

        let key = DotString::new("1-1-key");
        DotMapAccessor::merge(&mut map, &key, json!([4, 5, 6]));

        assert_eq!(map, json!({"1-1-key": [4, 5, 6]}));
    }

    #[test]
    fn test_merge_nested_objects() {
        let mut map = json!({
            "1-1-key": {
                "2-1-key": "value1",
                "2-2-key": {"3-1-key": 100}
            }
        });

        let key = DotString::new("1-1-key");
        DotMapAccessor::merge(&mut map, &key, json!({
            "2-3-key": "value2",
            "2-2-key": {"3-1-key": 200, "3-2-key": 300}
        }));

        assert_eq!(map, json!({
            "1-1-key": {
                "2-1-key": "value1",
                "2-3-key": "value2",
                "2-2-key": {"3-1-key": 200, "3-2-key": 300}
            }
        }));
    }

    #[test]
    fn test_has_with_null_value() {
        let map = json!({
            "1-1-key": {
                "2-1-key": "value",
                "2-2-key": null
            }
        });

        assert!(DotMapAccessor::has(&map, &DotString::new("1-1-key.2-2-key")));
        assert!(!DotMapAccessor::has(&map, &DotString::new("1-1-key.2-3-key")));
    }

    #[test]
    fn test_has_with_non_object_value() {
        let map = json!({
            "1-1-key": [1, 2, 3],
            "1-2-key": 42
        });

        assert!(DotMapAccessor::has(&map, &DotString::new("1-1-key")));
        assert!(DotMapAccessor::has(&map, &DotString::new("1-2-key")));
        assert!(!DotMapAccessor::has(&map, &DotString::new("1-1-key.0")));
    }

    #[test]
    fn test_has_empty_key() {
        let map = json!({"1-1-key": {"2-1-key": "value"}});

        assert!(!DotMapAccessor::has(&map, &DotString::new("")));
    }
}
