extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;

use super::pool::DynamicPool;
use super::fixed_bits;
use super::codec;

/// Re-export the public Value type for use in parsing.
pub use crate::ports::provided::Value;

/// Thin record for a single loaded manifest file.
/// Stores only the key_idx of the file root record in the shared keys vec.
pub struct ParsedManifest {
    pub file_key_idx: u16,
}

/// Parses a manifest value tree, appending into caller-owned vecs.
/// Returns a `ParsedManifest` referencing the file root record's index.
///
/// - `keys`: Vec<u64> — fixed-bits key records
/// - `values`: Vec<[u64; 2]> — fixed-bits value records
/// - `path_map`: Vec<Vec<u16>> — path segment index sequences
/// - `children_map`: Vec<Vec<u16>> — multi-child index lists
///
/// Index 0 of each vec is reserved as null by the caller.
pub fn parse(
    filename: &str,
    root: Value,
    dynamic: &mut DynamicPool,
    keys: &mut Vec<u64>,
    values: &mut Vec<[u64; 2]>,
    path_map: &mut Vec<Vec<u16>>,
    children_map: &mut Vec<Vec<u16>>,
) -> Result<ParsedManifest, String> {
    let Value::Mapping(mapping) = root else {
        return Err("DSL root must be a mapping".to_string());
    };


    // filename root record (placeholder, child index filled below)
    let dyn_idx = dynamic.intern(filename.as_bytes());
    let mut file_record = fixed_bits::new();
    file_record = fixed_bits::set(file_record, fixed_bits::K_OFFSET_DYNAMIC, fixed_bits::K_MASK_DYNAMIC, dyn_idx as u64);
    let file_idx = keys.len() as u16;
    keys.push(file_record);

    // traverse top-level keys
    let mut child_indices: Vec<u16> = Vec::new();
    for (key_bytes, value) in &mapping {
        let child_idx = traverse_field_key(key_bytes, value, filename, &[], dynamic, keys, values, path_map, children_map)?;
        child_indices.push(child_idx);
    }

    // update file record with children
    let file_record = keys[file_idx as usize];
    let file_record = match child_indices.len() {
        0 => file_record,
        1 => fixed_bits::set(file_record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, child_indices[0] as u64),
        _ => {
            let children_idx = children_map.len() as u16;
            children_map.push(child_indices);
            let r = fixed_bits::set(file_record, fixed_bits::K_OFFSET_HAS_CHILDREN, fixed_bits::K_MASK_HAS_CHILDREN, 1);
            fixed_bits::set(r, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, children_idx as u64)
        }
    };
    keys[file_idx as usize] = file_record;

    Ok(ParsedManifest { file_key_idx: file_idx })
}

/// Traverses a field key node (non-meta key).
/// `ancestors` excludes filename — only field key path segments (for qualify).
fn traverse_field_key(
    key_bytes: &[u8],
    value: &Value,
    filename: &str,
    ancestors: &[&[u8]],
    dynamic: &mut DynamicPool,
    keys: &mut Vec<u64>,
    values: &mut Vec<[u64; 2]>,
    path_map: &mut Vec<Vec<u16>>,
    children_map: &mut Vec<Vec<u16>>,
) -> Result<u16, String> {
    let dyn_idx = dynamic.intern(key_bytes);
    let mut record = fixed_bits::new();
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT, fixed_bits::ROOT_NULL);
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_DYNAMIC, fixed_bits::K_MASK_DYNAMIC, dyn_idx as u64);

    let key_idx = keys.len() as u16;
    keys.push(record);

    let mut current: Vec<&[u8]> = ancestors.to_vec();
    current.push(key_bytes);

    if let Value::Mapping(mapping) = value {
        let mut child_indices: Vec<u16> = Vec::new();
        let mut meta_indices: Vec<u16> = Vec::new();

        for (k_bytes, v) in mapping {
            if k_bytes.first() == Some(&b'_') {
                let meta_idx = traverse_meta_key(k_bytes, v, filename, ancestors, dynamic, keys, values, path_map, children_map)?;
                meta_indices.push(meta_idx);
            } else {
                let child_idx = traverse_field_key(k_bytes, v, filename, &current, dynamic, keys, values, path_map, children_map)?;
                child_indices.push(child_idx);
            }
        }

        let all_children: Vec<u16> = child_indices.iter()
            .chain(meta_indices.iter())
            .copied()
            .collect();

        let record = keys[key_idx as usize];
        let record = match all_children.len() {
            0 => record,
            1 => fixed_bits::set(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, all_children[0] as u64),
            _ => {
                let children_idx = children_map.len() as u16;
                children_map.push(all_children);
                let r = fixed_bits::set(record, fixed_bits::K_OFFSET_HAS_CHILDREN, fixed_bits::K_MASK_HAS_CHILDREN, 1);
                fixed_bits::set(r, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, children_idx as u64)
            }
        };
        keys[key_idx as usize] = record;
    } else {
        // scalar value → is_leaf
        let val_idx = build_yaml_value(value, filename, ancestors, dynamic, values, path_map)?;
        let record = keys[key_idx as usize];
        let record = fixed_bits::set(record, fixed_bits::K_OFFSET_IS_LEAF, fixed_bits::K_MASK_IS_LEAF, 1);
        let record = fixed_bits::set(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, val_idx as u64);
        keys[key_idx as usize] = record;
    }

    Ok(key_idx)
}

