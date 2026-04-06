use alloc::vec::Vec;

use crate::ports::provided::Tree;

// ── Wire format ───────────────────────────────────────────────────────────────
//
//   Null     : 0x00
//   Scalar   : 0x01 | len(u32le) | bytes
//   Sequence : 0x02 | count(u32le) | item...
//   Mapping  : 0x03 | count(u32le) | (key_len(u32le) | key_bytes | item)...

const TAG_NULL:     u8 = 0x00;
const TAG_SCALAR:   u8 = 0x01;
const TAG_SEQUENCE: u8 = 0x02;
const TAG_MAPPING:  u8 = 0x03;

impl Tree {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        write_value(self, &mut buf);
        buf
    }

    pub fn deserialize(bytes: &[u8]) -> Option<Self> {
        let (value, _) = read_value(bytes)?;
        Some(value)
    }
}

fn write_value(value: &Tree, buf: &mut Vec<u8>) {
    match value {
        Tree::Null => {
            buf.push(TAG_NULL);
        }
        Tree::Scalar(b) => {
            buf.push(TAG_SCALAR);
            buf.extend_from_slice(&(b.len() as u32).to_le_bytes());
            buf.extend_from_slice(b);
        }
        Tree::Sequence(items) => {
            buf.push(TAG_SEQUENCE);
            buf.extend_from_slice(&(items.len() as u32).to_le_bytes());
            for item in items {
                write_value(item, buf);
            }
        }
        Tree::Mapping(pairs) => {
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

fn read_value(bytes: &[u8]) -> Option<(Tree, &[u8])> {
    let (&tag, rest) = bytes.split_first()?;
    match tag {
        TAG_NULL => Some((Tree::Null, rest)),
        TAG_SCALAR => {
            let (len, rest) = read_u32(rest)?;
            let (data, rest) = split_at(rest, len)?;
            Some((Tree::Scalar(data.to_vec()), rest))
        }
        TAG_SEQUENCE => {
            let (count, mut rest) = read_u32(rest)?;
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                let (item, next) = read_value(rest)?;
                items.push(item);
                rest = next;
            }
            Some((Tree::Sequence(items), rest))
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
            Some((Tree::Mapping(pairs), rest))
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

    fn rt(v: &Tree) -> Tree {
        Tree::deserialize(&v.serialize()).unwrap()
    }

    #[test]
    fn test_null() {
        assert_eq!(rt(&Tree::Null), Tree::Null);
    }

    #[test]
    fn test_scalar() {
        assert_eq!(rt(&Tree::Scalar(b"hello".to_vec())), Tree::Scalar(b"hello".to_vec()));
    }

    #[test]
    fn test_scalar_empty() {
        assert_eq!(rt(&Tree::Scalar(vec![])), Tree::Scalar(vec![]));
    }

    #[test]
    fn test_sequence() {
        let v = Tree::Sequence(vec![
            Tree::Scalar(b"a".to_vec()),
            Tree::Null,
            Tree::Scalar(b"b".to_vec()),
        ]);
        assert_eq!(rt(&v), v);
    }

    #[test]
    fn test_mapping() {
        let v = Tree::Mapping(vec![
            (b"id".to_vec(),   Tree::Scalar(b"1".to_vec())),
            (b"name".to_vec(), Tree::Scalar(b"alice".to_vec())),
        ]);
        assert_eq!(rt(&v), v);
    }

    #[test]
    fn test_nested() {
        let v = Tree::Mapping(vec![
            (b"user".to_vec(), Tree::Mapping(vec![
                (b"id".to_vec(),    Tree::Scalar(b"1".to_vec())),
                (b"tags".to_vec(),  Tree::Sequence(vec![
                    Tree::Scalar(b"admin".to_vec()),
                    Tree::Scalar(b"staff".to_vec()),
                ])),
                (b"extra".to_vec(), Tree::Null),
            ])),
        ]);
        assert_eq!(rt(&v), v);
    }

    #[test]
    fn test_deserialize_invalid_returns_none() {
        assert_eq!(Tree::deserialize(&[0xFF]), None);
        assert_eq!(Tree::deserialize(&[TAG_SCALAR, 0x05, 0x00, 0x00, 0x00]), None);
    }

    #[test]
    fn test_roundtrip_null_field() {
        let v = Tree::Mapping(vec![
            (b"id".to_vec(),         Tree::Scalar(b"1".to_vec())),
            (b"deleted_at".to_vec(), Tree::Null),
        ]);
        assert_eq!(Tree::deserialize(&v.serialize()).unwrap(), v);
    }
}

#[cfg(feature = "precompile")]
mod inner {
    // precompile requires std: file I/O (std::fs::write) and UTF-8 parsing
    extern crate std;
    use crate::ports::provided::Tree as TreeData;

    pub struct Tree {
        paths:         Box<[u64]>,
        children:      Box<[u32]>,
        leaves:        Box<[u8]>,
        interning:     Box<[u8]>,
        interning_idx: Box<[u64]>,
    }

    impl Tree {
        pub fn new(
            paths:         Box<[u64]>,
            children:      Box<[u32]>,
            leaves:        Box<[u8]>,
            interning:     Box<[u8]>,
            interning_idx: Box<[u64]>,
        ) -> Self {
            Self { paths, children, leaves, interning, interning_idx }
        }

        pub fn write(&self, path: &str) -> std::io::Result<()> {
            let mut out = String::new();
            out.push_str("// @generated — do not edit by hand\n\n");
            push_u64_slice(&mut out, "PATHS",         &self.paths);
            push_u32_slice(&mut out, "CHILDREN",      &self.children);
            push_u8_slice (&mut out, "LEAVES",        &self.leaves);
            push_u8_slice (&mut out, "INTERNING",     &self.interning);
            push_u64_slice(&mut out, "INTERNING_IDX", &self.interning_idx);
            std::fs::write(path, out)
        }

        /// Parse a YAML byte slice into a `Tree` tree.
        pub fn parse(src: &[u8]) -> Result<TreeData, String> {
            let s = std::str::from_utf8(src)
                .map_err(|e| format!("UTF-8 error: {e}"))?;
            let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(s)
                .map_err(|e| format!("YAML parse error: {e}"))?;
            Ok(yaml_to_tree(yaml))
        }
    }

    fn yaml_to_tree(v: serde_yaml_ng::Value) -> Tree {
        match v {
            serde_yaml_ng::Value::Mapping(m) => Tree::Mapping(
                m.into_iter()
                    .filter_map(|(k, v)| {
                        if let serde_yaml_ng::Value::String(s) = k {
                            Some((s.into_bytes(), yaml_to_tree(v)))
                        } else {
                            None
                        }
                    })
                    .collect(),
            ),
            serde_yaml_ng::Value::Sequence(s) => {
                Tree::Sequence(s.into_iter().map(yaml_to_tree).collect())
            }
            serde_yaml_ng::Value::String(s)  => TreeData::Scalar(s.into_bytes()),
            serde_yaml_ng::Value::Number(n)  => TreeData::Scalar(n.to_string().into_bytes()),
            serde_yaml_ng::Value::Bool(b)    => Tree::Scalar(b.to_string().into_bytes()),
            serde_yaml_ng::Value::Null       => TreeData::Null,
            _                                => TreeData::Null,
        }
    }

    fn push_u64_slice(out: &mut String, name: &str, data: &[u64]) {
        out.push_str(&format!("pub static {name}: &[u64] = &[\n"));
        for chunk in data.chunks(8) {
            out.push_str("    ");
            for v in chunk {
                out.push_str(&format!("0x{v:016x}, "));
            }
            out.push('\n');
        }
        out.push_str("];\n\n");
    }

    fn push_u32_slice(out: &mut String, name: &str, data: &[u32]) {
        out.push_str(&format!("pub static {name}: &[u32] = &[\n"));
        for chunk in data.chunks(8) {
            out.push_str("    ");
            for v in chunk {
                out.push_str(&format!("0x{v:08x}, "));
            }
            out.push('\n');
        }
        out.push_str("];\n\n");
    }

    fn push_u8_slice(out: &mut String, name: &str, data: &[u8]) {
        out.push_str(&format!("pub static {name}: &[u8] = &[\n"));
        for chunk in data.chunks(16) {
            out.push_str("    ");
            for v in chunk {
                out.push_str(&format!("0x{v:02x}, "));
            }
            out.push('\n');
        }
        out.push_str("];\n\n");
    }
}

#[cfg(feature = "precompile")]
pub use inner::Tree;
