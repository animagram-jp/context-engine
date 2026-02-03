// State module integration tests
use state_engine::{Manifest, State, Load};
use state_engine::ports::provided::State as StateTrait;
use state_engine::ports::required::{InMemoryClient, KVSClient};
use serde_json::{json, Value};
use std::collections::HashMap;

fn get_fixtures_path() -> String {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/manifest");
    manifest_path.to_str().unwrap().to_string()
}

// Mock InMemoryClient
struct MockInMemory {
    data: HashMap<String, Value>,
}

impl MockInMemory {
    fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl InMemoryClient for MockInMemory {
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

// Mock KVSClient
struct MockKVS {
    data: HashMap<String, Value>,
}

impl MockKVS {
    fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl KVSClient for MockKVS {
    fn get(&self, key: &str) -> Option<Value> {
        self.data.get(key).cloned()
    }

    fn set(&mut self, key: &str, value: Value, _ttl: Option<u64>) -> bool {
        self.data.insert(key.to_string(), value);
        true
    }

    fn delete(&mut self, key: &str) -> bool {
        self.data.remove(key).is_some()
    }

    fn exists(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }
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
