// Manifest - YAMLマニフェストファイル読み込み・管理
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::common::DotArrayAccessor;

pub struct Manifest {
    manifest_dir: PathBuf,
    cache: HashMap<String, Value>,
    missing_keys: Vec<String>,
}

impl Manifest {
    pub fn new(manifest_dir: &str) -> Self {
        Self {
            manifest_dir: PathBuf::from(manifest_dir),
            cache: HashMap::new(),
            missing_keys: Vec::new(),
        }
    }

    /// キーからデータを取得
    /// 形式: "filename.path.to.key"
    /// 例: "connection.common.host"
    pub fn get(&mut self, key: &str, default: Option<Value>) -> Value {
        let (file, path) = self.parse_key(key);

        if let Err(_e) = self.load_file(&file) {
            self.missing_keys.push(key.to_string());
            return default.unwrap_or(Value::Null);
        }

        let value = if path.is_empty() {
            // ファイル全体を返す
            self.cache.get(&file).cloned()
        } else {
            // ドット記法でアクセス
            self.cache.get(&file).and_then(|data| {
                if let Value::Object(map) = data {
                    let hashmap: HashMap<String, Value> = map.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    DotArrayAccessor::get(&hashmap, &path).cloned()
                } else {
                    None
                }
            })
        };

        match value {
            Some(v) => {
                // メタデータ（_始まり）をフィルタリング
                self.filter_meta(&v)
            }
            None => {
                self.missing_keys.push(key.to_string());
                default.unwrap_or(Value::Null)
            }
        }
    }

    /// メタデータを取得
    /// 指定されたキーのパス上のすべての_始まりキーを収集
    pub fn get_meta(&mut self, key: &str) -> HashMap<String, Value> {
        let (file, path) = self.parse_key(key);

        if self.load_file(&file).is_err() {
            self.missing_keys.push(key.to_string());
            return HashMap::new();
        }

        let Some(root) = self.cache.get(&file) else {
            return HashMap::new();
        };

        let mut meta = HashMap::new();

        // ルートのメタデータを収集
        if let Value::Object(obj) = root {
            for (k, v) in obj {
                if k.starts_with('_') {
                    meta.insert(k.clone(), v.clone());
                }
            }
        }

        // パスが指定されている場合、各セグメントのメタデータを収集
        if !path.is_empty() {
            let mut current = root;
            for segment in path.split('.') {
                if let Value::Object(obj) = current {
                    // セグメントのメタデータを収集
                    for (k, v) in obj {
                        if k.starts_with('_') {
                            meta.insert(k.clone(), v.clone());
                        }
                    }
                    // 次のセグメントへ移動
                    if let Some(next) = obj.get(segment) {
                        current = next;
                    } else {
                        self.missing_keys.push(key.to_string());
                        return HashMap::new();
                    }
                }
            }

            // 最終ノードのメタデータを収集
            if let Value::Object(obj) = current {
                for (k, v) in obj {
                    if k.starts_with('_') {
                        meta.insert(k.clone(), v.clone());
                    }
                }
            }
        }

        meta
    }

    /// キーを "filename" と "path.to.key" に分解
    fn parse_key(&self, key: &str) -> (String, String) {
        let parts: Vec<&str> = key.splitn(2, '.').collect();
        let file = parts[0].to_string();
        let path = if parts.len() > 1 {
            parts[1].to_string()
        } else {
            String::new()
        };
        (file, path)
    }

    /// YAMLファイルをロード
    fn load_file(&mut self, file: &str) -> Result<(), String> {
        if self.cache.contains_key(file) {
            return Ok(());
        }

        let file_path = self.manifest_dir.join(format!("{}.yml", file));

        if !file_path.exists() {
            self.cache.insert(file.to_string(), Value::Object(serde_json::Map::new()));
            return Err(format!("File not found: {:?}", file_path));
        }

        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse YAML: {}", e))?;

        let json_value = self.yaml_to_json(&yaml);
        self.cache.insert(file.to_string(), json_value);

        Ok(())
    }

    /// serde_yaml::Value を serde_json::Value に変換
    fn yaml_to_json(&self, yaml: &serde_yaml::Value) -> Value {
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
                Value::Array(seq.iter().map(|v| self.yaml_to_json(v)).collect())
            }
            serde_yaml::Value::Mapping(map) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in map {
                    if let serde_yaml::Value::String(key) = k {
                        obj.insert(key.clone(), self.yaml_to_json(v));
                    }
                }
                Value::Object(obj)
            }
            _ => Value::Null,
        }
    }

    /// メタデータ（_始まりのキー）をフィルタリング
    fn filter_meta(&self, value: &Value) -> Value {
        match value {
            Value::Object(obj) => {
                let mut filtered = serde_json::Map::new();
                for (k, v) in obj {
                    if !k.starts_with('_') {
                        filtered.insert(k.clone(), self.filter_meta(v));
                    }
                }
                Value::Object(filtered)
            }
            Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| self.filter_meta(v)).collect())
            }
            _ => value.clone(),
        }
    }

    pub fn get_missing_keys(&self) -> &[String] {
        &self.missing_keys
    }

    pub fn clear_missing_keys(&mut self) {
        self.missing_keys.clear();
    }
}
