use regex::Regex;
use serde_json::Value;

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

    fn process<F>(&mut self, value: &mut Value, resolver: &mut F)
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
