use regex::Regex;
use std::collections::HashMap;

/// PlaceholderResolver - プレースホルダー抽出・置換ユーティリティ
///
/// 依存関係を持たない純粋な文字列処理ユーティリティ。
/// 値の解決は呼び出し側の責務。
///
/// 設計方針:
/// - エスケープは不要（${} は予約語、YAML DSLとして割り切る）
/// - 再帰置換を防止（置換後の値が再度置換されない）
/// - ドット記法は不要（フラットキーで十分、ParameterBuilderの責務）
pub struct PlaceholderResolver;

impl PlaceholderResolver {
    /// テンプレート文字列からプレースホルダ名を抽出
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::common::placeholder_resolver::PlaceholderResolver;
    ///
    /// let template = "user:${sso_user_id}:${tenant_id}";
    /// let result = PlaceholderResolver::extract_placeholders(template);
    /// assert_eq!(result, vec!["sso_user_id", "tenant_id"]);
    /// ```
    pub fn extract_placeholders(template: &str) -> Vec<String> {
        let re = Regex::new(r"\$\{(\w+)\}").unwrap();
        re.captures_iter(template)
            .map(|cap| cap[1].to_string())
            .collect()
    }

    /// プレースホルダを値で置換（再帰置換を防止）
    ///
    /// 置換は一度のみ実行され、置換後の値が再度置換されることはない。
    /// 未定義のプレースホルダーはそのまま残される。
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::common::placeholder_resolver::PlaceholderResolver;
    /// use std::collections::HashMap;
    ///
    /// let template = "user:${sso_user_id}:${tenant_id}";
    /// let mut params = HashMap::new();
    /// params.insert("sso_user_id".to_string(), "user001".to_string());
    /// params.insert("tenant_id".to_string(), "1".to_string());
    ///
    /// let result = PlaceholderResolver::replace(template, &params);
    /// assert_eq!(result, "user:user001:1");
    /// ```
    ///
    /// # 再帰置換の防止
    ///
    /// ```
    /// use state_engine::common::placeholder_resolver::PlaceholderResolver;
    /// use std::collections::HashMap;
    ///
    /// let template = "${a}";
    /// let mut params = HashMap::new();
    /// params.insert("a".to_string(), "${b}".to_string());
    /// params.insert("b".to_string(), "final".to_string());
    ///
    /// let result = PlaceholderResolver::replace(template, &params);
    /// // 'final' にはならず '${b}' のまま（意図的）
    /// assert_eq!(result, "${b}");
    /// ```
    pub fn replace(template: &str, params: &HashMap<String, String>) -> String {
        // PHPの strtr() と同等の挙動を実装
        // すべてのプレースホルダーを一度のパスで置換することで、
        // 置換後の値が再度置換されることを防ぐ

        let re = Regex::new(r"\$\{(\w+)\}").unwrap();
        let mut result = String::new();
        let mut last_match = 0;

        for cap in re.captures_iter(template) {
            let m = cap.get(0).unwrap();
            let var_name = &cap[1];

            // マッチ前の部分を追加
            result.push_str(&template[last_match..m.start()]);

            // プレースホルダーを置換（paramsに存在すれば値で、なければそのまま）
            if let Some(value) = params.get(var_name) {
                result.push_str(value);
            } else {
                result.push_str(m.as_str());
            }

            last_match = m.end();
        }

        // 残りの部分を追加
        result.push_str(&template[last_match..]);

        result
    }