/// Traverses a meta key node (_load, _store, _state).
fn traverse_meta_key(
    key_bytes: &[u8],
    value: &Value,
    filename: &str,
    ancestors: &[&[u8]],
    dynamic: &mut DynamicPool,
    keys: &mut Vec<u64>,
    values: &mut Vec<[u64; 2]>,
    path_map: &mut Vec<Vec<u16>>,
    children_map: &mut Vec<Vec<u16>>,
) -> Result<u16, String> {
    let root_val = codec::root_encode(key_bytes);

    let mut record = fixed_bits::new();
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT, root_val);

    let key_idx = keys.len() as u16;
    keys.push(record);

    if let Value::Mapping(mapping) = value {
        let mut child_indices: Vec<u16> = Vec::new();

        for (k_bytes, v) in mapping {
            let child_idx = traverse_prop_key(k_bytes, v, filename, ancestors, dynamic, keys, values, path_map, children_map)?;
            child_indices.push(child_idx);
        }

        let record = keys[key_idx as usize];
        let record = match child_indices.len() {
            0 => record,
            1 => fixed_bits::set(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, child_indices[0] as u64),
            _ => {
                let children_idx = children_map.len() as u16;
                children_map.push(child_indices);
                let r = fixed_bits::set(record, fixed_bits::K_OFFSET_HAS_CHILDREN, fixed_bits::K_MASK_HAS_CHILDREN, 1);
                fixed_bits::set(r, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, children_idx as u64)
            }
        };
        keys[key_idx as usize] = record;
    }

    Ok(key_idx)
}

