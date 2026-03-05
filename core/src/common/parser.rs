use serde_yaml_ng::Value;
use crate::common::pool::{DynamicPool, PathMap, ChildrenMap, KeyList, YamlValueList};
use crate::common::fixed_bits;

/// Thin record for a single loaded manifest file.
/// Stores only the key_idx of the file root record in the shared KeyList.
pub struct ParsedManifest {
    pub file_key_idx: u16,
}

/// Parses a YAML manifest string, appending into shared pool structures.
/// Returns a `ParsedManifest` referencing the file root record's index.
///
/// # Examples
///
/// ```
/// use state_engine_core::common::parser::parse;
/// use state_engine_core::common::pool::{DynamicPool, PathMap, ChildrenMap, KeyList, YamlValueList};
/// use state_engine_core::common::fixed_bits;
///
/// let yaml = "
/// user:
///   _store:
///     client: KVS
///     key: 'user:${session.sso_user_id}'
///     ttl: 14400
///   id:
///     _state:
///       type: integer
/// ";
///
/// let mut dynamic = DynamicPool::new();
/// let mut path_map = PathMap::new();
/// let mut children_map = ChildrenMap::new();
/// let mut keys = KeyList::new();
/// let mut values = YamlValueList::new();
///
/// let pm = parse("cache", yaml, &mut dynamic, &mut path_map, &mut children_map, &mut keys, &mut values).unwrap();
///
/// // file root record is at pm.file_key_idx
/// let root = keys.get(pm.file_key_idx).unwrap();
/// let dyn_idx = fixed_bits::get(root, fixed_bits::K_OFFSET_DYNAMIC, fixed_bits::K_MASK_DYNAMIC) as u16;
/// assert_eq!(dynamic.get(dyn_idx), Some("cache"));
/// ```
pub fn parse(
    filename: &str,
    yaml: &str,
    dynamic: &mut DynamicPool,
    path_map: &mut PathMap,
    children_map: &mut ChildrenMap,
    keys: &mut KeyList,
    values: &mut YamlValueList,
) -> Result<ParsedManifest, String> {
    let root: Value = serde_yaml_ng::from_str(yaml)
        .map_err(|e| format!("YAML parse error: {}", e))?;

    let Value::Mapping(mapping) = root else {
        return Err("YAML root must be a mapping".to_string());
    };

    // filename root record (placeholder, child index filled below)
    let dyn_idx = dynamic.intern(filename);
    let mut file_record = fixed_bits::new();
    file_record = fixed_bits::set(file_record, fixed_bits::K_OFFSET_DYNAMIC, fixed_bits::K_MASK_DYNAMIC, dyn_idx as u64);
    let file_idx = keys.push(file_record);

    // traverse top-level keys
    let mut child_indices: Vec<u16> = Vec::new();
    for (key, value) in &mapping {
        let key_str = yaml_str(key)?;
        let child_idx = traverse_field_key(key_str, value, filename, &[], dynamic, path_map, children_map, keys, values)?;
        child_indices.push(child_idx);
    }

    // update file record with children
    let file_record = keys.get(file_idx).unwrap();
    let file_record = match child_indices.len() {
        0 => file_record,
        1 => fixed_bits::set(file_record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, child_indices[0] as u64),
        _ => {
            let children_idx = children_map.push(child_indices);
            let r = fixed_bits::set(file_record, fixed_bits::K_OFFSET_HAS_CHILDREN, fixed_bits::K_MASK_HAS_CHILDREN, 1);
            fixed_bits::set(r, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, children_idx as u64)
        }
    };
    keys.set(file_idx, file_record);

    Ok(ParsedManifest { file_key_idx: file_idx })
}

