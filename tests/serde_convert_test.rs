#[test]
fn test_serde_json_to_value() {
    use serde_json::Value as JsonValue;
    use serde_yaml_ng::Value as YamlValue;

    let yaml_str = r#"
name: test
value: 123
nested:
  key: value
"#;
    
    let yaml: YamlValue = serde_yaml_ng::from_str(yaml_str).unwrap();
    
    // serde_json::to_value() を試す
    let result = serde_json::to_value(&yaml);
    
    assert!(result.is_ok(), "to_value should work if YamlValue implements Serialize");
    
    if let Ok(json) = result {
        println!("JSON: {:?}", json);
        assert!(json.is_object());
    }
}