/// Traverses a prop key node (client, key, ttl, table, connection, where, map, type).
fn traverse_prop_key(
    key_bytes: &[u8],
    value: &Value,
    filename: &str,
    ancestors: &[&[u8]],
    dynamic: &mut DynamicPool,
    keys: &mut Vec<u64>,
    values: &mut Vec<[u64; 2]>,
    path_map: &mut Vec<Vec<u16>>,
    children_map: &mut Vec<Vec<u16>>,
) -> Result<u16, String> {
    let (prop_val, client_val) = if key_bytes == b"client" {
        (fixed_bits::PROP_NULL, codec::client_encode(
            match value { Value::Scalar(s) => s.as_slice(), _ => b"" }
        ))
    } else {
        (codec::prop_encode(key_bytes), fixed_bits::CLIENT_NULL)
    };

    let mut record = fixed_bits::new();
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_PROP, fixed_bits::K_MASK_PROP, prop_val);
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_CLIENT, fixed_bits::K_MASK_CLIENT, client_val);

    if key_bytes == b"type" {
        let type_val = codec::type_encode(
            match value { Value::Scalar(s) => s.as_slice(), _ => b"" }
        );
        record = fixed_bits::set(record, fixed_bits::K_OFFSET_TYPE, fixed_bits::K_MASK_TYPE, type_val);
    }

    let key_idx = keys.len() as u16;
    keys.push(record);

    if key_bytes == b"map" {
        if let Value::Mapping(mapping) = value {
            let mut child_indices: Vec<u16> = Vec::new();
            for (k_bytes, v) in mapping {
                let child_idx = traverse_map_key(k_bytes, v, filename, ancestors, dynamic, keys, values, path_map)?;
                child_indices.push(child_idx);
            }
            let record = keys[key_idx as usize];
            let record = match child_indices.len() {
                0 => record,
                1 => fixed_bits::set(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, child_indices[0] as u64),
                _ => {
                    let children_idx = children_map.len() as u16;
                    children_map.push(child_indices);
                    let r = fixed_bits::set(record, fixed_bits::K_OFFSET_HAS_CHILDREN, fixed_bits::K_MASK_HAS_CHILDREN, 1);
                    fixed_bits::set(r, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, children_idx as u64)
                }
            };
            keys[key_idx as usize] = record;
        }
    } else if key_bytes != b"client" {
        let val_idx = build_yaml_value(value, filename, ancestors, dynamic, values, path_map)?;
        let record = keys[key_idx as usize];
        let record = fixed_bits::set(record, fixed_bits::K_OFFSET_IS_LEAF, fixed_bits::K_MASK_IS_LEAF, 1);
        let record = fixed_bits::set(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, val_idx as u64);
        keys[key_idx as usize] = record;
    }

    Ok(key_idx)
}

/// Traverses a map child key (is_path=true).
fn traverse_map_key(
    key_bytes: &[u8],
    value: &Value,
    filename: &str,
    ancestors: &[&[u8]],
    dynamic: &mut DynamicPool,
    keys: &mut Vec<u64>,
    values: &mut Vec<[u64; 2]>,
    path_map: &mut Vec<Vec<u16>>,
) -> Result<u16, String> {
    let qualified = build_qualified_path(filename, ancestors, key_bytes);
    let seg_indices: Vec<u16> = qualified.split(|&b| b == b'.')
        .map(|seg| dynamic.intern(seg))
        .collect();
    let path_idx = path_map.len() as u16;
    path_map.push(seg_indices);

    let mut record = fixed_bits::new();
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_IS_PATH, fixed_bits::K_MASK_IS_PATH, 1);
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_DYNAMIC, fixed_bits::K_MASK_DYNAMIC, path_idx as u64);

    let val_idx = build_yaml_value(value, filename, ancestors, dynamic, values, path_map)?;
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_IS_LEAF, fixed_bits::K_MASK_IS_LEAF, 1);
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, val_idx as u64);

    let key_idx = keys.len() as u16;
    keys.push(record);
    Ok(key_idx)
}

