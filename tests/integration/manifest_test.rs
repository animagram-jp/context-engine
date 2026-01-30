// Manifest module integration tests
use conduct_engine::Manifest;
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
    let result = manifest.get("connection.common.name", None);

    assert_eq!(result, json!("test_common"));
}

#[test]
fn test_manifest_get_nested_value() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    let host = manifest.get("connection.common.host", None);
    assert_eq!(host, json!("localhost"));

    let port = manifest.get("connection.common.port", None);
    assert_eq!(port, json!(5432));

    let database = manifest.get("connection.common.database", None);
    assert_eq!(database, json!("common_db"));
}

#[test]
fn test_manifest_filter_meta() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    // _load などのメタデータは除外される
    let common = manifest.get("connection.common", None);

    assert!(common.is_object());
    assert!(common.get("name").is_some());
    assert!(common.get("host").is_some());
    // _load は除外される
    assert!(common.get("_load").is_none());
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
        assert_eq!(load.get("source"), Some(&json!("ENV")));
    }
}

#[test]
fn test_manifest_get_cache_scope() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    let app = manifest.get("cache.app", None);

    assert!(app.is_object());
    // org_tenant_map は含まれる
    assert!(app.get("org_tenant_map").is_some());
    // _keyPrefix などのメタデータは除外される
    assert!(app.get("_keyPrefix").is_none());
    assert!(app.get("_ttl").is_none());
}

#[test]
fn test_manifest_get_cache_app_meta() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    let meta = manifest.get_meta("cache.app");

    assert!(meta.contains_key("_keyPrefix"));
    assert!(meta.contains_key("_ttl"));

    assert_eq!(meta.get("_keyPrefix"), Some(&json!("app:")));
    assert_eq!(meta.get("_ttl"), Some(&json!(null)));
}

#[test]
fn test_manifest_get_cache_org_tenant_map_meta() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    let meta = manifest.get_meta("cache.app.org_tenant_map");

    // appレベルのメタデータ
    assert!(meta.contains_key("_keyPrefix"));
    // org_tenant_mapレベルのメタデータ
    assert!(meta.contains_key("_key"));
    assert!(meta.contains_key("_structure"));
    assert!(meta.contains_key("_load"));

    assert_eq!(meta.get("_key"), Some(&json!("org_tenant_map")));
}

#[test]
fn test_manifest_get_cache_user_meta() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    let meta = manifest.get_meta("cache.user");

    assert!(meta.contains_key("_key"));
    assert!(meta.contains_key("_ttl"));
    assert!(meta.contains_key("_load"));

    assert_eq!(meta.get("_key"), Some(&json!("user:{sso_user_id}")));
    assert_eq!(meta.get("_ttl"), Some(&json!(14400)));
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
fn test_manifest_database_yml() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    let tenants = manifest.get("database.tenants", None);
    assert!(tenants.is_object());
    // メタデータは除外される
    assert!(tenants.get("_connection").is_none());
    assert!(tenants.get("_table").is_none());

    // メタデータは get_meta で取得できる
    let meta = manifest.get_meta("database.tenants");
    assert_eq!(meta.get("_connection"), Some(&json!("common")));
    assert_eq!(meta.get("_table"), Some(&json!("tenants")));
}

#[test]
fn test_manifest_cache_structure() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    // cache.app.org_tenant_map のデータ取得（メタデータなし）
    let org_tenant_map = manifest.get("cache.app.org_tenant_map", None);
    assert!(org_tenant_map.is_object());
    // _key, _structure, _load は除外される
    assert!(org_tenant_map.get("_key").is_none());
    assert!(org_tenant_map.get("_structure").is_none());
    assert!(org_tenant_map.get("_load").is_none());
}

#[test]
fn test_manifest_connection_tenant() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    let tenant_name = manifest.get("connection.tenant.name", None);
    assert_eq!(tenant_name, json!("tenant_{tenant_id}"));

    let tenant_db = manifest.get("connection.tenant.database", None);
    assert_eq!(tenant_db, json!("db_tenant{tenant_id}"));
}
