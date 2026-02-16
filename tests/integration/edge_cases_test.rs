// Edge case tests for serialization and type handling
use state_engine::{Manifest, State, Load};
use state_engine::ports::provided::State as StateTrait;
use serde_json::json;
use crate::mocks::{MockInMemory, MockKVS};
use state_engine::ports::required::InMemoryClient;

fn get_fixtures_path() -> String {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/manifest");
    manifest_path.to_str().unwrap().to_string()
}

#[test]
fn test_kvs_serialization_edge_case_zero() {
    // 0 (数値) が正しくシリアライズ/デシリアライズされることを確認
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);
    let load = Load::new();
    let mut in_memory = MockInMemory::new();
    let mut kvs = MockKVS::new();

    // session.sso_user_id を InMemory に設定（placeholder解決用）
    in_memory.set("request-attributes-user-key", json!(123));

    let mut state = State::new(&mut manifest, load)
        .with_in_memory(&mut in_memory)
        .with_kvs_client(&mut kvs);

    // cache.user に 0 を含むデータを保存
    let value = json!({
        "id": 0,
        "org_id": 0,
        "tenant_id": 0
    });

    let result = state.set("cache.user", value.clone(), Some(3600));
    assert!(result, "set should succeed");

    // 取得して型が保持されていることを確認
    let retrieved = state.get("cache.user");
    assert_eq!(retrieved, Some(value));

    // 各フィールドも正しく取得できることを確認
    let id = state.get("cache.user.id");
    assert_eq!(id, Some(json!(0)), "id should be 0 (number), not false or null");

    let org_id = state.get("cache.user.org_id");
    assert_eq!(org_id, Some(json!(0)), "org_id should be 0 (number)");
}

#[test]
fn test_kvs_serialization_edge_case_one() {
    // 1 (数値) が正しくシリアライズ/デシリアライズされることを確認
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);
    let load = Load::new();
    let mut in_memory = MockInMemory::new();
    let mut kvs = MockKVS::new();

    in_memory.set("request-attributes-user-key", json!(123));

    let mut state = State::new(&mut manifest, load)
        .with_in_memory(&mut in_memory)
        .with_kvs_client(&mut kvs);

    // cache.user に 1 を含むデータを保存
    let value = json!({
        "id": 1,
        "org_id": 1,
        "tenant_id": 1
    });

    let result = state.set("cache.user", value.clone(), Some(3600));
    assert!(result, "set should succeed");

    let retrieved = state.get("cache.user");
    assert_eq!(retrieved, Some(value));

    let id = state.get("cache.user.id");
    assert_eq!(id, Some(json!(1)), "id should be 1 (number), not true");
}

#[test]
fn test_kvs_serialization_boolean_vs_number() {
    // true/false と 1/0 が区別されることを確認
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);
    let load = Load::new();
    let mut in_memory = MockInMemory::new();
    let mut kvs = MockKVS::new();

    in_memory.set("request-attributes-user-key", json!(123));

    let mut state = State::new(&mut manifest, load)
        .with_in_memory(&mut in_memory)
        .with_kvs_client(&mut kvs);

    // 数値とブール値が混在するデータ
    let value = json!({
        "num_zero": 0,
        "num_one": 1,
        "bool_false": false,
        "bool_true": true,
        "null_value": null,
        "string_zero": "0",
        "string_one": "1"
    });

    state.set("cache.user", value.clone(), None);
    let retrieved = state.get("cache.user");

    assert_eq!(retrieved, Some(value));

    // 各フィールドの型が正しいことを確認
    assert_eq!(state.get("cache.user.num_zero"), Some(json!(0)));
    assert_eq!(state.get("cache.user.num_one"), Some(json!(1)));
    assert_eq!(state.get("cache.user.bool_false"), Some(json!(false)));
    assert_eq!(state.get("cache.user.bool_true"), Some(json!(true)));
    assert_eq!(state.get("cache.user.null_value"), Some(json!(null)));
    assert_eq!(state.get("cache.user.string_zero"), Some(json!("0")));
    assert_eq!(state.get("cache.user.string_one"), Some(json!("1")));

    // JSON型として区別されていることを確認
    let num_zero = state.get("cache.user.num_zero").unwrap();
    let bool_false = state.get("cache.user.bool_false").unwrap();
    assert!(num_zero.is_number());
    assert!(bool_false.is_boolean());
    assert_ne!(num_zero, bool_false, "0 and false should be different");

    let num_one = state.get("cache.user.num_one").unwrap();
    let bool_true = state.get("cache.user.bool_true").unwrap();
    assert!(num_one.is_number());
    assert!(bool_true.is_boolean());
    assert_ne!(num_one, bool_true, "1 and true should be different");
}

#[test]
fn test_kvs_serialization_empty_string() {
    // 空文字列が正しく扱われることを確認
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);
    let load = Load::new();
    let mut in_memory = MockInMemory::new();
    let mut kvs = MockKVS::new();

    in_memory.set("request-attributes-user-key", json!(123));

    let mut state = State::new(&mut manifest, load)
        .with_in_memory(&mut in_memory)
        .with_kvs_client(&mut kvs);

    let value = json!({
        "empty": "",
        "null_val": null,
        "zero": 0,
        "false_val": false
    });

    state.set("cache.user", value.clone(), None);
    let retrieved = state.get("cache.user");

    assert_eq!(retrieved, Some(value));

    // 空文字列とnull, 0, falseが区別されることを確認
    assert_eq!(state.get("cache.user.empty"), Some(json!("")));
    assert_eq!(state.get("cache.user.null_val"), Some(json!(null)));
    assert_eq!(state.get("cache.user.zero"), Some(json!(0)));
    assert_eq!(state.get("cache.user.false_val"), Some(json!(false)));
}

#[test]
fn test_kvs_serialization_array_and_object() {
    // 配列とオブジェクトが正しくシリアライズされることを確認
    let fixtures_path = get_fixtures_path();
    let mut manifest = Manifest::new(&fixtures_path);
    let load = Load::new();
    let mut in_memory = MockInMemory::new();
    let mut kvs = MockKVS::new();

    in_memory.set("request-attributes-user-key", json!(123));

    let mut state = State::new(&mut manifest, load)
        .with_in_memory(&mut in_memory)
        .with_kvs_client(&mut kvs);

    let value = json!({
        "array": [0, 1, false, true, null, "", "string"],
        "nested": {
            "inner": {
                "value": 0
            }
        }
    });

    state.set("cache.user", value.clone(), None);
    let retrieved = state.get("cache.user");

    assert_eq!(retrieved, Some(value));

    // 配列要素が正しいことを確認
    let array = state.get("cache.user.array").unwrap();
    assert!(array.is_array());
    let arr = array.as_array().unwrap();
    assert_eq!(arr[0], json!(0));
    assert_eq!(arr[1], json!(1));
    assert_eq!(arr[2], json!(false));
    assert_eq!(arr[3], json!(true));
    assert_eq!(arr[4], json!(null));
    assert_eq!(arr[5], json!(""));
    assert_eq!(arr[6], json!("string"));
}