/// Builds a YAML value record ([u64; 2]) from a scalar or template string.
fn build_yaml_value(
    value: &Value,
    filename: &str,
    ancestors: &[&[u8]],
    dynamic: &mut DynamicPool,
    values: &mut Vec<[u64; 2]>,
    path_map: &mut Vec<Vec<u16>>,
) -> Result<u16, String> {
    let s = match value {
        Value::Scalar(s)   => s.clone(),
        Value::Null        => return Ok(0),
        Value::Mapping(_)  => return Err("unexpected mapping as scalar value".to_string()),
        Value::Sequence(_) => return Err("unexpected sequence as scalar value".to_string()),
    };

    let tokens = split_template(&s);
    if tokens.len() > 6 {
        return Err(format!("value has {} tokens, max 6", tokens.len()));
    }
    let is_template = tokens.len() > 1;

    let mut vo = [0u64; 2];

    if is_template {
        vo[0] = fixed_bits::set(vo[0], fixed_bits::V_OFFSET_IS_TEMPLATE, fixed_bits::V_MASK_IS_TEMPLATE, 1);
    }

    const TOKEN_OFFSETS: [(u32, u32); 6] = [
        (fixed_bits::V_OFFSET_T0_IS_PATH, fixed_bits::V_OFFSET_T0_DYNAMIC),
        (fixed_bits::V_OFFSET_T1_IS_PATH, fixed_bits::V_OFFSET_T1_DYNAMIC),
        (fixed_bits::V_OFFSET_T2_IS_PATH, fixed_bits::V_OFFSET_T2_DYNAMIC),
        (fixed_bits::V_OFFSET_T3_IS_PATH, fixed_bits::V_OFFSET_T3_DYNAMIC),
        (fixed_bits::V_OFFSET_T4_IS_PATH, fixed_bits::V_OFFSET_T4_DYNAMIC),
        (fixed_bits::V_OFFSET_T5_IS_PATH, fixed_bits::V_OFFSET_T5_DYNAMIC),
    ];

    for (i, token) in tokens.iter().enumerate().take(6) {
        let dyn_idx = if token.is_path {
            let qualified = qualify_path(&token.text, filename, ancestors);
            let seg_indices: Vec<u16> = qualified.split(|&b| b == b'.')
                .map(|seg| dynamic.intern(seg))
                .collect();
            let path_idx = path_map.len() as u16;
            path_map.push(seg_indices);
            path_idx
        } else {
            dynamic.intern(&token.text)
        };

        let word = if i < 3 { 0 } else { 1 };
        let (off_is_path, off_dynamic) = TOKEN_OFFSETS[i];
        vo[word] = fixed_bits::set(vo[word], off_is_path, fixed_bits::V_MASK_IS_PATH, token.is_path as u64);
        vo[word] = fixed_bits::set(vo[word], off_dynamic, fixed_bits::V_MASK_DYNAMIC, dyn_idx as u64);
    }

    let val_idx = values.len() as u16;
    values.push(vo);
    Ok(val_idx)
}


/// A single template token: either a literal byte sequence or a path placeholder.
struct Token {
    text: Vec<u8>,
    is_path: bool,
}

/// Splits a byte slice by `${}` placeholders into tokens.
/// `b"user:${session.id}"` → [Token(b"user:", false), Token(b"session.id", true)]
fn split_template(s: &[u8]) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut rest = s;

    loop {
        if let Some(start) = find_bytes(rest, b"${") {
            if start > 0 {
                tokens.push(Token { text: rest[..start].to_vec(), is_path: false });
            }
            rest = &rest[start + 2..];
            if let Some(end) = rest.iter().position(|&b| b == b'}') {
                tokens.push(Token { text: rest[..end].to_vec(), is_path: true });
                rest = &rest[end + 1..];
            } else {
                tokens.push(Token { text: rest.to_vec(), is_path: false });
                break;
            }
        } else {
            if !rest.is_empty() {
                tokens.push(Token { text: rest.to_vec(), is_path: false });
            }
            break;
        }
    }

    if tokens.is_empty() {
        tokens.push(Token { text: s.to_vec(), is_path: false });
    }

    tokens
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Qualifies a placeholder path to an absolute path.
fn qualify_path(path: &[u8], filename: &str, ancestors: &[&[u8]]) -> Vec<u8> {
    if path.contains(&b'.') {
        return path.to_vec();
    }
    let mut result = filename.as_bytes().to_vec();
    for ancestor in ancestors {
        result.push(b'.');
        result.extend_from_slice(ancestor);
    }
    result.push(b'.');
    result.extend_from_slice(path);
    result
}