/// Traverses a field key node (non-meta key).
/// `ancestors` excludes filename — only field key path segments (for qualify).
fn traverse_field_key(
    key_str: &str,
    value: &Value,
    filename: &str,
    ancestors: &[&str],
    dynamic: &mut DynamicPool,
    path_map: &mut PathMap,
    children_map: &mut ChildrenMap,
    keys: &mut KeyList,
    values: &mut YamlValueList,
) -> Result<u16, String> {
    let dyn_idx = dynamic.intern(key_str);
    let mut record = fixed_bits::new();
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT, fixed_bits::ROOT_NULL);
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_DYNAMIC, fixed_bits::K_MASK_DYNAMIC, dyn_idx as u64);

    let key_idx = keys.push(record);

    let mut current: Vec<&str> = ancestors.to_vec();
    current.push(key_str);

    if let Value::Mapping(mapping) = value {
        let mut child_indices: Vec<u16> = Vec::new();
        let mut meta_indices: Vec<u16> = Vec::new();

        for (k, v) in mapping {
            let k_str = yaml_str(k)?;
            if k_str.starts_with('_') {
                let meta_idx = traverse_meta_key(k_str, v, filename, &current, dynamic, path_map, children_map, keys, values)?;
                meta_indices.push(meta_idx);
            } else {
                let child_idx = traverse_field_key(k_str, v, filename, &current, dynamic, path_map, children_map, keys, values)?;
                child_indices.push(child_idx);
            }
        }

        let all_children: Vec<u16> = child_indices.iter()
            .chain(meta_indices.iter())
            .copied()
            .collect();

        let record = keys.get(key_idx).unwrap();
        let record = match all_children.len() {
            0 => record,
            1 => fixed_bits::set(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, all_children[0] as u64),
            _ => {
                let children_idx = children_map.push(all_children);
                let r = fixed_bits::set(record, fixed_bits::K_OFFSET_HAS_CHILDREN, fixed_bits::K_MASK_HAS_CHILDREN, 1);
                fixed_bits::set(r, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, children_idx as u64)
            }
        };
        keys.set(key_idx, record);
    } else {
        // scalar value → is_leaf
        let val_idx = build_yaml_value(value, filename, ancestors, dynamic, path_map, values)?;
        let record = keys.get(key_idx).unwrap();
        let record = fixed_bits::set(record, fixed_bits::K_OFFSET_IS_LEAF, fixed_bits::K_MASK_IS_LEAF, 1);
        let record = fixed_bits::set(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, val_idx as u64);
        keys.set(key_idx, record);
    }

    Ok(key_idx)
}

/// Traverses a meta key node (_load, _store, _state).
fn traverse_meta_key(
    key_str: &str,
    value: &Value,
    filename: &str,
    ancestors: &[&str],
    dynamic: &mut DynamicPool,
    path_map: &mut PathMap,
    children_map: &mut ChildrenMap,
    keys: &mut KeyList,
    values: &mut YamlValueList,
) -> Result<u16, String> {
    let root_val = match key_str {
        "_load"  => fixed_bits::ROOT_LOAD,
        "_store" => fixed_bits::ROOT_STORE,
        "_state" => fixed_bits::ROOT_STATE,
        _ => fixed_bits::ROOT_NULL,
    };

    let mut record = fixed_bits::new();
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT, root_val);

    let key_idx = keys.push(record);

    if let Value::Mapping(mapping) = value {
        let mut child_indices: Vec<u16> = Vec::new();

        for (k, v) in mapping {
            let k_str = yaml_str(k)?;
            let child_idx = traverse_prop_key(k_str, v, filename, ancestors, dynamic, path_map, children_map, keys, values)?;
            child_indices.push(child_idx);
        }

        let record = keys.get(key_idx).unwrap();
        let record = match child_indices.len() {
            0 => record,
            1 => fixed_bits::set(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, child_indices[0] as u64),
            _ => {
                let children_idx = children_map.push(child_indices);
                let r = fixed_bits::set(record, fixed_bits::K_OFFSET_HAS_CHILDREN, fixed_bits::K_MASK_HAS_CHILDREN, 1);
                fixed_bits::set(r, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, children_idx as u64)
            }
        };
        keys.set(key_idx, record);
    }

    Ok(key_idx)
}

