use serde_json::Value;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: state-engine <command> [key_path]");
        eprintln!("Commands: manifest, connection, cache, database");
        process::exit(1);
    }

    let command = &args[1];
    let key_path = if args.len() > 2 {
        Some(args[2].as_str())
    } else {
        None
    };

    let result = match command.as_str() {
        "manifest" => read_yaml("/app/manifest", key_path),
        "connection" => read_yaml("/app/manifest/connection.yml", key_path),
        "cache" => read_yaml("/app/manifest/cache.yml", key_path),
        "database" => read_yaml("/app/manifest/database.yml", key_path),
        _ => {
            eprintln!("Unknown command: {}", command);
            process::exit(1);
        }
    };

    match result {
        Ok(value) => {
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn read_yaml(path: &str, key_path: Option<&str>) -> Result<Value, String> {
    // YAMLファイルを読み込む
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file {}: {}", path, e))?;

    // YAMLをパース
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse YAML: {}", e))?;

    // serde_yaml::Value を serde_json::Value に変換
    let json_value = yaml_to_json(&yaml);

    // キーパスが指定されている場合はドット記法でアクセス
    if let Some(path) = key_path {
        get_by_path(&json_value, path)
    } else {
        Ok(json_value)
    }
}

fn yaml_to_json(yaml: &serde_yaml::Value) -> Value {
    match yaml {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(b) => Value::Bool(*b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Number(serde_json::Number::from(i))
            } else if let Some(f) = n.as_f64() {
                Value::Number(serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0)))
            } else {
                Value::Null
            }
        }
        serde_yaml::Value::String(s) => Value::String(s.clone()),
        serde_yaml::Value::Sequence(seq) => {
            Value::Array(seq.iter().map(yaml_to_json).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                if let serde_yaml::Value::String(key) = k {
                    obj.insert(key.clone(), yaml_to_json(v));
                }
            }
            Value::Object(obj)
        }
        _ => Value::Null,
    }
}

fn get_by_path(value: &Value, path: &str) -> Result<Value, String> {
    let keys: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for key in &keys {
        current = current
            .get(key)
            .ok_or_else(|| format!("Key '{}' not found in path '{}'", key, path))?;
    }

    Ok(current.clone())
}
