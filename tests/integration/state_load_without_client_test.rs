// State load without client test - _load.client が無い場合の挙動テスト
use state_engine::{Manifest, State, Load};
use state_engine::ports::provided::State as StateTrait;
use state_engine::ports::required::{InMemoryClient, KVSClient};
use serde_json::json;
use crate::mocks::{MockInMemory, MockKVS};

fn get_fixtures_path() -> String {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/manifest");
    manifest_path.to_str().unwrap().to_string()
}

#[test]
fn test_load_without_client_key_reference() {
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
