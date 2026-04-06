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
    use alloc::vec;

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

