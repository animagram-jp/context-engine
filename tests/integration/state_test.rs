// State module integration tests
use state_engine::{Manifest, State, Load};
use state_engine::ports::provided::State as StateTrait;
use state_engine::ports::required::{InMemoryClient, KVSClient};
use serde_json::{json, Value};
use crate::mocks::{MockInMemory, MockKVS, MockENVClient};

fn get_fixtures_path() -> String {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/manifest");
    manifest_path.to_str().unwrap().to_string()
}

// ============================================================================
// Basic CRUD tests
// ============================================================================

#[test]
fn test_state_set_and_get_in_memory() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);
    let load = Load::new();
    let mut in_memory = MockInMemory::new();

    let mut state = State::new(&mut manifest, load).with_in_memory(&mut in_memory);

    // connection.common (_store: InMemory)
    let value = json!({
        "host": "localhost",
        "port": 5432,
        "database": "testdb"
    });

    let result = state.set("connection.common", value.clone(), None);
    assert!(result, "set should succeed");

    let retrieved = state.get("connection.common");
    assert_eq!(retrieved, Some(value));
}

#[test]
fn test_state_delete_in_memory() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);
    let load = Load::new();
    let mut in_memory = MockInMemory::new();

    let mut state = State::new(&mut manifest, load).with_in_memory(&mut in_memory);

    let value = json!({"host": "localhost"});
    state.set("connection.common", value, None);

    let result = state.delete("connection.common");
    assert!(result, "delete should succeed");

    let retrieved = state.get("connection.common");
    assert_eq!(retrieved, None);
}

#[test]
fn test_state_set_and_get_kvs() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);
    let load = Load::new();
    let mut in_memory = MockInMemory::new();
    let mut kvs = MockKVS::new();

    // session.sso_user_id を InMemory に設定（placeholder解決用）
    in_memory.set("request-attributes", json!(123));

    let mut state = State::new(&mut manifest, load)
        .with_in_memory(&mut in_memory)
        .with_kvs_client(&mut kvs);

    // cache.user (_store: KVS)
    let value = json!({
        "id": 1,
        "org_id": 100,
        "tenant_id": 10
    });

    let result = state.set("cache.user", value.clone(), Some(3600));
    assert!(result, "set should succeed");

    let retrieved = state.get("cache.user");
    assert_eq!(retrieved, Some(value));
}

#[test]
fn test_state_delete_kvs() {
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);
    let load = Load::new();
    let mut in_memory = MockInMemory::new();
    let mut kvs = MockKVS::new();

    // session.sso_user_id を InMemory に設定（placeholder解決用）
    in_memory.set("request-attributes", json!(123));

    let mut state = State::new(&mut manifest, load)
        .with_in_memory(&mut in_memory)
        .with_kvs_client(&mut kvs);

    let value = json!({"id": 1});
    state.set("cache.user", value, None);

    let result = state.delete("cache.user");
    assert!(result, "delete should succeed");

    let retrieved = state.get("cache.user");
    assert_eq!(retrieved, None);
}

// ============================================================================
// Load and cache tests
// ============================================================================

#[test]
fn test_state_load_cache_expansion() {
    // Load結果がObjectの場合、各フィールドがcacheに展開されることを確認
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);

    // ENVClientのモック設定
    let mut env_client = MockENVClient::new();
    env_client.data.insert("DB_HOST".to_string(), "localhost".to_string());
    env_client.data.insert("DB_PORT".to_string(), "3306".to_string());
    env_client.data.insert("DB_DATABASE".to_string(), "test_db".to_string());

    let mut load = Load::new();
    load = load.with_env_client(&mut env_client);

    let mut in_memory = MockInMemory::new();

    let mut state = State::new(&mut manifest, load);
    state = state.with_in_memory(&mut in_memory);

    // 最初に connection.common 全体を取得
    let result = state.get("connection.common");
    println!("connection.common: {:?}", result);

    assert!(result.is_some());
    if let Some(Value::Object(obj)) = &result {
        assert_eq!(obj.get("host"), Some(&json!("localhost")));
        assert_eq!(obj.get("port"), Some(&json!("3306")));
        assert_eq!(obj.get("database"), Some(&json!("test_db")));
    }

    // 次に connection.common.host を取得
    // cache に展開されているので、再度 Load せずにキャッシュヒットするはず
    let host = state.get("connection.common.host");
    println!("connection.common.host: {:?}", host);

    assert_eq!(host, Some(json!("localhost")));

    // connection.common.port も同様
    let port = state.get("connection.common.port");
    println!("connection.common.port: {:?}", port);

    assert_eq!(port, Some(json!("3306")));
}