/// Traverses a prop key node (client, key, ttl, table, connection, where, map, type).
fn traverse_prop_key(
    key_str: &str,
    value: &Value,
    filename: &str,
    ancestors: &[&str],
    dynamic: &mut DynamicPool,
    path_map: &mut PathMap,
    children_map: &mut ChildrenMap,
    keys: &mut KeyList,
    values: &mut YamlValueList,
) -> Result<u16, String> {
    let (prop_val, client_val) = match key_str {
        "client"     => (fixed_bits::PROP_NULL, parse_client(value)),
        "type"       => (fixed_bits::PROP_TYPE, fixed_bits::CLIENT_NULL),
        "key"        => (fixed_bits::PROP_KEY, fixed_bits::CLIENT_NULL),
        "connection" => (fixed_bits::PROP_CONNECTION, fixed_bits::CLIENT_NULL),
        "map"        => (fixed_bits::PROP_MAP, fixed_bits::CLIENT_NULL),
        "ttl"        => (fixed_bits::PROP_TTL, fixed_bits::CLIENT_NULL),
        "table"      => (fixed_bits::PROP_TABLE, fixed_bits::CLIENT_NULL),
        "where"      => (fixed_bits::PROP_WHERE, fixed_bits::CLIENT_NULL),
        _            => (fixed_bits::PROP_NULL, fixed_bits::CLIENT_NULL),
    };

    let mut record = fixed_bits::new();
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_PROP, fixed_bits::K_MASK_PROP, prop_val);
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_CLIENT, fixed_bits::K_MASK_CLIENT, client_val);

    if key_str == "type" {
        let type_val = parse_type(value);
        record = fixed_bits::set(record, fixed_bits::K_OFFSET_TYPE, fixed_bits::K_MASK_TYPE, type_val);
    }

    let key_idx = keys.push(record);

    if key_str == "map" {
        if let Value::Mapping(mapping) = value {
            let mut child_indices: Vec<u16> = Vec::new();
            for (k, v) in mapping {
                let k_str = yaml_str(k)?;
                let child_idx = traverse_map_key(k_str, v, filename, ancestors, dynamic, path_map, keys, values)?;
                child_indices.push(child_idx);
            }
            let record = keys.get(key_idx).unwrap();
            let record = match child_indices.len() {
                0 => record,
                1 => fixed_bits::set(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, child_indices[0] as u64),
                _ => {
                    let children_idx = children_map.push(child_indices);
                    let r = fixed_bits::set(record, fixed_bits::K_OFFSET_HAS_CHILDREN, fixed_bits::K_MASK_HAS_CHILDREN, 1);
                    fixed_bits::set(r, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, children_idx as u64)
                }
            };
            keys.set(key_idx, record);
        }
    } else if key_str != "client" {
        let val_idx = build_yaml_value(value, filename, ancestors, dynamic, path_map, values)?;
        let record = keys.get(key_idx).unwrap();
        let record = fixed_bits::set(record, fixed_bits::K_OFFSET_IS_LEAF, fixed_bits::K_MASK_IS_LEAF, 1);
        let record = fixed_bits::set(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, val_idx as u64);
        keys.set(key_idx, record);
    }

    Ok(key_idx)
}

/// Traverses a map child key (is_path=true).
fn traverse_map_key(
    key_str: &str,
    value: &Value,
    filename: &str,
    ancestors: &[&str],
    dynamic: &mut DynamicPool,
    path_map: &mut PathMap,
    keys: &mut KeyList,
    values: &mut YamlValueList,
) -> Result<u16, String> {
    let qualified = build_qualified_path(filename, ancestors, key_str);
    let seg_indices: Vec<u16> = qualified.split('.')
        .map(|seg| dynamic.intern(seg))
        .collect();
    let path_idx = path_map.push(seg_indices);

    let mut record = fixed_bits::new();
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_IS_PATH, fixed_bits::K_MASK_IS_PATH, 1);
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_DYNAMIC, fixed_bits::K_MASK_DYNAMIC, path_idx as u64);

    let val_idx = build_yaml_value(value, filename, ancestors, dynamic, path_map, values)?;
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_IS_LEAF, fixed_bits::K_MASK_IS_LEAF, 1);
    record = fixed_bits::set(record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD, val_idx as u64);

    Ok(keys.push(record))
}

