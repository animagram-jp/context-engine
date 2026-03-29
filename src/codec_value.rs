use crate::ports::provided::Value;

// Wire format:
//   Null     : 0x00
//   Scalar   : 0x01 | len(u32le) | bytes
//   Sequence : 0x02 | count(u32le) | item...
//   Mapping  : 0x03 | count(u32le) | (key_len(u32le) | key_bytes | item)...

const TAG_NULL:     u8 = 0x00;
const TAG_SCALAR:   u8 = 0x01;
const TAG_SEQUENCE: u8 = 0x02;
const TAG_MAPPING:  u8 = 0x03;

pub fn encode(value: &Value) -> Vec<u8> {
    let mut buf = Vec::new();
    write_value(value, &mut buf);
    buf
}

pub fn decode(bytes: &[u8]) -> Option<Value> {
    let (value, _) = read_value(bytes)?;
    Some(value)
}

fn write_value(value: &Value, buf: &mut Vec<u8>) {
    match value {
        Value::Null => {
            buf.push(TAG_NULL);
        }
        Value::Scalar(b) => {
            buf.push(TAG_SCALAR);
            buf.extend_from_slice(&(b.len() as u32).to_le_bytes());
            buf.extend_from_slice(b);
        }
        Value::Sequence(items) => {
            buf.push(TAG_SEQUENCE);
            buf.extend_from_slice(&(items.len() as u32).to_le_bytes());
            for item in items {
                write_value(item, buf);
            }
        }
        Value::Mapping(pairs) => {
            buf.push(TAG_MAPPING);
            buf.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
            for (k, v) in pairs {
                buf.extend_from_slice(&(k.len() as u32).to_le_bytes());
                buf.extend_from_slice(k);
                write_value(v, buf);
            }
        }
    }
}

fn read_value(bytes: &[u8]) -> Option<(Value, &[u8])> {
    let (&tag, rest) = bytes.split_first()?;
    match tag {
        TAG_NULL => Some((Value::Null, rest)),
        TAG_SCALAR => {
            let (len, rest) = read_u32(rest)?;
            let (data, rest) = split_at(rest, len)?;
            Some((Value::Scalar(data.to_vec()), rest))
        }
        TAG_SEQUENCE => {
            let (count, mut rest) = read_u32(rest)?;
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                let (item, next) = read_value(rest)?;
                items.push(item);
                rest = next;
            }
            Some((Value::Sequence(items), rest))
        }
        TAG_MAPPING => {
            let (count, mut rest) = read_u32(rest)?;
            let mut pairs = Vec::with_capacity(count);
            for _ in 0..count {
                let (klen, next) = read_u32(rest)?;
                let (kdata, next) = split_at(next, klen)?;
                let (val, next) = read_value(next)?;
                pairs.push((kdata.to_vec(), val));
                rest = next;
            }
            Some((Value::Mapping(pairs), rest))
        }
        _ => None,
    }
}

fn read_u32(bytes: &[u8]) -> Option<(usize, &[u8])> {
    let (b, rest) = split_at(bytes, 4)?;
    let n = u32::from_le_bytes(b.try_into().ok()?) as usize;
    Some((n, rest))
}

