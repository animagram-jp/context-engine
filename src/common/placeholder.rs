use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Pure logic collection for ${...} placeholder resolution
///
/// This struct provides stateless utility functions for placeholder operations.
/// State management (resolved values, pending paths) should be handled by the caller (State).
///
/// dev note:
/// - エスケープは不要（${} は予約語、YAML DSLとして割り切る）
/// - 再帰置換を防止（置換後の値が再度置換されない）
/// - ドット解釈は不要
pub struct Placeholder;

impl Placeholder {
    /// Collect all unique placeholder names from a HashMap
    ///
    /// Walks through HashMap values and extracts all ${key} patterns.
    /// Returns unique placeholder names (order depends on HashMap iteration).
    ///
    /// # Examples
    /// ```
    /// use state_engine::common::Placeholder;
    /// use serde_json::json;
    /// use std::collections::HashMap;
    ///
    /// let mut map = HashMap::new();
    /// map.insert("key1".to_string(), json!("user:${session.id}"));
    /// map.insert("key2".to_string(), json!("tenant:${cache.user.org_id}"));
    /// map.insert("key3".to_string(), json!("${session.id}"));  // duplicate
    ///
    /// let names = Placeholder::collect(&map);
    /// assert_eq!(names.len(), 2);
    /// assert!(names.contains(&"session.id".to_string()));
    /// assert!(names.contains(&"cache.user.org_id".to_string()));
    /// ```
    pub fn collect(map: &HashMap<String, Value>) -> Vec<String> {
        let mut names = Vec::new();
        let mut seen = HashSet::new();
        let re = Regex::new(r"\$\{([\w.]+)\}").unwrap();

        fn walk(value: &Value, names: &mut Vec<String>, seen: &mut HashSet<String>, re: &Regex) {
            match value {
                Value::String(s) => {
                    for cap in re.captures_iter(s) {
                        let name = cap[1].to_string();
                        if seen.insert(name.clone()) {
                            names.push(name);
                        }
                    }
                }
                Value::Object(map) => {
                    for v in map.values() {
                        walk(v, names, seen, re);
                    }
                }
                Value::Array(arr) => {
                    for v in arr {
                        walk(v, names, seen, re);
                    }
                }
                _ => {}
            }
        }

        for v in map.values() {
            walk(v, &mut names, &mut seen, &re);
        }
        names
    }

    /// Replace placeholders in a HashMap using a resolved values map
    ///
    /// Walks through all values and replaces ${key} patterns with resolved values.
    /// Returns list of placeholder names that were not found in resolved_values.
    ///
    /// Type preservation:
    /// - Single placeholder ("${key}") → preserves original type
    /// - Multiple or embedded ("user:${id}") → string replacement
    ///
    /// # Examples
    /// ```
    /// use state_engine::common::Placeholder;
    /// use serde_json::json;
    /// use std::collections::HashMap;
    ///
    /// let mut config = HashMap::new();
    /// config.insert("key".to_string(), json!("user:${session.id}"));
    ///
    /// let mut resolved = HashMap::new();
    /// resolved.insert("session.id".to_string(), json!(123));
    ///
    /// let missing = Placeholder::replace(&mut config, &resolved);
    /// assert!(missing.is_empty());
    /// assert_eq!(config.get("key").unwrap(), &json!("user:123"));
    /// ```
    pub fn replace(
        config: &mut HashMap<String, Value>,
        resolved_values: &HashMap<String, Value>,
    ) -> Vec<String> {
        let mut missing = Vec::new();
        let mut seen = HashSet::new();

        let mut resolver = |name: &str| -> Option<Value> {
            let result = resolved_values.get(name).cloned();
            if result.is_none() && seen.insert(name.to_string()) {
                missing.push(name.to_string());
            }
            result
        };

        for v in config.values_mut() {
            Self::process_value(v, &mut resolver);
        }

        missing
    }