/// Builds a qualified path for map keys: `filename.ancestors.key`
fn build_qualified_path(filename: &str, ancestors: &[&[u8]], key: &[u8]) -> Vec<u8> {
    let mut result = filename.as_bytes().to_vec();
    for ancestor in ancestors {
        result.push(b'.');
        result.extend_from_slice(ancestor);
    }
    result.push(b'.');
    result.extend_from_slice(key);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fixed_bits;
    use alloc::vec::Vec;
    #[allow(unused_imports)]
    use alloc::vec;

    fn make_vecs() -> (DynamicPool, Vec<u64>, Vec<[u64; 2]>, Vec<Vec<u16>>, Vec<Vec<u16>>) {
        (DynamicPool::new(), vec![0], vec![[0, 0]], vec![vec![]], vec![vec![]])
    }

    fn s(v: &str) -> Value { Value::Scalar(v.as_bytes().to_vec()) }
    fn m(pairs: Vec<(&str, Value)>) -> Value {
        Value::Mapping(pairs.into_iter().map(|(k, v)| (k.as_bytes().to_vec(), v)).collect())
    }

    // --- split_template ---

    #[test]
    fn test_split_template_static() {
        let tokens = split_template(b"literal");
        assert_eq!(tokens.len(), 1);
        assert!(!tokens[0].is_path);
        assert_eq!(tokens[0].text, b"literal");
    }

    #[test]
    fn test_split_template_path_only() {
        let tokens = split_template(b"${connection.tenant}");
        assert_eq!(tokens.len(), 1);
        assert!(tokens[0].is_path);
        assert_eq!(tokens[0].text, b"connection.tenant");
    }

    #[test]
    fn test_split_template_mixed() {
        let tokens = split_template(b"user:${session.id}");
        assert_eq!(tokens.len(), 2);
        assert!(!tokens[0].is_path);
        assert_eq!(tokens[0].text, b"user:");
        assert!(tokens[1].is_path);
        assert_eq!(tokens[1].text, b"session.id");
    }

    // --- qualify_path ---

    #[test]
    fn test_qualify_path_absolute() {
        assert_eq!(qualify_path(b"connection.common", "cache", &[b"user".as_slice()]), b"connection.common");
    }

    #[test]
    fn test_qualify_path_relative() {
        assert_eq!(qualify_path(b"org_id", "cache", &[b"user".as_slice()]), b"cache.user.org_id");
    }

    #[test]
    fn test_qualify_path_relative_no_ancestors() {
        assert_eq!(qualify_path(b"org_id", "cache", &[]), b"cache.org_id");
    }

    // --- parse: field key → ROOT_NULL ---

    #[test]
    fn test_field_key_root_is_null() {
        let (mut dynamic, mut keys, mut values, mut path_map, mut children_map) = make_vecs();
        let root = m(vec![("foo", m(vec![]))]);
        let pm = parse("f", root, &mut dynamic, &mut keys, &mut values, &mut path_map, &mut children_map).unwrap();

        let file_rec = keys[pm.file_key_idx as usize];
        let child_idx = fixed_bits::get(file_rec, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD) as usize;
        assert_eq!(fixed_bits::get(keys[child_idx], fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT), fixed_bits::ROOT_NULL);
    }

    // --- parse: meta key → ROOT bits ---

    #[test]
    fn test_meta_key_root_bits() {
        let (mut dynamic, mut keys, mut values, mut path_map, mut children_map) = make_vecs();
        let root = m(vec![("foo", m(vec![
            ("_state", m(vec![("type", s("integer"))])),
            ("_load",  m(vec![("client", s("InMemory")), ("key", s("k"))])),
            ("_store", m(vec![("client", s("InMemory")), ("key", s("k"))])),
        ]))]);
        parse("f", root, &mut dynamic, &mut keys, &mut values, &mut path_map, &mut children_map).unwrap();

        let roots: Vec<u64> = keys.iter().map(|&r| fixed_bits::get(r, fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT)).collect();
        assert!(roots.contains(&fixed_bits::ROOT_STATE));
        assert!(roots.contains(&fixed_bits::ROOT_LOAD));
        assert!(roots.contains(&fixed_bits::ROOT_STORE));
    }

    // --- parse: type encoding ---

    #[test]
    fn test_type_encoding() {
        let (mut dynamic, mut keys, mut values, mut path_map, mut children_map) = make_vecs();
        let root = m(vec![("foo", m(vec![
            ("_state", m(vec![("type", s("integer"))])),
        ]))]);
        parse("f", root, &mut dynamic, &mut keys, &mut values, &mut path_map, &mut children_map).unwrap();

        let types: Vec<u64> = keys.iter().map(|&r| fixed_bits::get(r, fixed_bits::K_OFFSET_TYPE, fixed_bits::K_MASK_TYPE)).collect();
        assert!(types.contains(&fixed_bits::TYPE_I64));
    }

    // --- parse: client encoding ---

    #[test]
    fn test_client_encoding() {
        let (mut dynamic, mut keys, mut values, mut path_map, mut children_map) = make_vecs();
        let root = m(vec![("foo", m(vec![
            ("_store", m(vec![("client", s("KVS")), ("key", s("k")), ("ttl", s("3600"))])),
        ]))]);
        parse("f", root, &mut dynamic, &mut keys, &mut values, &mut path_map, &mut children_map).unwrap();

        let clients: Vec<u64> = keys.iter().map(|&r| fixed_bits::get(r, fixed_bits::K_OFFSET_CLIENT, fixed_bits::K_MASK_CLIENT)).collect();
        assert!(clients.contains(&fixed_bits::CLIENT_KVS));
    }

    // --- parse: template value → is_template flag + path_map ---

    #[test]
    fn test_template_value() {
        let (mut dynamic, mut keys, mut values, mut path_map, mut children_map) = make_vecs();
        let root = m(vec![("foo", m(vec![
            ("_store", m(vec![("client", s("KVS")), ("key", s("foo:${session.id}"))])),
        ]))]);
        parse("f", root, &mut dynamic, &mut keys, &mut values, &mut path_map, &mut children_map).unwrap();

        let has_template = values.iter().any(|&vo| fixed_bits::get(vo[0], fixed_bits::V_OFFSET_IS_TEMPLATE, fixed_bits::V_MASK_IS_TEMPLATE) == 1);
        assert!(has_template);
        assert!(path_map.len() > 1);
    }

    // --- parse: map key → path_map expansion ---

    #[test]
    fn test_map_key_path_expansion() {
        let (mut dynamic, mut keys, mut values, mut path_map, mut children_map) = make_vecs();
        let root = m(vec![("foo", m(vec![
            ("_load", m(vec![
                ("client", s("Env")),
                ("map", m(vec![("host", s("DB_HOST")), ("port", s("DB_PORT"))])),
            ])),
        ]))]);
        parse("f", root, &mut dynamic, &mut keys, &mut values, &mut path_map, &mut children_map).unwrap();

        // map keys produce is_path=1 records
        let has_path = keys.iter().any(|&r| fixed_bits::get(r, fixed_bits::K_OFFSET_IS_PATH, fixed_bits::K_MASK_IS_PATH) == 1);
        assert!(has_path);
    }

    // --- parse: two files → globally unique key indices ---

    #[test]
    fn test_two_files_unique_indices() {
        let (mut dynamic, mut keys, mut values, mut path_map, mut children_map) = make_vecs();
        let a = m(vec![("x", m(vec![]))]);
        let b = m(vec![("y", m(vec![]))]);
        let pm_a = parse("a", a, &mut dynamic, &mut keys, &mut values, &mut path_map, &mut children_map).unwrap();
        let pm_b = parse("b", b, &mut dynamic, &mut keys, &mut values, &mut path_map, &mut children_map).unwrap();

        assert_ne!(pm_a.file_key_idx, pm_b.file_key_idx);

        let dyn_a = fixed_bits::get(keys[pm_a.file_key_idx as usize], fixed_bits::K_OFFSET_DYNAMIC, fixed_bits::K_MASK_DYNAMIC) as u16;
        let dyn_b = fixed_bits::get(keys[pm_b.file_key_idx as usize], fixed_bits::K_OFFSET_DYNAMIC, fixed_bits::K_MASK_DYNAMIC) as u16;
        assert_eq!(dynamic.get(dyn_a), Some(b"a".as_slice()));
        assert_eq!(dynamic.get(dyn_b), Some(b"b".as_slice()));
    }

    // --- parse: root must be Mapping ---

    #[test]
    fn test_root_must_be_mapping() {
        let (mut dynamic, mut keys, mut values, mut path_map, mut children_map) = make_vecs();
        assert!(parse("f", s("bad"), &mut dynamic, &mut keys, &mut values, &mut path_map, &mut children_map).is_err());
    }
}