fn split_at(bytes: &[u8], n: usize) -> Option<(&[u8], &[u8])> {
    if bytes.len() >= n { Some(bytes.split_at(n)) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(v: &Value) -> Value {
        decode(&encode(v)).unwrap()
    }

    #[test]
    fn test_null() {
        assert_eq!(rt(&Value::Null), Value::Null);
    }

    #[test]
    fn test_scalar() {
        assert_eq!(rt(&Value::Scalar(b"hello".to_vec())), Value::Scalar(b"hello".to_vec()));
    }

    #[test]
    fn test_scalar_empty() {
        assert_eq!(rt(&Value::Scalar(vec![])), Value::Scalar(vec![]));
    }

    #[test]
    fn test_sequence() {
        let v = Value::Sequence(vec![
            Value::Scalar(b"a".to_vec()),
            Value::Null,
            Value::Scalar(b"b".to_vec()),
        ]);
        assert_eq!(rt(&v), v);
    }

    #[test]
    fn test_mapping() {
        let v = Value::Mapping(vec![
            (b"id".to_vec(),   Value::Scalar(b"1".to_vec())),
            (b"name".to_vec(), Value::Scalar(b"alice".to_vec())),
        ]);
        assert_eq!(rt(&v), v);
    }

    #[test]
    fn test_nested() {
        let v = Value::Mapping(vec![
            (b"user".to_vec(), Value::Mapping(vec![
                (b"id".to_vec(),    Value::Scalar(b"1".to_vec())),
                (b"tags".to_vec(),  Value::Sequence(vec![
                    Value::Scalar(b"admin".to_vec()),
                    Value::Scalar(b"staff".to_vec()),
                ])),
                (b"extra".to_vec(), Value::Null),
            ])),
        ]);
        assert_eq!(rt(&v), v);
    }

    #[test]
    fn test_decode_invalid_returns_none() {
        assert_eq!(decode(&[0xFF]), None);
        assert_eq!(decode(&[TAG_SCALAR, 0x05, 0x00, 0x00, 0x00]), None); // truncated
    }

    /// Simulate what State::get("cache.user") would return after Db load —
    /// a Mapping built from yaml_to_parse_value output (String → Scalar bytes,
    /// Number → Scalar bytes, Null → Null). Verify encode→decode roundtrip.
    #[test]
    fn test_roundtrip_cache_user_from_yaml() {
        // Equivalent to serde_yaml_ng parsing:
        //   id: 1
        //   org_id: 100
        //   tenant_id: 10
        // yaml_to_parse_value converts Number → Scalar(n.to_string().into_bytes())
        let original = Value::Mapping(vec![
            (b"id".to_vec(),        Value::Scalar(b"1".to_vec())),
            (b"org_id".to_vec(),    Value::Scalar(b"100".to_vec())),
            (b"tenant_id".to_vec(), Value::Scalar(b"10".to_vec())),
        ]);

        let bytes = encode(&original);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded, original);

        // spot-check the wire bytes start with TAG_MAPPING
        assert_eq!(bytes[0], TAG_MAPPING);
        // 3 pairs
        assert_eq!(&bytes[1..5], &3u32.to_le_bytes());
    }

    /// Simulate cache.tenant which has a nested Mapping and a Sequence.
    #[test]
    fn test_roundtrip_nested_from_yaml() {
        // Equivalent to:
        //   name: "acme"
        //   health:
        //     status: "ok"
        //   tags:
        //     - "gold"
        //     - "active"
        let original = Value::Mapping(vec![
            (b"name".to_vec(),   Value::Scalar(b"acme".to_vec())),
            (b"health".to_vec(), Value::Mapping(vec![
                (b"status".to_vec(), Value::Scalar(b"ok".to_vec())),
            ])),
            (b"tags".to_vec(), Value::Sequence(vec![
                Value::Scalar(b"gold".to_vec()),
                Value::Scalar(b"active".to_vec()),
            ])),
        ]);

        let bytes = encode(&original);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded, original);
    }

    /// Null fields survive the roundtrip (yaml `~` or missing values).
    #[test]
    fn test_roundtrip_with_null_field() {
        let original = Value::Mapping(vec![
            (b"id".to_vec(),      Value::Scalar(b"1".to_vec())),
            (b"deleted_at".to_vec(), Value::Null),
        ]);
        assert_eq!(decode(&encode(&original)).unwrap(), original);
    }

    fn from_yaml(v: serde_yaml_ng::Value) -> Value {
        match v {
            serde_yaml_ng::Value::Mapping(m) => Value::Mapping(
                m.into_iter()
                    .filter_map(|(k, v)| {
                        let key = match k {
                            serde_yaml_ng::Value::String(s) => s.into_bytes(),
                            _ => return None,
                        };
                        Some((key, from_yaml(v)))
                    })
                    .collect(),
            ),
            serde_yaml_ng::Value::Sequence(s) => Value::Sequence(
                s.into_iter().map(from_yaml).collect()
            ),
            serde_yaml_ng::Value::String(s)  => Value::Scalar(s.into_bytes()),
            serde_yaml_ng::Value::Number(n)  => Value::Scalar(n.to_string().into_bytes()),
            serde_yaml_ng::Value::Bool(b)    => Value::Scalar(b.to_string().into_bytes()),
            serde_yaml_ng::Value::Null       => Value::Null,
            _                                => Value::Null,
        }
    }

    /// Parse a real YAML string with serde_yaml_ng, convert to Value,
    /// then verify encode→decode roundtrip produces identical Value.
    #[test]
    fn test_roundtrip_real_yaml_cache_user() {
        let yaml = r#"
id: 1
org_id: 100
tenant_id: 10
name: "alice"
active: true
score: 3.14
deleted_at: ~
"#;
        let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
        let original = from_yaml(parsed);

        let bytes = encode(&original);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_roundtrip_real_yaml_nested() {
        let yaml = r#"
user:
  id: 1
  tags:
    - admin
    - staff
  address:
    city: Tokyo
    zip: "100-0001"
  note: ~
"#;
        let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
        let original = from_yaml(parsed);

        let bytes = encode(&original);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded, original);
    }
}
