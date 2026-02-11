// Placeholder normalization integration tests
// PHP版 MainTest.php の testPlaceholderNormalization に対応

use state_engine::manifest::Manifest;
use std::fs;
use std::path::Path;

#[test]
fn test_placeholder_normalization_cross_file_reference() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/placeholder_norm");
    fs::create_dir_all(&manifest_dir).unwrap();

    // session.yml
    let session_yml = r#"
sso_user_id:
  _state:
    type: integer
"#;
    fs::write(manifest_dir.join("session.yml"), session_yml).unwrap();

    // connection.yml
    let connection_yml = r#"
tenant:
  _store:
    client: InMemory
    key: 'connection.tenant'
"#;
    fs::write(manifest_dir.join("connection.yml"), connection_yml).unwrap();

    // cache.yml
    let cache_yml = r#"
user:
  _store:
    client: KVS
    key: 'user:${session.sso_user_id}'
    ttl: 14400
  _load:
    client: DB
    connection: ${connection.tenant}
    table: 'users'
    where: 'sso_user_id=${session.sso_user_id}'

  org_id:
    _state:
      type: integer

  tenant_id:
    _state:
      type: integer
    _load:
      client: State
      key: ${org_id}
"#;
    fs::write(manifest_dir.join("cache.yml"), cache_yml).unwrap();

    let mut manifest = Manifest::new(manifest_dir.to_str().unwrap());

    // cache.user のメタデータを取得
    let meta = manifest.get_meta("cache.user");

    // 異なるファイルを参照する placeholder はそのまま（絶対パス）
    assert_eq!(
        meta.get("_store").unwrap().get("key").unwrap().as_str().unwrap(),
        "user:${session.sso_user_id}"
    );
    assert_eq!(
        meta.get("_load").unwrap().get("connection").unwrap().as_str().unwrap(),
        "${connection.tenant}"
    );
    assert_eq!(
        meta.get("_load").unwrap().get("where").unwrap().as_str().unwrap(),
        "sso_user_id=${session.sso_user_id}"
    );

    // cache.user.tenant_id のメタデータを取得
    let meta_tenant_id = manifest.get_meta("cache.user.tenant_id");

    // 同じファイル内の相対パス ${org_id} は親のコンテキストで絶対パス化される（ファイル名を含む）
    assert_eq!(
        meta_tenant_id.get("_load").unwrap().get("key").unwrap().as_str().unwrap(),
        "${cache.user.org_id}"
    );

    // クリーンアップ
    fs::remove_dir_all(&manifest_dir).ok();
}

#[test]
fn test_placeholder_normalization_relative_path() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/placeholder_norm2");
    fs::create_dir_all(&manifest_dir).unwrap();

    let test_yml = r#"
cache:
  user:
    _store:
      client: KVS
      key: 'user:${id}'
      ttl: 3600

    id:
      _state:
        type: integer

    active:
      _store:
        key: 'active_user:${id}'
"#;
    fs::write(manifest_dir.join("test.yml"), test_yml).unwrap();

    let mut manifest = Manifest::new(manifest_dir.to_str().unwrap());
    let meta = manifest.get_meta("test.cache.user.active");

    // 親から継承した _store.client
    assert_eq!(
        meta.get("_store").unwrap().get("client").unwrap().as_str().unwrap(),
        "KVS"
    );
    assert_eq!(
        meta.get("_store").unwrap().get("ttl").unwrap().as_u64().unwrap(),
        3600
    );

    // 子で上書きされた key（相対パスは正規化される、ファイル名を含む）
    assert_eq!(
        meta.get("_store").unwrap().get("key").unwrap().as_str().unwrap(),
        "active_user:${test.cache.user.id}"
    );

    // クリーンアップ
    fs::remove_dir_all(&manifest_dir).ok();
}

#[test]
fn test_placeholder_normalization_file_root_reference() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/placeholder_norm3");
    fs::create_dir_all(&manifest_dir).unwrap();

    let test_yml = r#"
root_value: 'test'

cache:
  user:
    _store:
      client: InMemory
      key: 'user:${root_value}'
"#;
    fs::write(manifest_dir.join("test.yml"), test_yml).unwrap();

    let mut manifest = Manifest::new(manifest_dir.to_str().unwrap());
    let meta = manifest.get_meta("test.cache.user");

    // root_value は test ファイルのルートに存在するため、絶対パスとして扱われる
    // (file.rootなので、そのまま)
    // ただし、親パスが空の場合は正規化しない仕様のため、そのまま
    assert_eq!(
        meta.get("_store").unwrap().get("key").unwrap().as_str().unwrap(),
        "user:${root_value}"
    );

    // クリーンアップ
    fs::remove_dir_all(&manifest_dir).ok();
}