/// Builds a YAML value record ([u64; 2]) from a scalar or template string.
fn build_yaml_value(
    value: &Value,
    filename: &str,
    ancestors: &[&str],
    dynamic: &mut DynamicPool,
    path_map: &mut PathMap,
    values: &mut YamlValueList,
) -> Result<u16, String> {
    let s = match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b)   => b.to_string(),
        Value::Null      => return Ok(0),
        _ => return Err(format!("unexpected value type: {:?}", value)),
    };

    let tokens = split_template(&s);
    if tokens.len() > 6 {
        return Err(format!("value '{}' has {} tokens, max 6", s, tokens.len()));
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
            let seg_indices: Vec<u16> = qualified.split('.')
                .map(|seg| dynamic.intern(seg))
                .collect();
            path_map.push(seg_indices)
        } else {
            dynamic.intern(&token.text)
        };

        let word = if i < 3 { 0 } else { 1 };
        let (off_is_path, off_dynamic) = TOKEN_OFFSETS[i];
        vo[word] = fixed_bits::set(vo[word], off_is_path, fixed_bits::V_MASK_IS_PATH, token.is_path as u64);
        vo[word] = fixed_bits::set(vo[word], off_dynamic, fixed_bits::V_MASK_DYNAMIC, dyn_idx as u64);
    }

    Ok(values.push(vo))
}

fn parse_client(value: &Value) -> u64 {
    let s = match value { Value::String(s) => s.as_str(), _ => "" };
    match s {
        "State"    => fixed_bits::CLIENT_STATE,
        "InMemory" => fixed_bits::CLIENT_IN_MEMORY,
        "Env"      => fixed_bits::CLIENT_ENV,
        "KVS"      => fixed_bits::CLIENT_KVS,
        "Db"       => fixed_bits::CLIENT_DB,
        "API"      => fixed_bits::CLIENT_API,
        "File"     => fixed_bits::CLIENT_FILE,
        _          => fixed_bits::CLIENT_NULL,
    }
}

fn parse_type(value: &Value) -> u64 {
    let s = match value { Value::String(s) => s.as_str(), _ => "" };
    match s {
        "integer"  => fixed_bits::TYPE_I64,
        "string"   => fixed_bits::TYPE_UTF8,
        "float"    => fixed_bits::TYPE_F64,
        "boolean"  => fixed_bits::TYPE_BOOLEAN,
        "datetime" => fixed_bits::TYPE_DATETIME,
        _          => fixed_bits::TYPE_NULL,
    }
}

/// A single template token: either a literal string or a path placeholder.
struct Token {
    text: String,
    is_path: bool,
}

/// Splits a string by `${}` placeholders into tokens.
/// `"user:${session.id}"` → [Token("user:", false), Token("session.id", true)]
fn split_template(s: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut rest = s;

    loop {
        if let Some(start) = rest.find("${") {
            if start > 0 {
                tokens.push(Token { text: rest[..start].to_string(), is_path: false });
            }
            rest = &rest[start + 2..];
            if let Some(end) = rest.find('}') {
                tokens.push(Token { text: rest[..end].to_string(), is_path: true });
                rest = &rest[end + 1..];
            } else {
                tokens.push(Token { text: rest.to_string(), is_path: false });
                break;
            }
        } else {
            if !rest.is_empty() {
                tokens.push(Token { text: rest.to_string(), is_path: false });
            }
            break;
        }
    }

    if tokens.is_empty() {
        tokens.push(Token { text: s.to_string(), is_path: false });
    }

    tokens
}

/// Qualifies a placeholder path to an absolute path.
fn qualify_path(path: &str, filename: &str, ancestors: &[&str]) -> String {
    if path.contains('.') {
        return path.to_string();
    }
    if ancestors.is_empty() {
        format!("{}.{}", filename, path)
    } else {
        format!("{}.{}.{}", filename, ancestors.join("."), path)
    }
}

/// Builds a qualified path string for map keys: `filename.ancestors.key_str`
fn build_qualified_path(filename: &str, ancestors: &[&str], key_str: &str) -> String {
    if ancestors.is_empty() {
        format!("{}.{}", filename, key_str)
    } else {
        format!("{}.{}.{}", filename, ancestors.join("."), key_str)
    }
}

