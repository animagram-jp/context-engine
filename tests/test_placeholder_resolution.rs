// Test actual placeholder resolution in State
use state_engine::{Manifest, State, Load};
use state_engine::ports::provided::State as StateTrait;
use serde_json::json;
use std::collections::HashMap;
use std::fs;

// Mock clients
struct MockInMemory {
    data: HashMap<String, serde_json::Value>,
}

impl MockInMemory {
    fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    fn set(&mut self, key: &str, value: serde_json::Value) {
        self.data.insert(key.to_string(), value);
    }
}

impl state_engine::ports::required::InMemoryClient for MockInMemory {
    fn get(&self, key: &str) -> Option<serde_json::Value> {
        self.data.get(key).cloned()
    }

    fn set(&mut self, key: &str, value: serde_json::Value) {
        self.data.insert(key.to_string(), value);
    }

    fn delete(&mut self, key: &str) -> bool {
        self.data.remove(key).is_some()
    }
}

#[test]
fn test_placeholder_resolution_with_state_client() {
    // manifestディレクトリを作成（/tmp直下に作成）
    let temp_path = format!("/tmp/test_placeholder_res_{}", std::process::id());
    let manifest_dir = std::path::Path::new(&temp_path);
    fs::create_dir_all(&manifest_dir).unwrap();

    // cache.yml を作成
    let cache_yml = r#"
user:
  org_id:
    _state:
      type: integer
    _store:
      client: InMemory
      key: "cache-user-org_id"
  tenant_id:
    _state:
      type: integer
    _load:
      client: State
      key: ${org_id}
"#;
    fs::write(manifest_dir.join("cache.yml"), cache_yml).unwrap();

    let mut manifest = Manifest::new(manifest_dir.to_str().unwrap());
    let load = Load::new();
    let mut in_memory = MockInMemory::new();

    // cache.user.org_id に値をセット
    in_memory.set("cache-user-org_id", json!(100));

    let mut state = State::new(&mut manifest, load);
    state = state.with_in_memory(&mut in_memory);

    // cache.user.org_id を取得して、キャッシュに入れる
    let org_id = state.get("cache.user.org_id");
    assert_eq!(org_id, Some(json!(100)));

    // cache.user.tenant_id を取得
    // YAMLでは key: ${org_id} だが、Manifestが ${cache.user.org_id} に変換
    // resolve_placeholder が ${cache.user.org_id} を 100 に解決
    let tenant_id = state.get("cache.user.tenant_id");
    println!("tenant_id: {:?}", tenant_id);

    // tenant_idは org_id の値（100）が返されるはず
    assert_eq!(tenant_id, Some(json!(100)));

    // クリーンアップ
    fs::remove_dir_all(&manifest_dir).ok();
}