    /// 配列の値でプレースホルダを一括置換（再帰的）
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::common::placeholder_resolver::PlaceholderResolver;
    /// use std::collections::HashMap;
    /// use serde_yaml::Value;
    ///
    /// let mut values = HashMap::new();
    /// values.insert("key1".to_string(), Value::String("${value1}".to_string()));
    /// values.insert("key2".to_string(), Value::String("${value2}".to_string()));
    ///
    /// let mut params = HashMap::new();
    /// params.insert("value1".to_string(), "a".to_string());
    /// params.insert("value2".to_string(), "b".to_string());
    ///
    /// let result = PlaceholderResolver::replace_in_map(Value::Mapping(
    ///     values.into_iter().map(|(k, v)| (Value::String(k), v)).collect()
    /// ), &params);
    ///
    /// // result["key1"] == "a", result["key2"] == "b"
    /// ```
    pub fn replace_in_map(value: serde_yaml::Value, params: &HashMap<String, String>) -> serde_yaml::Value {
        match value {
            serde_yaml::Value::String(s) => {
                serde_yaml::Value::String(Self::replace(&s, params))
            }
            serde_yaml::Value::Mapping(map) => {
                let new_map = map
                    .into_iter()
                    .map(|(k, v)| (k, Self::replace_in_map(v, params)))
                    .collect();
                serde_yaml::Value::Mapping(new_map)
            }
            serde_yaml::Value::Sequence(seq) => {
                let new_seq = seq
                    .into_iter()
                    .map(|v| Self::replace_in_map(v, params))
                    .collect();
                serde_yaml::Value::Sequence(new_seq)
            }
            // その他の型（Number, Bool, Null）はそのまま
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_placeholders() {
        let template = "user:${sso_user_id}:${tenant_id}";
        let result = PlaceholderResolver::extract_placeholders(template);
        assert_eq!(result, vec!["sso_user_id", "tenant_id"]);
    }

    #[test]
    fn test_extract_placeholders_empty() {
        let template = "user:123:456";
        let result = PlaceholderResolver::extract_placeholders(template);
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_replace() {
        let template = "user:${sso_user_id}:${tenant_id}";
        let mut params = HashMap::new();
        params.insert("sso_user_id".to_string(), "user001".to_string());
        params.insert("tenant_id".to_string(), "1".to_string());

        let result = PlaceholderResolver::replace(template, &params);
        assert_eq!(result, "user:user001:1");
    }

    #[test]
    fn test_replace_prevent_recursion() {
        // 再帰置換を防止（置換後の値が再度置換されない）
        let template = "${a}";
        let mut params = HashMap::new();
        params.insert("a".to_string(), "${b}".to_string());
        params.insert("b".to_string(), "final".to_string());

        let result = PlaceholderResolver::replace(template, &params);
        // 'final' にはならず '${b}' のまま（意図的）
        assert_eq!(result, "${b}");
    }

    #[test]
    fn test_replace_partial_match() {
        let template = "value: ${key}, other: ${other_key}";
        let mut params = HashMap::new();
        params.insert("key".to_string(), "replaced".to_string());

        let result = PlaceholderResolver::replace(template, &params);
        // 未定義のプレースホルダーはそのまま
        assert_eq!(result, "value: replaced, other: ${other_key}");
    }

    #[test]
    fn test_replace_literal_dollar() {
        // $ 単体は問題ない（${} でなければ置換されない）
        let template = "価格は$100です";
        let mut params = HashMap::new();
        params.insert("price".to_string(), "200".to_string());

        let result = PlaceholderResolver::replace(template, &params);
        assert_eq!(result, "価格は$100です");
    }

    #[test]
    fn test_replace_in_map_simple() {
        use serde_yaml::{Mapping, Value};

        let mut map = Mapping::new();
        map.insert(
            Value::String("key1".to_string()),
            Value::String("${value1}".to_string()),
        );
        map.insert(
            Value::String("key2".to_string()),
            Value::String("${value2}".to_string()),
        );

        let mut params = HashMap::new();
        params.insert("value1".to_string(), "a".to_string());
        params.insert("value2".to_string(), "b".to_string());

        let result = PlaceholderResolver::replace_in_map(Value::Mapping(map), &params);

        if let Value::Mapping(result_map) = result {
            assert_eq!(
                result_map.get(&Value::String("key1".to_string())),
                Some(&Value::String("a".to_string()))
            );
            assert_eq!(
                result_map.get(&Value::String("key2".to_string())),
                Some(&Value::String("b".to_string()))
            );
        } else {
            panic!("Expected Mapping");
        }
    }

    #[test]
    fn test_replace_in_map_nested() {
        use serde_yaml::{Mapping, Value};

        let mut inner = Mapping::new();
        inner.insert(
            Value::String("key3".to_string()),
            Value::String("${value3}".to_string()),
        );
        inner.insert(
            Value::String("literal".to_string()),
            Value::String("no placeholder".to_string()),
        );

        let mut outer = Mapping::new();
        outer.insert(
            Value::String("key1".to_string()),
            Value::String("${value1}".to_string()),
        );
        outer.insert(Value::String("nested".to_string()), Value::Mapping(inner));

        let mut params = HashMap::new();
        params.insert("value1".to_string(), "a".to_string());
        params.insert("value3".to_string(), "c".to_string());

        let result = PlaceholderResolver::replace_in_map(Value::Mapping(outer), &params);

        if let Value::Mapping(result_map) = result {
            assert_eq!(
                result_map.get(&Value::String("key1".to_string())),
                Some(&Value::String("a".to_string()))
            );

            if let Some(Value::Mapping(nested_map)) =
                result_map.get(&Value::String("nested".to_string()))
            {
                assert_eq!(
                    nested_map.get(&Value::String("key3".to_string())),
                    Some(&Value::String("c".to_string()))
                );
                assert_eq!(
                    nested_map.get(&Value::String("literal".to_string())),
                    Some(&Value::String("no placeholder".to_string()))
                );
            } else {
                panic!("Expected nested Mapping");
            }
        } else {
            panic!("Expected Mapping");
        }
    }

    #[test]
    fn test_replace_in_map_preserves_types() {
        use serde_yaml::{Mapping, Value};

        let mut map = Mapping::new();
        map.insert(
            Value::String("string".to_string()),
            Value::String("${key}".to_string()),
        );
        map.insert(Value::String("int".to_string()), Value::Number(123.into()));
        map.insert(Value::String("bool".to_string()), Value::Bool(true));
        map.insert(Value::String("null".to_string()), Value::Null);

        let mut params = HashMap::new();
        params.insert("key".to_string(), "replaced".to_string());

        let result = PlaceholderResolver::replace_in_map(Value::Mapping(map), &params);

        if let Value::Mapping(result_map) = result {
            assert_eq!(
                result_map.get(&Value::String("string".to_string())),
                Some(&Value::String("replaced".to_string()))
            );
            assert_eq!(
                result_map.get(&Value::String("int".to_string())),
                Some(&Value::Number(123.into()))
            );
            assert_eq!(
                result_map.get(&Value::String("bool".to_string())),
                Some(&Value::Bool(true))
            );
            assert_eq!(
                result_map.get(&Value::String("null".to_string())),
                Some(&Value::Null)
            );
        } else {
            panic!("Expected Mapping");
        }
    }
}
