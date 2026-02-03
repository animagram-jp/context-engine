// Resolver - placeholder 解決と再帰制御
//
// State と Load の中間層。placeholder を namespace ルールで解決し、
// 自己再帰を管理する。

use crate::common::PlaceholderResolver;
use crate::load::Load;
use serde_json::Value;
use std::collections::HashMap;

/// Resolver - 再帰エンジン
///
/// placeholder 解決と再帰深度管理を担当。
pub struct Resolver<'a> {
    load: Load<'a>,
    recursion_depth: usize,
    max_recursion: usize,
}

impl<'a> Resolver<'a> {
    /// 新しい Resolver を作成
    pub fn new(load: Load<'a>) -> Self {
        Self {
            load,
            recursion_depth: 0,
            max_recursion: 10,
        }
    }

    /// load_config 内の placeholder を解決して Load を実行
    ///
    /// # Arguments
    /// * `context_key` - 現在のコンテキスト（例: "cache.user"）
    /// * `load_config` - _load メタデータ
    /// * `state_callback` - State.get() への callback（自己再帰用）
    ///
    /// # Returns
    /// * `Ok(Value)` - ロード成功
    /// * `Err(String)` - ロード失敗
    pub fn handle<F>(
        &mut self,
        context_key: &str,
        load_config: &HashMap<String, Value>,
        mut state_callback: F,
    ) -> Result<Value, String>
    where
        F: FnMut(&str) -> Option<Value>,
    {
        // 1. 再帰深度チェック
        if self.recursion_depth >= self.max_recursion {
            return Err(format!(
                "Resolver::handle: max recursion depth ({}) reached",
                self.max_recursion
            ));
        }
        self.recursion_depth += 1;

        // 2. placeholder 解決
        // load_config を Value::Object に変換
        let config_map: serde_json::Map<String, Value> = load_config
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let config_value = Value::Object(config_map);

        // placeholder resolver: namespace ルールで解決
        // FnMut を受け入れるために内部で clone
        let mut resolver = |placeholder_name: &str| -> Option<Value> {
            // namespace 解決順序:
            // 1. 同層参照: ${id} → {context_key}.id
            let same_level_key = format!("{}.{}", context_key, placeholder_name);
            if let Some(value) = state_callback(&same_level_key) {
                return Some(value);
            }

            // 2. 親層参照: ${user.id} → {parent_context}.user.id
            if placeholder_name.contains('.') {
                if let Some(parent) = context_key.rsplit_once('.').map(|(p, _)| p) {
                    let parent_ref_key = format!("{}.{}", parent, placeholder_name);
                    if let Some(value) = state_callback(&parent_ref_key) {
                        return Some(value);
                    }
                }
            }

            // 3. 絶対パス: ${connection.tenant} → connection.tenant
            if placeholder_name.contains('.') {
                if let Some(value) = state_callback(placeholder_name) {
                    return Some(value);
                }
            }

            // 4. 全て miss → None
            None
        };

        // PlaceholderResolver で型付き解決
        // resolver を immutable として扱うため、内部で mut にラップ
        let resolved_config_value = {
            let resolver_fn = |name: &str| resolver(name);
            PlaceholderResolver::resolve_typed(config_value, &resolver_fn)
        };

        // HashMap に戻す
        let resolved_config: HashMap<String, Value> = if let Value::Object(map) =
            resolved_config_value
        {
            map.into_iter().collect()
        } else {
            self.recursion_depth -= 1;
            return Err("Resolver::handle: failed to resolve config".to_string());
        };

        // 3. Load 実行
        let result = self.load.handle(&resolved_config);

        self.recursion_depth -= 1;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::required::{
        DBClient, ENVClient, ExpressionClient, ConnectionConfig
    };
    use std::collections::HashMap;

    // Mock ENVClient
    struct MockENVClient {
        data: HashMap<String, String>,
    }

    impl ENVClient for MockENVClient {
        fn get(&self, key: &str) -> Option<String> {
            self.data.get(key).cloned()
        }
    }

    // Mock ExpressionClient
    struct MockExpressionClient;

    impl ExpressionClient for MockExpressionClient {
        fn evaluate(&self, expression: &str) -> Result<Value, String> {
            // 簡易実装
            Ok(Value::String(format!("evaluated: {}", expression)))
        }
    }

    #[test]
    fn test_resolver_with_env_client() {
        // ENV client 経由のロード
        let mut env_data = HashMap::new();
        env_data.insert("DB_HOST".to_string(), "localhost".to_string());

        let env_client = MockENVClient { data: env_data };
        let load = Load::new().with_env_client(&env_client);
        let mut resolver = Resolver::new(load);

        let mut load_config = HashMap::new();
        load_config.insert("client".to_string(), Value::String("Env".to_string()));

        let mut map = serde_json::Map::new();
        map.insert("host".to_string(), Value::String("DB_HOST".to_string()));
        load_config.insert("map".to_string(), Value::Object(map));

        let state_callback = |_key: &str| -> Option<Value> { None };

        let result = resolver.handle("connection.common", &load_config, state_callback);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_recursion_limit() {
        let load = Load::new();
        let mut resolver = Resolver::new(load);
        resolver.max_recursion = 2;

        let load_config = HashMap::new();

        // 自己再帰する callback
        let mut recursion_count = 0;
        let state_callback = |_key: &str| -> Option<Value> {
            recursion_count += 1;
            None
        };

        // 3回目で失敗するはず
        for _ in 0..3 {
            let _ = resolver.handle("test", &load_config, |k| state_callback(k));
        }

        // max_recursion 超過でエラー
        let result = resolver.handle("test", &load_config, state_callback);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max recursion depth"));
    }
}