    /// Internal recursive function to process a single Value
    fn process_value<F>(value: &mut Value, resolver: &mut F)
    where
        F: FnMut(&str) -> Option<Value>,
    {
        match value {
            Value::String(s) => {
                let re = Regex::new(r"\$\{([\w.]+)\}").unwrap();
                let placeholders: Vec<(String, String)> = re
                    .captures_iter(s)
                    .map(|cap| (cap[0].to_string(), cap[1].to_string()))
                    .collect();

                if placeholders.is_empty() {
                    return;
                }

                // Single placeholder ("${key}") → preserve type
                if placeholders.len() == 1 && *s == placeholders[0].0 {
                    if let Some(resolved) = resolver(&placeholders[0].1) {
                        *value = resolved;
                    }
                    return;
                }

                // Multiple or embedded placeholders → string replacement
                let mut result = s.clone();
                for (pattern, key) in placeholders {
                    if let Some(resolved) = resolver(&key) {
                        let replacement = match resolved {
                            Value::String(s) => s,
                            Value::Number(n) => n.to_string(),
                            Value::Bool(b) => b.to_string(),
                            _ => continue,
                        };
                        result = result.replace(&pattern, &replacement);
                    }
                }
                *s = result;
            }
            Value::Object(map) => {
                for v in map.values_mut() {
                    Self::process_value(v, resolver);
                }
            }
            Value::Array(arr) => {
                for v in arr.iter_mut() {
                    Self::process_value(v, resolver);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_collect_simple() {
        let mut map = HashMap::new();
        map.insert("key".to_string(), json!("user:${session.id}"));
        let names = Placeholder::collect(&map);
        assert_eq!(names, vec!["session.id"]);
    }

    #[test]
    fn test_collect_multiple() {
        let mut map = HashMap::new();
        map.insert("key1".to_string(), json!("${session.id}"));
        map.insert("key2".to_string(), json!("${cache.user.org_id}"));
        map.insert("key3".to_string(), json!("prefix:${connection.host}:suffix"));
        let names = Placeholder::collect(&map);
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"session.id".to_string()));
        assert!(names.contains(&"cache.user.org_id".to_string()));
        assert!(names.contains(&"connection.host".to_string()));
    }

    #[test]
    fn test_collect_duplicates() {
        let mut map = HashMap::new();
        map.insert("key1".to_string(), json!("${session.id}"));
        map.insert("key2".to_string(), json!("${cache.user.org_id}"));
        map.insert("key3".to_string(), json!("${session.id}"));
        let names = Placeholder::collect(&map);
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"session.id".to_string()));
        assert!(names.contains(&"cache.user.org_id".to_string()));
    }

    #[test]
    fn test_collect_nested() {
        let mut map = HashMap::new();
        map.insert(
            "level1".to_string(),
            json!({
                "level2": {
                    "key": "${nested.value}"
                }
            }),
        );
        map.insert(
            "array".to_string(),
            json!(["${array.item1}", "${array.item2}"]),
        );
        let names = Placeholder::collect(&map);
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"nested.value".to_string()));
        assert!(names.contains(&"array.item1".to_string()));
        assert!(names.contains(&"array.item2".to_string()));
    }

    #[test]
    fn test_collect_no_placeholders() {
        let mut map = HashMap::new();
        map.insert("key".to_string(), json!("plain text"));
        let names = Placeholder::collect(&map);
        assert!(names.is_empty());
    }

    #[test]
    fn test_replace_simple() {
        let mut config = HashMap::new();
        config.insert("key".to_string(), json!("user:${session.id}"));

        let mut resolved = HashMap::new();
        resolved.insert("session.id".to_string(), json!(123));

        let missing = Placeholder::replace(&mut config, &resolved);
        assert!(missing.is_empty());
        assert_eq!(config.get("key").unwrap(), &json!("user:123"));
    }

    #[test]
    fn test_replace_type_preservation() {
        let mut config = HashMap::new();
        config.insert("key".to_string(), json!("${session.id}"));

        let mut resolved = HashMap::new();
        resolved.insert("session.id".to_string(), json!(123));

        let missing = Placeholder::replace(&mut config, &resolved);
        assert!(missing.is_empty());
        assert_eq!(config.get("key").unwrap(), &json!(123)); // Number preserved
    }

    #[test]
    fn test_replace_missing_placeholder() {
        let mut config = HashMap::new();
        config.insert("key".to_string(), json!("user:${session.id}"));

        let resolved = HashMap::new(); // Empty

        let missing = Placeholder::replace(&mut config, &resolved);
        assert_eq!(missing, vec!["session.id"]);
        assert_eq!(config.get("key").unwrap(), &json!("user:${session.id}")); // Unchanged
    }

    #[test]
    fn test_replace_multiple_same_placeholder() {
        let mut config = HashMap::new();
        config.insert("key1".to_string(), json!("${session.id}"));
        config.insert("key2".to_string(), json!("user:${session.id}"));

        let resolved = HashMap::new();

        let missing = Placeholder::replace(&mut config, &resolved);
        // Should only report "session.id" once
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], "session.id");
    }
}
