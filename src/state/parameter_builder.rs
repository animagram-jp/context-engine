// ParameterBuilder - namespace-based placeholder resolution
//
// プレースホルダーを namespace ルールに従って解決する。

use serde_yaml::Value;

/// ParameterBuilder
///
/// namespace ルールに従ってプレースホルダーを解決する。
///
/// # 解決順序
/// 1. 同層参照: ${id} → {context_key}.id
/// 2. 親層参照: ${user.id} → {parent_context}.user.id
/// 3. 絶対パス: ${connection.tenant} → connection.tenant
///
/// ProcessMemory fallback は廃止。全て state 参照で統一。
pub struct ParameterBuilder;

impl ParameterBuilder {
    /// placeholder を namespace ルールで解決
    ///
    /// # Arguments
    /// * `name` - placeholder 名（例: "id", "user.id", "connection.tenant"）
    /// * `context_key` - 現在のコンテキスト（例: "cache.user"）
    /// * `state_resolver` - state.get() への callback
    ///
    /// # Returns
    /// * 解決された値、または None
    pub fn resolve_placeholder<F>(
        name: &str,
        context_key: &str,
        state_resolver: &F,
    ) -> Option<Value>
    where
        F: Fn(&str) -> Option<Value>,
    {
        // 1. 同層参照を試行（相対パス優先）
        let same_level_key = format!("{}.{}", context_key, name);
        if let Some(value) = state_resolver(&same_level_key) {
            return Some(value);
        }

        // 2. 親層参照を試行（一つ上の階層）
        if name.contains('.') {
            if let Some(parent) = context_key.rsplit_once('.').map(|(p, _)| p) {
                let parent_ref_key = format!("{}.{}", parent, name);
                if let Some(value) = state_resolver(&parent_ref_key) {
                    return Some(value);
                }
            }
        }

        // 3. 絶対パス（フルパス）として試行
        if name.contains('.') {
            if let Some(value) = state_resolver(name) {
                return Some(value);
            }
        }

        // 4. 全て miss → None
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;
    use std::collections::HashMap;

    #[test]
    fn test_same_level_resolution() {
        // cache.user.id を ${id} で参照
        let mut mock_state = HashMap::new();
        mock_state.insert("cache.user.id".to_string(), Value::Number(123.into()));

        let resolver = |key: &str| mock_state.get(key).cloned();

        let result = ParameterBuilder::resolve_placeholder("id", "cache.user", &resolver);
        assert_eq!(result, Some(Value::Number(123.into())));
    }

    #[test]
    fn test_parent_level_resolution() {
        // cache.user.id を ${user.id} で参照（親層から）
        let mut mock_state = HashMap::new();
        mock_state.insert("cache.user.id".to_string(), Value::Number(456.into()));

        let resolver = |key: &str| mock_state.get(key).cloned();

        let result = ParameterBuilder::resolve_placeholder("user.id", "cache", &resolver);
        assert_eq!(result, Some(Value::Number(456.into())));
    }

    #[test]
    fn test_absolute_path_resolution() {
        // connection.tenant を ${connection.tenant} で参照
        let mut mock_state = HashMap::new();
        mock_state.insert(
            "connection.tenant".to_string(),
            Value::String("tenant_conn".to_string()),
        );

        let resolver = |key: &str| mock_state.get(key).cloned();

        let result =
            ParameterBuilder::resolve_placeholder("connection.tenant", "cache.user", &resolver);
        assert_eq!(
            result,
            Some(Value::String("tenant_conn".to_string()))
        );
    }

    #[test]
    fn test_resolution_priority() {
        // 同層 > 親層 > 絶対パス の優先順位を確認
        let mut mock_state = HashMap::new();
        mock_state.insert("cache.user.id".to_string(), Value::Number(1.into())); // 同層
        mock_state.insert("cache.id".to_string(), Value::Number(2.into())); // 親層
        mock_state.insert("id".to_string(), Value::Number(3.into())); // 絶対パス

        let resolver = |key: &str| mock_state.get(key).cloned();

        // 同層が優先される
        let result = ParameterBuilder::resolve_placeholder("id", "cache.user", &resolver);
        assert_eq!(result, Some(Value::Number(1.into())));
    }

    #[test]
    fn test_unresolved_placeholder() {
        // 全て miss → None
        let mock_state = HashMap::<String, Value>::new();
        let resolver = |key: &str| mock_state.get(key).cloned();

        let result = ParameterBuilder::resolve_placeholder("missing_key", "cache.user", &resolver);
        assert_eq!(result, None);
    }

    #[test]
    fn test_nested_context() {
        // 深いネスト: app.feature.module.component
        let mut mock_state = HashMap::new();
        mock_state.insert(
            "app.feature.module.component.value".to_string(),
            Value::String("nested".to_string()),
        );

        let resolver = |key: &str| mock_state.get(key).cloned();

        let result = ParameterBuilder::resolve_placeholder(
            "value",
            "app.feature.module.component",
            &resolver,
        );
        assert_eq!(result, Some(Value::String("nested".to_string())));
    }
}
