// State module integration tests
use state_engine::{Manifest, State, Load};
use state_engine::ports::provided::State as StateTrait;
use state_engine::ports::required::InMemoryClient;
use serde_json::{json, Value};
use crate::mocks::{MockInMemory, MockKVS, MockENVClient};

fn get_fixtures_path() -> String {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/manifest");
    manifest_path.to_str().unwrap().to_string()
}

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
