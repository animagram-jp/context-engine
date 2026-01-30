// DotArrayAccessor
// ドット記法での配列アクセスを提供

use std::collections::HashMap;
use serde_json::Value;

pub struct DotArrayAccessor;

impl DotArrayAccessor {
    /// ドット記法で値を取得
    /// 例: get(data, "user.profile.name")
    pub fn get<'a>(data: &'a HashMap<String, Value>, path: &str) -> Option<&'a Value> {
        let keys: Vec<&str> = path.split('.').collect();
        if keys.is_empty() {
            return None;
        }

        let mut current = data.get(keys[0])?;

        for key in &keys[1..] {
            current = current.get(key)?;
        }

        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_dot_array_accessor() {
        let mut data = HashMap::new();
        data.insert("user".to_string(), json!({
            "profile": {
                "name": "Test User"
            }
        }));

        let result = DotArrayAccessor::get(&data, "user.profile.name");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), &json!("Test User"));
    }
}