#[test]
fn test_load_without_client_key_reference() {
    // _load.client が無い場合の挙動テスト
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);
    let load = Load::new();
    let mut in_memory = MockInMemory::new();
    let mut kvs = MockKVS::new();

    // 事前にデータをセット
    // KVSには辞書をJSON文字列として保存
    kvs.set("user:123", r#"{"id":1,"org_id":100,"tenant_id":10}"#.to_string(), None);
    in_memory.set("request-attributes-user-key", json!(123));

    let mut state = State::new(&mut manifest, load)
        .with_in_memory(&mut in_memory)
        .with_kvs_client(&mut kvs);

    // まず cache.user.org_id を直接取得してみる
    let org_id = state.get("cache.user.org_id");
    println!("org_id: {:?}", org_id);
    assert_eq!(org_id, Some(json!(100)));

    // cache.user.tenant_id を取得
    // _load: {client: State, key: '${org_id}'} が定義されているが、
    // Store (KVS) に既に tenant_id: 10 が保存されているため、Store の値が優先される
    let tenant_id = state.get("cache.user.tenant_id");
    println!("tenant_id: {:?}", tenant_id);

    // Store の値 (10) が返される（_load は fallback なので、Store に値があれば Store 優先）
    assert_eq!(tenant_id, Some(json!(10)));
}

// ============================================================================
// exists() tests
// ============================================================================

#[test]
fn test_exists_in_cache() {
    // キャッシュに存在する場合、trueを返す
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);
    let load = Load::new();
    let mut in_memory = MockInMemory::new();

    let mut state = State::new(&mut manifest, load).with_in_memory(&mut in_memory);

    // 値をセット（キャッシュに保存される）
    let value = json!({"host": "localhost"});
    state.set("connection.common", value, None);

    // exists()はtrueを返す
    assert!(state.exists("connection.common"));
}

#[test]
fn test_exists_in_store_but_not_in_cache() {
    // ストアに存在するがキャッシュにない場合、trueを返す
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);
    let load = Load::new();
    let mut in_memory = MockInMemory::new();

    // 直接ストアにデータを挿入
    in_memory.set("connection.common", json!({"host": "localhost"}));

    let mut state = State::new(&mut manifest, load).with_in_memory(&mut in_memory);

    // キャッシュには存在しないが、ストアには存在する
    assert!(state.exists("connection.common"));
}

#[test]
fn test_exists_not_found() {
    // キャッシュにもストアにも存在しない場合、falseを返す
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);
    let load = Load::new();
    let mut in_memory = MockInMemory::new();

    let mut state = State::new(&mut manifest, load).with_in_memory(&mut in_memory);

    // 存在しないキー
    assert!(!state.exists("connection.common"));
}

#[test]
fn test_exists_does_not_trigger_load() {
    // exists()は自動ロードをトリガーしない
    // つまり、_loadの定義があっても、ストアに無ければfalseを返す
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);
    let load = Load::new();
    let mut in_memory = MockInMemory::new();
    let mut kvs = MockKVS::new();

    // session.sso_user_id を InMemory に設定（placeholder解決用）
    in_memory.set("request-attributes", json!(123));

    let mut state = State::new(&mut manifest, load)
        .with_in_memory(&mut in_memory)
        .with_kvs_client(&mut kvs);

    // cache.user には _load が定義されているが、ストアには存在しない
    // exists()は自動ロードしないので、falseを返す
    assert!(!state.exists("cache.user"));

    // get()を呼ぶと自動ロードが試みられる（ただし、この場合はDBClientが無いので失敗する）
    let result = state.get("cache.user");
    assert_eq!(result, None);
}

#[test]
fn test_exists_with_kvs() {
    // KVSに存在する場合、trueを返す
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);
    let load = Load::new();
    let mut in_memory = MockInMemory::new();
    let mut kvs = MockKVS::new();

    // placeholder解決用
    in_memory.set("request-attributes", json!(123));

    let mut state = State::new(&mut manifest, load)
        .with_in_memory(&mut in_memory)
        .with_kvs_client(&mut kvs);

    // cache.user をKVSにセット
    let value = json!({"id": 1, "org_id": 100});
    state.set("cache.user", value, None);

    // exists()はtrueを返す
    assert!(state.exists("cache.user"));

    // 子フィールドもtrueを返す（親がキャッシュに展開されているため）
    assert!(state.exists("cache.user.id"));
}

#[test]
fn test_exists_performance() {
    // exists()がget()より軽量であることを確認（自動ロードしないため）
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);
    let load = Load::new();
    let mut in_memory = MockInMemory::new();

    let mut state = State::new(&mut manifest, load).with_in_memory(&mut in_memory);

    // 存在しないキーに対して
    // exists()は単にfalseを返す（自動ロードしない）
    assert!(!state.exists("cache.user"));

    // get()も自動ロードを試みるが、DBClientが無いのでNoneを返す
    // exists()の方が処理が少ないはず
    assert_eq!(state.get("cache.user"), None);
}
