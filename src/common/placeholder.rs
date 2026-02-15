use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;

/// Placeholder resolver for ${...} patterns in JSON values
///
/// dev note:
/// - エスケープは不要（${} は予約語、YAML DSLとして割り切る）
/// - 再帰置換を防止（置換後の値が再度置換されない）
/// - ドット解釈は不要
pub struct Placeholder {
    missing_keys: Vec<String>,
}

impl Placeholder {
    pub fn new() -> Self {
        Self {
            missing_keys: Vec::new(),
        }
    }

    pub fn get_missing_keys(&self) -> &[String] {
        &self.missing_keys
    }

    pub fn clear_missing_keys(&mut self) {
        self.missing_keys.clear();
    }

    /// Collect all placeholder names from a value (unique list)
    ///
    /// Walks through the value and extracts all ${key} patterns.
    /// Returns unique placeholder names in the order they appear.
    ///
    /// # Examples
    /// ```
    /// use state_engine::common::Placeholder;
    /// use serde_json::json;
    ///
    /// let value = json!({
    ///     "key1": "user:${session.id}",
    ///     "key2": "tenant:${cache.user.org_id}",
    ///     "key3": "${session.id}"  // duplicate
    /// });
    ///
    /// let names = Placeholder::collect(&value);
    /// assert_eq!(names, vec!["session.id", "cache.user.org_id"]);
    /// ```
    pub fn collect(value: &Value) -> Vec<String> {
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

        walk(value, &mut names, &mut seen, &re);
        names
    }

    /// Map placeholders (${...}) to actual values using resolver callback
    ///
    /// Walks through all nodes and values, replaces ${key} patterns with resolved values.
    /// Records missing keys when resolver returns None.
    ///
    /// Type preservation:
    /// - Single placeholder ("${key}") → preserves original type
    /// - Multiple or embedded ("user:${id}") → string replacement
    pub fn map<F>(&mut self, mut value: Value, resolver: &mut F) -> Value
    where
        F: FnMut(&str) -> Option<Value>,
    {
        self.process(&mut value, resolver);
        value
    }

    pub fn process<F>(&mut self, value: &mut Value, resolver: &mut F)
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

                // 単一プレースホルダー（値全体が ${key} のみ）の場合は型を保持
                if placeholders.len() == 1 && *s == placeholders[0].0 {
                    let key = &placeholders[0].1;
                    if let Some(resolved) = resolver(key) {
                        *value = resolved;
                    } else {
                        self.missing_keys.push(key.clone());
                    }
                    return;
                }

                // 複数または文字列内プレースホルダーの場合は文字列置換
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
                    } else {
                        self.missing_keys.push(key.clone());
                    }
                }
                *s = result;
            }
            Value::Object(map) => {
                for (_, v) in map.iter_mut() {
                    self.process(v, resolver);
                }
            }
            Value::Array(arr) => {
                for v in arr.iter_mut() {
                    self.process(v, resolver);
                }
            }
            _ => {}
        }
    }
}

impl Default for Placeholder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_collect_simple() {
        let value = json!({
            "key": "user:${session.id}"
        });
        let names = Placeholder::collect(&value);
        assert_eq!(names, vec!["session.id"]);
    }

    #[test]
    fn test_collect_multiple() {
        let value = json!({
            "key1": "${session.id}",
            "key2": "${cache.user.org_id}",
            "key3": "prefix:${connection.host}:suffix"
        });
        let names = Placeholder::collect(&value);
        assert_eq!(names, vec!["session.id", "cache.user.org_id", "connection.host"]);
    }

    #[test]
    fn test_collect_duplicates() {
        let value = json!({
            "key1": "${session.id}",
            "key2": "${cache.user.org_id}",
            "key3": "${session.id}"  // duplicate
        });
        let names = Placeholder::collect(&value);
        assert_eq!(names, vec!["session.id", "cache.user.org_id"]);
    }

    #[test]
    fn test_collect_nested() {
        let value = json!({
            "level1": {
                "level2": {
                    "key": "${nested.value}"
                }
            },
            "array": ["${array.item1}", "${array.item2}"]
        });
        let names = Placeholder::collect(&value);
        // Order depends on Object iteration, just check all are present
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"nested.value".to_string()));
        assert!(names.contains(&"array.item1".to_string()));
        assert!(names.contains(&"array.item2".to_string()));
    }

    #[test]
    fn test_collect_no_placeholders() {
        let value = json!({
            "key": "plain text"
        });
        let names = Placeholder::collect(&value);
        assert!(names.is_empty());
    }
}
