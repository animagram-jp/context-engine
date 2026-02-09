// Manifest impl
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::common::DotArrayAccessor;
use crate::ports::provided;

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
                let mut accessor = DotArrayAccessor::new();
                accessor.get(data, &path).cloned()
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
    /// _load.mapのキーは絶対パスに正規化される
    pub fn get_meta(&mut self, key: &str) -> HashMap<String, Value> {
        let (file, path) = self.parse_key(key);

        if self.load_file(&file).is_err() {
            self.missing_keys.push(key.to_string());
            return HashMap::new();
        }

        let Some(root) = self.cache.get(&file) else {
            return HashMap::new();
        };

        // ルートから指定Nodeまでのパス上のすべてのNodeを収集
        let mut nodes = vec![root.clone()];
        if !path.is_empty() {
            let mut accessor = DotArrayAccessor::new();
            let mut current = root;
            for segment in path.split('.') {
                current = match accessor.get(current, segment) {
                    Some(node) => node,
                    None => {
                        // fail fast: キーが存在しない場合は即座に失敗
                        self.missing_keys.push(key.to_string());
                        return HashMap::new();
                    }
                };
                nodes.push(current.clone());
            }
        }

        // すべてのNodeから_始まりのキーを抽出
        // 各 node の階層パスを構築しながら処理
        let mut meta: HashMap<String, Value> = HashMap::new();
        let mut load_map_owner_path: Option<String> = None; // _load.map を持つ node のパス
        let segments: Vec<&str> = if path.is_empty() {
            vec![]
        } else {
            path.split('.').collect()
        };

        for (depth, node) in nodes.iter().enumerate() {
            let Value::Object(obj) = node else {
                continue;
            };

            // この node のパスを構築
            let node_path = if depth == 0 {
                file.clone()
            } else {
                let node_segments: Vec<&str> = segments.iter().copied().take(depth).collect();
                if node_segments.is_empty() {
                    file.clone()
                } else {
                    format!("{}.{}", file, node_segments.join("."))
                }
            };

            // metadata を収集
            for (k, v) in obj {
                if k.starts_with('_') {
                    // メタブロックのマージ/上書きルール
                    // ルートのメタキー (_load, _store) → マージ
                    // 子のメタキー (client, map,...) → 上書き
                    if let Some(existing_value) = meta.get(k).cloned() {
                        // 既存のメタキーがある場合
                        if existing_value.is_object() && v.is_object() {
                            // 両方がObjectの場合はマージ（子が親を上書き）
                            if let (Value::Object(existing_obj), Value::Object(new_obj)) = (&existing_value, v) {
                                let mut merged = existing_obj.clone();
                                for (child_key, child_value) in new_obj {
                                    merged.insert(child_key.clone(), child_value.clone());
                                }
                                meta.insert(k.clone(), Value::Object(merged));
                            }
                        } else {
                            // それ以外は上書き
                            meta.insert(k.clone(), v.clone());
                        }
                    } else {
                        // 新規のメタキーは追加
                        meta.insert(k.clone(), v.clone());
                    }

                    // _load.map を持つ node のパスを記録（最も深い階層を優先）
                    // ループ中に上書きされる → 最も深い階層が残る
                    if k == "_load" {
                        if let Value::Object(load_obj) = v {
                            if load_obj.contains_key("map") {
                                load_map_owner_path = Some(node_path.clone());
                            }
                        }
                    }
                }
            }
        }

        // _load.map のキーを絶対パスに正規化
        if let (Some(Value::Object(load_obj)), Some(owner_path)) = (meta.get("_load"), &load_map_owner_path) {
            if let Some(Value::Object(map_obj)) = load_obj.get("map") {
                let mut normalized_map = serde_json::Map::new();

                // map のキーを絶対パスに変換
                for (relative_key, db_column) in map_obj {
                    let absolute_key = format!("{}.{}", owner_path, relative_key);
                    normalized_map.insert(absolute_key, db_column.clone());
                }

                // _load を更新
                let mut new_load = load_obj.clone();
                new_load.insert("map".to_string(), Value::Object(normalized_map));
                meta.insert("_load".to_string(), Value::Object(new_load));
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

        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&content)
            .map_err(|e| format!("Failed to parse YAML: {}", e))?;

        let json_value = self.yaml_to_json(&yaml);
        self.cache.insert(file.to_string(), json_value);

        Ok(())
    }

    /// serde_yaml_ng::Value を serde_json::Value に変換
    fn yaml_to_json(&self, yaml: &serde_yaml_ng::Value) -> Value {
        match yaml {
            serde_yaml_ng::Value::Null => Value::Null,
            serde_yaml_ng::Value::Bool(b) => Value::Bool(*b),
            serde_yaml_ng::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Number(serde_json::Number::from(i))
                } else if let Some(f) = n.as_f64() {
                    Value::Number(serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0)))
                } else {
                    Value::Null
                }
            }
            serde_yaml_ng::Value::String(s) => Value::String(s.clone()),
            serde_yaml_ng::Value::Sequence(seq) => {
                Value::Array(seq.iter().map(|v| self.yaml_to_json(v)).collect())
            }
            serde_yaml_ng::Value::Mapping(map) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in map {
                    if let serde_yaml_ng::Value::String(key) = k {
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

// Provided::Manifest trait の実装
impl provided::Manifest for Manifest {
    fn get(&mut self, key: &str, default: Option<Value>) -> Value {
        Manifest::get(self, key, default)
    }

    fn get_meta(&mut self, key: &str) -> HashMap<String, Value> {
        Manifest::get_meta(self, key)
    }

    fn get_missing_keys(&self) -> &[String] {
        Manifest::get_missing_keys(self)
    }

    fn clear_missing_keys(&mut self) {
        Manifest::clear_missing_keys(self)
    }
}

