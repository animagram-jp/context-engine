// Manifest module integration tests
use state_engine::Manifest;
use serde_json::json;

fn get_fixtures_path() -> String {
    // examples/manifest を使用（raw と同様）
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/manifest");
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
fn test_manifest_get_cache_user_root_meta() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    let meta = manifest.get_meta("cache.user");

    // user は _state が省略されている（子要素があるため自明）
    assert!(meta.contains_key("_store"));
    assert!(meta.contains_key("_load"));

    // _store内の設定確認
    if let Some(store) = meta.get("_store") {
        assert_eq!(store.get("client"), Some(&json!("KVS")));
        assert_eq!(store.get("key"), Some(&json!("user:${session.sso_user_id}")));
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

    // client: State の_load（keyのみの相対参照）
    if let Some(load) = meta.get("_load") {
        // client: State の場合はloadを呼ばずkeyの値を使用（親の_load継承を防ぐため明示）
        assert_eq!(load.get("client"), Some(&json!("State")));
        // placeholder正規化により ${org_id} → ${cache.user.org_id} に変換される
        assert_eq!(load.get("key"), Some(&json!("${cache.user.org_id}")));
    }
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
fn test_manifest_connection_common_meta() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    let meta = manifest.get_meta("connection.common");

    // connection.common には _state はない（子ノードにのみ存在）
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
    // name は含まれる
    assert!(tenant.get("name").is_some());
    // メタデータは除外される
    assert!(tenant.get("_state").is_none());
    assert!(tenant.get("_store").is_none());
    assert!(tenant.get("_load").is_none());
}

#[test]
fn test_manifest_get_meta_qualified_path_for_load_map() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    // cache.user の _load.map のキーが絶対パスに正規化されることを確認
    let meta = manifest.get_meta("cache.user");

    assert!(meta.contains_key("_load"));

    if let Some(load) = meta.get("_load") {
        assert!(load.is_object());

        if let Some(map) = load.get("map") {
            assert!(map.is_object());

            // 相対パス "id" → 絶対パス "cache.user.id"
            assert!(map.get("cache.user.id").is_some());
            assert_eq!(map.get("cache.user.id"), Some(&json!("id")));

            // 相対パス "org_id" → 絶対パス "cache.user.org_id"
            assert!(map.get("cache.user.org_id").is_some());
            assert_eq!(map.get("cache.user.org_id"), Some(&json!("sso_org_id")));

            // 相対パスのキーは存在しない
            assert!(map.get("id").is_none());
            assert!(map.get("org_id").is_none());
        }
    }
}

#[test]
fn test_manifest_get_meta_absolute_path_for_nested_load_map() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    // cache.tenant の _load.map も絶対パスに正規化される
    let meta = manifest.get_meta("cache.tenant");

    assert!(meta.contains_key("_load"));

    if let Some(load) = meta.get("_load") {
        if let Some(map) = load.get("map") {
            assert!(map.is_object());

            // 相対パス "name" → 絶対パス "cache.tenant.name"
            assert!(map.get("cache.tenant.name").is_some());
            assert_eq!(map.get("cache.tenant.name"), Some(&json!("name")));

            // 相対パスのキーは存在しない
            assert!(map.get("name").is_none());
        }
    }
}

#[test]
fn test_manifest_get_meta_child_node_without_load_map() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    // cache.user.tenant_id は _load を持つ
    // 親の _load とマージされるため、親の map も継承される
    let meta = manifest.get_meta("cache.user.tenant_id");

    assert!(meta.contains_key("_load"));

    if let Some(load) = meta.get("_load") {
        // tenant_id の _load は親とマージされる
        // 子の client: State が親の client: DB を上書き
        assert_eq!(load.get("client"), Some(&json!("State")));
        // placeholder正規化により ${org_id} → ${cache.user.org_id} に変換される
        assert_eq!(load.get("key"), Some(&json!("${cache.user.org_id}")));

        // 親の map が継承される（_load のマージルール）
        // ただし、client: State の場合 Load::handle() は map を無視する
        assert!(load.get("map").is_some());

        // map は親（user）レベルで定義されているため、絶対パス化されている
        if let Some(map) = load.get("map") {
            assert!(map.is_object());
            // cache.user.id, cache.user.org_id が含まれる
            assert!(map.get("cache.user.id").is_some());
            assert!(map.get("cache.user.org_id").is_some());
        }
    }
}

#[test]
fn test_manifest_yaml_extension_support() {
    // .yaml 拡張子のファイルを作成してテスト
    let fixtures_path = get_fixtures_path();
    let yaml_file_path = std::path::Path::new(&fixtures_path).join("test_yaml.yaml");

    // テスト用 .yaml ファイルを作成
    std::fs::write(&yaml_file_path, r#"
test_node:
  _state:
    type: string
  value: "test value"
"#).unwrap();

    let mut manifest = Manifest::new(&fixtures_path);

    // .yaml 拡張子のファイルを読み込めることを確認
    let result = manifest.get("test_yaml.test_node.value", None);
    assert_eq!(result, json!("test value"));

    // クリーンアップ
    std::fs::remove_file(&yaml_file_path).ok();
}

#[test]
fn test_manifest_ambiguous_extension_error() {
    // 同名で .yml と .yaml が両方存在する場合のエラーテスト
    let fixtures_path = get_fixtures_path();
    let yml_file_path = std::path::Path::new(&fixtures_path).join("ambiguous.yml");
    let yaml_file_path = std::path::Path::new(&fixtures_path).join("ambiguous.yaml");

    // 両方のファイルを作成
    std::fs::write(&yml_file_path, "test: yml").unwrap();
    std::fs::write(&yaml_file_path, "test: yaml").unwrap();

    let mut manifest = Manifest::new(&fixtures_path);

    // エラーが返されることを確認
    let result = manifest.get("ambiguous.test", None);
    assert_eq!(result, json!(null));

    // missing_keys に記録されることを確認
    let missing = manifest.get_missing_keys();
    assert!(missing.contains(&"ambiguous.test".to_string()));

    // クリーンアップ
    std::fs::remove_file(&yml_file_path).ok();
    std::fs::remove_file(&yaml_file_path).ok();
}
