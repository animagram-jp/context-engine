// Integration tests for common::state::State (new fixed-length record implementation)
use state_engine::common::state::State;
use state_engine::load::Load;
use state_engine::{InMemoryClient, KVSClient, EnvClient};
use serde_json::{json, Value};
use crate::mocks::{MockInMemory, MockKVS, MockEnvClient};

fn fixtures_path() -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/manifest")
        .to_str()
        .unwrap()
        .to_string()
}

// ============================================================================
// Basic CRUD — KVS
// ============================================================================

#[test]
fn test_common_state_set_and_get_kvs() {
    let path = fixtures_path();
    let mut in_memory = MockInMemory::new();
    let mut kvs = MockKVS::new();

    // placeholder resolution for cache.user key: "user:${session.sso_user_id}"
    in_memory.set("request-attributes-user-key", json!(123));

    let mut state = State::new(&path, Load::new())
        .with_in_memory(&mut in_memory)
        .with_kvs_client(&mut kvs);

    let value = json!({"id": 1, "org_id": 100, "tenant_id": 10});
    assert!(state.set("cache.user", value.clone(), Some(3600)), "set should succeed");

    let retrieved = state.get("cache.user");
    assert_eq!(retrieved, Some(value));
}

#[test]
fn test_common_state_delete_kvs() {
    let path = fixtures_path();
    let mut in_memory = MockInMemory::new();
    let mut kvs = MockKVS::new();

    in_memory.set("request-attributes-user-key", json!(123));

    let mut state = State::new(&path, Load::new())
        .with_in_memory(&mut in_memory)
        .with_kvs_client(&mut kvs);

    assert!(state.set("cache.user", json!({"id": 1}), None), "set should succeed");

    assert!(state.delete("cache.user"), "delete should succeed");
    assert_eq!(state.get("cache.user"), None);
}

// ============================================================================
// Load — Env client
// ============================================================================

#[test]
fn test_common_state_load_env() {
    let path = fixtures_path();
    let mut env_client = MockEnvClient::new();
    env_client.data.insert("DB_HOST".to_string(), "localhost".to_string());
    env_client.data.insert("DB_PORT".to_string(), "3306".to_string());
    env_client.data.insert("DB_DATABASE".to_string(), "test_db".to_string());

    let mut load = Load::new();
    load = load.with_env_client(&mut env_client);

    let mut in_memory = MockInMemory::new();

    let mut state = State::new(&path, load).with_in_memory(&mut in_memory);

    let result = state.get("connection.common");
    assert!(result.is_some(), "connection.common should be loaded from Env");

    if let Some(Value::Object(obj)) = &result {
        assert_eq!(obj.get("host"), Some(&json!("localhost")));
        assert_eq!(obj.get("port"), Some(&json!("3306")));
        assert_eq!(obj.get("database"), Some(&json!("test_db")));
    } else {
        panic!("expected Object, got {:?}", result);
    }
}

// ============================================================================
// exists()
// ============================================================================

#[test]
fn test_common_state_exists_after_set() {
    let path = fixtures_path();
    let mut in_memory = MockInMemory::new();
    let mut kvs = MockKVS::new();

    in_memory.set("request-attributes-user-key", json!(123));

    let mut state = State::new(&path, Load::new())
        .with_in_memory(&mut in_memory)
        .with_kvs_client(&mut kvs);

    assert!(!state.exists("cache.user"), "should not exist before set");

    state.set("cache.user", json!({"id": 1, "org_id": 100}), None);

    assert!(state.exists("cache.user"), "should exist after set");
}

#[test]
fn test_common_state_exists_does_not_trigger_load() {
    let path = fixtures_path();
    let mut in_memory = MockInMemory::new();
    let mut kvs = MockKVS::new();

    in_memory.set("request-attributes-user-key", json!(123));

    let mut state = State::new(&path, Load::new())
        .with_in_memory(&mut in_memory)
        .with_kvs_client(&mut kvs);

    // cache.user has _load defined but nothing in store
    assert!(!state.exists("cache.user"), "exists() must not trigger _load");
}

#[test]
fn test_common_state_exists_in_store() {
    let path = fixtures_path();
    let mut in_memory = MockInMemory::new();

    in_memory.set("connection.common", json!({"host": "localhost"}));

    let mut state = State::new(&path, Load::new()).with_in_memory(&mut in_memory);

    assert!(state.exists("connection.common"));
}

// ============================================================================
// State-client load (recursive get)
// ============================================================================

#[test]
fn test_common_state_store_priority_over_load() {
    // When a value is already in store, _load should not be called
    let path = fixtures_path();
    let mut in_memory = MockInMemory::new();
    let mut kvs = MockKVS::new();

    in_memory.set("request-attributes-user-key", json!(123));

    // Pre-populate KVS with a full cache.user record
    kvs.set("user:123", r#"{"id":1,"org_id":100,"tenant_id":10}"#.to_string(), None);

    let mut state = State::new(&path, Load::new())
        .with_in_memory(&mut in_memory)
        .with_kvs_client(&mut kvs);

    let result = state.get("cache.user");
    assert!(result.is_some());
    if let Some(Value::Object(obj)) = &result {
        assert_eq!(obj.get("id"), Some(&json!(1)));
        assert_eq!(obj.get("org_id"), Some(&json!(100)));
        assert_eq!(obj.get("tenant_id"), Some(&json!(10)));
    }
}
