// PHP extension binding for state-engine.
//
// Reads manifest YAML files via std::fs (same as crate/).
// PHP's open_basedir is enforced at the Zend engine level regardless,
// so plain std::fs is sufficient here.
// Memory: uses Rust's standard allocator (malloc/free), not PHP's emalloc.
//
// TODO: add ext-php-rs dep and expose Manifest/State as PHP classes.

use std::fs;
use core::parser::{Value, parse, ParsedManifest};
use core::pool::DynamicPool;

/// Reads a YAML file and converts it to core::parser::Value.
pub fn load_yaml(path: &str) -> Option<Value> {
    let content = fs::read_to_string(path).ok()?;
    let v: serde_yaml_ng::Value = serde_yaml_ng::from_str(&content).ok()?;
    Some(yaml_to_value(v))
}

fn yaml_to_value(v: serde_yaml_ng::Value) -> Value {
    match v {
        serde_yaml_ng::Value::Mapping(m) => Value::Mapping(
            m.into_iter()
                .filter_map(|(k, v)| {
                    let key = match k {
                        serde_yaml_ng::Value::String(s) => s,
                        _ => return None,
                    };
                    Some((key, yaml_to_value(v)))
                })
                .collect(),
        ),
        serde_yaml_ng::Value::String(s) => Value::Scalar(s),
        serde_yaml_ng::Value::Number(n) => Value::Scalar(n.to_string()),
        serde_yaml_ng::Value::Bool(b)   => Value::Scalar(b.to_string()),
        serde_yaml_ng::Value::Null      => Value::Null,
        _                               => Value::Null,
    }
}

/// Parses all YAML files in a directory into shared pools.
/// Returns (dynamic, keys, values, path_map, children_map, parsed_manifests).
pub fn load_manifest_dir(
    dir: &str,
) -> Result<
    (DynamicPool, Vec<u64>, Vec<[u64; 2]>, Vec<Vec<u16>>, Vec<Vec<u16>>, Vec<(String, ParsedManifest)>),
    String,
> {
    let mut dynamic = DynamicPool::new();
    let mut keys: Vec<u64> = vec![0];
    let mut values: Vec<[u64; 2]> = vec![[0, 0]];
    let mut path_map: Vec<Vec<u16>> = vec![vec![]];
    let mut children_map: Vec<Vec<u16>> = vec![vec![]];
    let mut manifests: Vec<(String, ParsedManifest)> = Vec::new();

    let entries = fs::read_dir(dir)
        .map_err(|e| format!("read_dir failed: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yml") {
            continue;
        }
        let filename = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("invalid filename: {:?}", path))?
            .to_string();

        let root = load_yaml(path.to_str().unwrap())
            .ok_or_else(|| format!("failed to load: {:?}", path))?;

        let pm = parse(&filename, root, &mut dynamic, &mut keys, &mut values, &mut path_map, &mut children_map)
            .map_err(|e| format!("parse error in {}: {}", filename, e))?;

        manifests.push((filename, pm));
    }

    Ok((dynamic, keys, values, path_map, children_map, manifests))
}
