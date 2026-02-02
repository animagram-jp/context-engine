// ParameterBuilder - プレースホルダー値の解決
//
// State/Load層で使用するパラメータを構築する。

use crate::common::PlaceholderResolver;
use crate::ports::provided::Manifest;
use crate::ports::required::ProcessMemoryClient;
use serde_json::Value;
use std::collections::HashMap;

/// ParameterBuilder
///
/// manifest keyから必要なプレースホルダーを抽出し、
/// 各種ソース（ProcessMemory等）から値を解決する。
pub struct ParameterBuilder<'a> {
    manifest: &'a mut dyn Manifest,
    process_memory: Option<&'a dyn ProcessMemoryClient>,
}

impl<'a> ParameterBuilder<'a> {
    /// 新しいParameterBuilderを作成
    pub fn new(manifest: &'a mut dyn Manifest) -> Self {
        Self {
            manifest,
            process_memory: None,
        }
    }

    /// ProcessMemoryClientを設定
    pub fn with_process_memory(mut self, client: &'a dyn ProcessMemoryClient) -> Self {
        self.process_memory = Some(client);
        self
    }

    /// manifest keyからパラメータを構築
    ///
    /// # Arguments
    /// * `key` - manifest key ("cache.user.tenant_id")
    /// * `additional_params` - 追加パラメータ（優先）
    ///
    /// # Returns
    /// * プレースホルダー名と値のマップ
    ///
    /// # 解決優先順位
    /// 1. additional_params（引数で渡された値）
    /// 2. ProcessMemory "placeholder.{name}"
    /// 3. ProcessMemory "userkey.{name}" (sso_user_idなど)
    /// 4. 空文字列（未解決）
    pub fn build(
        &mut self,
        key: &str,
        additional_params: HashMap<String, String>,
    ) -> HashMap<String, String> {
        // メタデータ取得
        let meta = self.manifest.get_meta(key);
        if meta.is_empty() {
            return additional_params;
        }

        // キーテンプレート収集
        let mut templates = Vec::new();

        // _store.key
        if let Some(store) = meta.get("_store").and_then(|v| v.as_object()) {
            if let Some(key_template) = store.get("key").and_then(|v| v.as_str()) {
                templates.push(key_template);
            }
        }

        // _load.where
        if let Some(load) = meta.get("_load").and_then(|v| v.as_object()) {
            if let Some(where_template) = load.get("where").and_then(|v| v.as_str()) {
                templates.push(where_template);
            }
            // _load.key (KVS/InMemory)
            if let Some(key_template) = load.get("key").and_then(|v| v.as_str()) {
                templates.push(key_template);
            }
            // _load.url (API)
            if let Some(url_template) = load.get("url").and_then(|v| v.as_str()) {
                templates.push(url_template);
            }
            // _load.expression (EXPRESSION)
            if let Some(expr_template) = load.get("expression").and_then(|v| v.as_str()) {
                templates.push(expr_template);
            }
        }

        // プレースホルダー抽出
        let mut placeholders = Vec::new();
        for template in templates {
            let extracted = PlaceholderResolver::extract_placeholders(template);
            placeholders.extend(extracted);
        }

        // 重複削除
        placeholders.sort();
        placeholders.dedup();

        // パラメータ構築
        let mut params = additional_params;
        for name in placeholders {
            if !params.contains_key(&name) {
                if let Some(value) = self.resolve_value(&name, key) {
                    params.insert(name, value);
                }
            }
        }

        params
    }

    /// プレースホルダー値を解決
    ///
    /// # 解決優先順位
    /// 1. ProcessMemory "placeholder.{name}"
    /// 2. ProcessMemory "userkey.{name}" (sso_user_id, tenant_id, org_id等)
    /// 3. None（未解決）
    fn resolve_value(&self, name: &str, _context_key: &str) -> Option<String> {
        let process_memory = self.process_memory?;

        // 1. placeholder namespace
        if let Some(value) = process_memory.get(&format!("placeholder.{}", name)) {
            return Self::value_to_string(value);
        }

        // 2. userkey namespace (特定のキー用)
        match name {
            "sso_user_id" | "tenant_id" | "org_id" | "session_id" => {
                if let Some(value) = process_memory.get(&format!("userkey.{}", name)) {
                    return Self::value_to_string(value);
                }
            }
            _ => {}
        }

        // 3. 未解決
        None
    }

    /// Valueを文字列に変換
    fn value_to_string(value: Value) -> Option<String> {
        match value {
            Value::String(s) => Some(s),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Manifest as ManifestImpl;

    // Mock ProcessMemoryClient
    struct MockProcessMemory {
        data: HashMap<String, Value>,
    }

    impl MockProcessMemory {
        fn new() -> Self {
            Self {
                data: HashMap::new(),
            }
        }

        fn set(&mut self, key: &str, value: Value) {
            self.data.insert(key.to_string(), value);
        }
    }

    impl ProcessMemoryClient for MockProcessMemory {
        fn get(&self, key: &str) -> Option<Value> {
            self.data.get(key).cloned()
        }

        fn set(&mut self, key: &str, value: Value) {
            self.data.insert(key.to_string(), value);
        }

        fn delete(&mut self, key: &str) -> bool {
            self.data.remove(key).is_some()
        }
    }

    #[test]
    fn test_parameter_builder_extract_placeholders() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/manifest");
        let mut manifest = ManifestImpl::new(manifest_path.to_str().unwrap());

        let mut process_memory = MockProcessMemory::new();
        process_memory.set("userkey.sso_user_id", Value::String("user001".to_string()));

        let mut builder = ParameterBuilder::new(&mut manifest)
            .with_process_memory(&process_memory);

        let params = builder.build("cache.user", HashMap::new());

        // cache.user の _store.key: "user:${sso_user_id}" から抽出
        assert!(params.contains_key("sso_user_id"));
        assert_eq!(params.get("sso_user_id"), Some(&"user001".to_string()));
    }

    #[test]
    fn test_parameter_builder_additional_params_priority() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/manifest");
        let mut manifest = ManifestImpl::new(manifest_path.to_str().unwrap());

        let mut process_memory = MockProcessMemory::new();
        process_memory.set("userkey.sso_user_id", Value::String("user001".to_string()));

        let mut builder = ParameterBuilder::new(&mut manifest)
            .with_process_memory(&process_memory);

        let mut additional = HashMap::new();
        additional.insert("sso_user_id".to_string(), "override_user".to_string());

        let params = builder.build("cache.user", additional);

        // additional_params が優先される
        assert_eq!(params.get("sso_user_id"), Some(&"override_user".to_string()));
    }

    #[test]
    fn test_parameter_builder_placeholder_namespace() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/manifest");
        let mut manifest = ManifestImpl::new(manifest_path.to_str().unwrap());

        let mut process_memory = MockProcessMemory::new();
        process_memory.set("placeholder.tenant_id", Value::Number(123.into()));

        let mut builder = ParameterBuilder::new(&mut manifest)
            .with_process_memory(&process_memory);

        let params = builder.build("cache.tenant", HashMap::new());

        // placeholder namespace から取得
        assert_eq!(params.get("tenant_id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_parameter_builder_no_process_memory() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/manifest");
        let mut manifest = ManifestImpl::new(manifest_path.to_str().unwrap());

        let mut builder = ParameterBuilder::new(&mut manifest);

        let params = builder.build("cache.user", HashMap::new());

        // ProcessMemoryがない場合、プレースホルダーは抽出されるが値は解決されない
        // sso_user_idキーは存在しないか、値がない
        assert!(!params.contains_key("sso_user_id") || params.get("sso_user_id").is_none());
    }
}
