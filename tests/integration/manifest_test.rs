// Manifest module integration tests
use state_engine::Manifest;
use serde_json::json;

fn get_fixtures_path() -> String {
    // samples/manifest を使用（raw と同様）
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/manifest");
    manifest_path.to_str().unwrap().to_string()
}

#[test]
fn test_manifest_get_file_root() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    // ファイル全体を取得
    let result = manifest.get("connection", None);

    assert!(result.is_object());
    assert!(result.get("common").is_some());
    assert!(result.get("tenant").is_some());
}

#[test]
fn test_manifest_get_with_path() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    // ドット記法でネストされた値を取得
    // YAMLにデータ値はなく、_state定義のみなので空オブジェクトが返る
    let result = manifest.get("connection.common.host", None);

    assert_eq!(result, json!({}));
}

#[test]
fn test_manifest_get_nested_value() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    // YAMLにはメタデータのみでデータ値はないため、空オブジェクトが返る
    let host = manifest.get("connection.common.host", None);
    assert_eq!(host, json!({}));

    let port = manifest.get("connection.common.port", None);
    assert_eq!(port, json!({}));

    let database = manifest.get("connection.common.database", None);
    assert_eq!(database, json!({}));
}

#[test]
fn test_manifest_filter_meta() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    // _load などのメタデータは除外される
    let common = manifest.get("connection.common", None);

    assert!(common.is_object());
    // host, port等の子要素は存在する（空オブジェクトだが）
    assert!(common.get("host").is_some());
    assert!(common.get("port").is_some());
    // _load は除外される
    assert!(common.get("_load").is_none());
    assert!(common.get("_store").is_none());
    assert!(common.get("_state").is_none());
    assert!(common.get("_key").is_none());
}

#[test]
fn test_manifest_get_meta() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    // メタデータを取得
    let meta = manifest.get_meta("connection.common");

    assert!(meta.contains_key("_load"));

    if let Some(load) = meta.get("_load") {
        assert!(load.is_object());
        let expected_value = json!("Env");
        assert_eq!(load.get("client"), Some(&expected_value));
    }
}

#[test]
fn test_manifest_get_cache_scope() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    let user = manifest.get("cache.user", None);

    assert!(user.is_object());
    // id, org_id, tenant_id は含まれる
    assert!(user.get("id").is_some());
    assert!(user.get("org_id").is_some());
    assert!(user.get("tenant_id").is_some());
    // メタデータは除外される
    assert!(user.get("_state").is_none());
    assert!(user.get("_store").is_none());
    assert!(user.get("_load").is_none());
}

#[test]
fn test_manifest_get_cache_user_root_meta() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    let meta = manifest.get_meta("cache.user");

    assert!(meta.contains_key("_state"));
    assert!(meta.contains_key("_store"));
    assert!(meta.contains_key("_load"));

    // _store内の設定確認
    if let Some(store) = meta.get("_store") {
        assert_eq!(store.get("client"), Some(&json!("KVS")));
        assert_eq!(store.get("key"), Some(&json!("user:${sso_user_id}")));
        assert_eq!(store.get("ttl"), Some(&json!(14400)));
    }
}

#[test]
fn test_manifest_get_cache_tenant_id_meta() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    let meta = manifest.get_meta("cache.user.tenant_id");

    // userレベルのメタデータ（継承）
    assert!(meta.contains_key("_state"));
    assert!(meta.contains_key("_store"));

    // tenant_id固有のメタデータ
    assert!(meta.contains_key("_load"));

    // EXPRESSION clientの確認
    if let Some(load) = meta.get("_load") {
        assert_eq!(load.get("client"), Some(&json!("EXPRESSION")));
        assert!(load.get("expression").is_some());
    }
}

#[test]
fn test_manifest_get_cache_session_meta() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    let meta = manifest.get_meta("cache.session");

    assert!(meta.contains_key("_state"));
    assert!(meta.contains_key("_store"));
    // sessionは_loadなし（set専用）
    assert!(!meta.contains_key("_load"));

    // _store内のttl確認
    if let Some(store) = meta.get("_store") {
        assert_eq!(store.get("ttl"), Some(&json!(1800)));  // 30 minutes
    }
}

#[test]
fn test_manifest_missing_file() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    // 存在しないファイルを参照
    let result = manifest.get("nonexistent.key", Some(json!("default")));

    // デフォルト値が返る
    assert_eq!(result, json!("default"));

    // missing_keys に記録される
    let missing = manifest.get_missing_keys();
    assert!(missing.contains(&"nonexistent.key".to_string()));
}

#[test]
fn test_manifest_missing_key() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    // 存在しないキーを参照
    let result = manifest.get("connection.common.missing_key", Some(json!("default")));

    assert_eq!(result, json!("default"));

    let missing = manifest.get_missing_keys();
    assert!(missing.contains(&"connection.common.missing_key".to_string()));
}

#[test]
fn test_manifest_clear_missing_keys() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    // 存在しないキーでmissing_keysに追加
    manifest.get("connection.missing", None);
    assert!(!manifest.get_missing_keys().is_empty());

    // クリア
    manifest.clear_missing_keys();
    assert!(manifest.get_missing_keys().is_empty());
}

#[test]
fn test_manifest_connection_common_meta() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    let meta = manifest.get_meta("connection.common");

    assert!(meta.contains_key("_state"));
    assert!(meta.contains_key("_store"));
    assert!(meta.contains_key("_load"));

    // _store確認
    if let Some(store) = meta.get("_store") {
        assert_eq!(store.get("client"), Some(&json!("InMemory")));
        assert_eq!(store.get("key"), Some(&json!("connection.common")));
    }

    // _load確認
    if let Some(load) = meta.get("_load") {
        assert_eq!(load.get("client"), Some(&json!("Env")));
        assert!(load.get("map").is_some());
    }
}

#[test]
fn test_manifest_cache_tenant_structure() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    // cache.tenant のデータ取得（メタデータなし）
    let tenant = manifest.get("cache.tenant", None);
    assert!(tenant.is_object());
    // name, display_name は含まれる
    assert!(tenant.get("name").is_some());
    assert!(tenant.get("display_name").is_some());
    // メタデータは除外される
    assert!(tenant.get("_state").is_none());
    assert!(tenant.get("_store").is_none());
    assert!(tenant.get("_load").is_none());
}

#[test]
fn test_manifest_connection_tenant() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    // YAMLにはデータ値がなくメタデータのみなので空オブジェクトが返る
    let tenant_host = manifest.get("connection.tenant.host", None);
    assert_eq!(tenant_host, json!({}));

    let tenant_db = manifest.get("connection.tenant.database", None);
    assert_eq!(tenant_db, json!({}));
}
