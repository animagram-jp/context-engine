// Common module integration tests
use state_engine::{DotMapAccessor, DotString};
use serde_json::json;

#[test]
fn test_dot_accessor_simple_key() {
    let data = json!({
        "name": "Test User"
    });

    let mut accessor = DotMapAccessor::new();
    let key = DotString::new("name");
    let result = accessor.get(&data, &key);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), &json!("Test User"));
}

#[test]
fn test_dot_accessor_nested_key() {
    let data = json!({
        "user": {
            "profile": {
                "name": "Test User",
                "age": 30
            }
        }
    });

    let mut accessor = DotMapAccessor::new();
    let key = DotString::new("user.profile.name");
    let result = accessor.get(&data, &key);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), &json!("Test User"));

    let key_age = DotString::new("user.profile.age");
    let result_age = accessor.get(&data, &key_age);
    assert!(result_age.is_some());
    assert_eq!(result_age.unwrap(), &json!(30));
}

#[test]
fn test_dot_accessor_missing_key() {
    let data = json!({
        "user": {
            "profile": {
                "name": "Test User"
            }
        }
    });

    let mut accessor = DotMapAccessor::new();
    let key = DotString::new("user.profile.missing");
    let result = accessor.get(&data, &key);
    assert!(result.is_none());
}

#[test]
fn test_dot_accessor_array_access() {
    let data = json!({
        "items": [
            {"id": 1, "name": "Item 1"},
            {"id": 2, "name": "Item 2"}
        ]
    });

    let mut accessor = DotMapAccessor::new();
    let key = DotString::new("items");
    let result = accessor.get(&data, &key);
    assert!(result.is_some());

    if let Some(json_value) = result {
        if let serde_json::Value::Array(arr) = json_value {
            assert_eq!(arr.len(), 2);
        } else {
            panic!("Expected array");
        }
    }
}

#[test]
fn test_dot_accessor_empty_path() {
    let data = json!({});
    let mut accessor = DotMapAccessor::new();
    let key = DotString::new("");
    let result = accessor.get(&data, &key);
    assert!(result.is_none());
}

#[test]
fn test_dot_accessor_deep_nesting() {
    let data = json!({
        "level1": {
            "level2": {
                "level3": {
                    "level4": {
                        "value": "deep value"
                    }
                }
            }
        }
    });

    let mut accessor = DotMapAccessor::new();
    let key = DotString::new("level1.level2.level3.level4.value");
    let result = accessor.get(&data, &key);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), &json!("deep value"));
}