fn yaml_str(value: &Value) -> Result<&str, String> {
    match value {
        Value::String(s) => Ok(s.as_str()),
        _ => Err(format!("expected string key, got {:?}", value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::fixed_bits;

    fn make_pools() -> (DynamicPool, PathMap, ChildrenMap, KeyList, YamlValueList) {
        (DynamicPool::new(), PathMap::new(), ChildrenMap::new(), KeyList::new(), YamlValueList::new())
    }

    const YAML_SESSION: &str = "
sso_user_id:
  _state:
    type: integer
  _store:
    client: InMemory
    key: 'request-attributes-user-key'
  _load:
    client: InMemory
    key: 'request-header-user-key'
";

    const YAML_CACHE: &str = "
user:
  _store:
    client: KVS
    key: 'user:${session.sso_user_id}'
    ttl: 14400
  _load:
    client: Db
    connection: ${connection.tenant}
    table: 'users'
    where: 'sso_user_id=${session.sso_user_id}'
    map:
      id: 'id'
      org_id: 'sso_org_id'
  id:
    _state:
      type: integer
  org_id:
    _state:
      type: integer
";

    #[test]
    fn test_parse_session_yaml() {
        let (mut dynamic, mut path_map, mut children_map, mut keys, mut values) = make_pools();
        let pm = parse("session", YAML_SESSION, &mut dynamic, &mut path_map, &mut children_map, &mut keys, &mut values).unwrap();

        let idx = dynamic.intern("sso_user_id");
        assert_ne!(idx, 0);
        assert!(keys.get(pm.file_key_idx).is_some());
    }

    #[test]
    fn test_field_key_record_root_is_null() {
        let (mut dynamic, mut path_map, mut children_map, mut keys, mut values) = make_pools();
        let pm = parse("session", YAML_SESSION, &mut dynamic, &mut path_map, &mut children_map, &mut keys, &mut values).unwrap();

        // first child of file record should be a field key (ROOT_NULL)
        let file_record = keys.get(pm.file_key_idx).unwrap();
        let child_idx = fixed_bits::get(file_record, fixed_bits::K_OFFSET_CHILD, fixed_bits::K_MASK_CHILD) as u16;
        let record = keys.get(child_idx).unwrap();
        assert_eq!(fixed_bits::get(record, fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT), fixed_bits::ROOT_NULL);
    }

    #[test]
    fn test_meta_key_record_root_index() {
        let (mut dynamic, mut path_map, mut children_map, mut keys, mut values) = make_pools();
        let pm = parse("session", YAML_SESSION, &mut dynamic, &mut path_map, &mut children_map, &mut keys, &mut values).unwrap();

        let mut found = false;
        let start = pm.file_key_idx;
        for i in start..start + 20 {
            if let Some(r) = keys.get(i) {
                if fixed_bits::get(r, fixed_bits::K_OFFSET_ROOT, fixed_bits::K_MASK_ROOT) == fixed_bits::ROOT_STATE {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "_state record with ROOT_STATE not found");
    }

    #[test]
    fn test_type_index_integer() {
        let (mut dynamic, mut path_map, mut children_map, mut keys, mut values) = make_pools();
        let pm = parse("session", YAML_SESSION, &mut dynamic, &mut path_map, &mut children_map, &mut keys, &mut values).unwrap();

        let mut found = false;
        let start = pm.file_key_idx;
        for i in start..start + 20 {
            if let Some(r) = keys.get(i) {
                if fixed_bits::get(r, fixed_bits::K_OFFSET_TYPE, fixed_bits::K_MASK_TYPE) == fixed_bits::TYPE_I64 {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "type=integer record not found");
    }

    #[test]
    fn test_static_value_interned() {
        let (mut dynamic, mut path_map, mut children_map, mut keys, mut values) = make_pools();
        parse("session", YAML_SESSION, &mut dynamic, &mut path_map, &mut children_map, &mut keys, &mut values).unwrap();

        let idx = dynamic.intern("request-attributes-user-key");
        assert_ne!(idx, 0);
    }

    #[test]
    fn test_template_value_is_template_flag() {
        let (mut dynamic, mut path_map, mut children_map, mut keys, mut values) = make_pools();
        parse("cache", YAML_CACHE, &mut dynamic, &mut path_map, &mut children_map, &mut keys, &mut values).unwrap();

        let mut found = false;
        for i in 1..=30 {
            if let Some(vo) = values.get(i) {
                if fixed_bits::get(vo[0], fixed_bits::V_OFFSET_IS_TEMPLATE, fixed_bits::V_MASK_IS_TEMPLATE) == 1 {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "no is_template=1 value record found");
    }

    #[test]
    fn test_path_token_stored_in_path_map() {
        let (mut dynamic, mut path_map, mut children_map, mut keys, mut values) = make_pools();
        parse("cache", YAML_CACHE, &mut dynamic, &mut path_map, &mut children_map, &mut keys, &mut values).unwrap();

        assert!(path_map.get(1).is_some(), "path map is empty");
    }

    #[test]
    fn test_split_template_static() {
        let tokens = split_template("request-attributes-user-key");
        assert_eq!(tokens.len(), 1);
        assert!(!tokens[0].is_path);
        assert_eq!(tokens[0].text, "request-attributes-user-key");
    }

    #[test]
    fn test_split_template_path_only() {
        let tokens = split_template("${connection.tenant}");
        assert_eq!(tokens.len(), 1);
        assert!(tokens[0].is_path);
        assert_eq!(tokens[0].text, "connection.tenant");
    }

    #[test]
    fn test_split_template_mixed() {
        let tokens = split_template("user:${session.sso_user_id}");
        assert_eq!(tokens.len(), 2);
        assert!(!tokens[0].is_path);
        assert_eq!(tokens[0].text, "user:");
        assert!(tokens[1].is_path);
        assert_eq!(tokens[1].text, "session.sso_user_id");
    }

    #[test]
    fn test_qualify_path_absolute() {
        assert_eq!(qualify_path("connection.common", "cache", &["user"]), "connection.common");
    }

    #[test]
    fn test_qualify_path_relative() {
        assert_eq!(qualify_path("org_id", "cache", &["user"]), "cache.user.org_id");
    }

    #[test]
    fn test_qualify_path_relative_no_ancestors() {
        assert_eq!(qualify_path("org_id", "cache", &[]), "cache.org_id");
    }

    #[test]
    fn test_client_kvs_record() {
        let (mut dynamic, mut path_map, mut children_map, mut keys, mut values) = make_pools();
        let pm = parse("cache", YAML_CACHE, &mut dynamic, &mut path_map, &mut children_map, &mut keys, &mut values).unwrap();

        let mut found = false;
        let start = pm.file_key_idx;
        for i in start..start + 30 {
            if let Some(r) = keys.get(i) {
                if fixed_bits::get(r, fixed_bits::K_OFFSET_CLIENT, fixed_bits::K_MASK_CLIENT) == fixed_bits::CLIENT_KVS {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "CLIENT_KVS record not found");
    }

    #[test]
    fn test_two_files_globally_unique_key_idx() {
        // Both session and cache parsed into the same pools — key_idx must be globally unique
        let (mut dynamic, mut path_map, mut children_map, mut keys, mut values) = make_pools();
        let pm_session = parse("session", YAML_SESSION, &mut dynamic, &mut path_map, &mut children_map, &mut keys, &mut values).unwrap();
        let pm_cache   = parse("cache",   YAML_CACHE,   &mut dynamic, &mut path_map, &mut children_map, &mut keys, &mut values).unwrap();

        // file root indices must differ
        assert_ne!(pm_session.file_key_idx, pm_cache.file_key_idx);

        // each file root record holds correct dynamic string
        let sess_rec = keys.get(pm_session.file_key_idx).unwrap();
        let sess_dyn = fixed_bits::get(sess_rec, fixed_bits::K_OFFSET_DYNAMIC, fixed_bits::K_MASK_DYNAMIC) as u16;
        assert_eq!(dynamic.get(sess_dyn), Some("session"));

        let cache_rec = keys.get(pm_cache.file_key_idx).unwrap();
        let cache_dyn = fixed_bits::get(cache_rec, fixed_bits::K_OFFSET_DYNAMIC, fixed_bits::K_MASK_DYNAMIC) as u16;
        assert_eq!(dynamic.get(cache_dyn), Some("cache"));
    }
}
