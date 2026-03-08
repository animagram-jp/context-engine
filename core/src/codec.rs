use crate::fixed_bits;

pub const ROOT_NAMES: &[(&str, u64)] = &[
    ("_load",  fixed_bits::ROOT_LOAD),
    ("_store", fixed_bits::ROOT_STORE),
    ("_state", fixed_bits::ROOT_STATE),
];

pub fn root_encode(s: &str) -> u64 {
    ROOT_NAMES.iter()
        .find(|(name, _)| *name == s)
        .map(|(_, v)| *v)
        .unwrap_or(fixed_bits::ROOT_NULL)
}

pub fn root_decode(v: u64) -> Option<&'static str> {
    ROOT_NAMES.iter()
        .find(|(_, val)| *val == v)
        .map(|(name, _)| *name)
}

pub const CLIENT_NAMES: &[(&str, u64)] = &[
    ("State",    fixed_bits::CLIENT_STATE),
    ("InMemory", fixed_bits::CLIENT_IN_MEMORY),
    ("Env",      fixed_bits::CLIENT_ENV),
    ("KVS",      fixed_bits::CLIENT_KVS),
    ("Db",       fixed_bits::CLIENT_DB),
    ("HTTP",     fixed_bits::CLIENT_HTTP),
    ("File",     fixed_bits::CLIENT_FILE),
];

pub fn client_encode(s: &str) -> u64 {
    CLIENT_NAMES.iter()
        .find(|(name, _)| *name == s)
        .map(|(_, v)| *v)
        .unwrap_or(fixed_bits::CLIENT_NULL)
}

pub fn client_decode(v: u64) -> Option<&'static str> {
    CLIENT_NAMES.iter()
        .find(|(_, val)| *val == v)
        .map(|(name, _)| *name)
}

pub const PROP_NAMES: &[(&str, u64)] = &[
    ("type",       fixed_bits::PROP_TYPE),
    ("key",        fixed_bits::PROP_KEY),
    ("connection", fixed_bits::PROP_CONNECTION),
    ("map",        fixed_bits::PROP_MAP),
    ("ttl",        fixed_bits::PROP_TTL),
    ("table",      fixed_bits::PROP_TABLE),
    ("where",      fixed_bits::PROP_WHERE),
];

pub fn prop_encode(s: &str) -> u64 {
    PROP_NAMES.iter()
        .find(|(name, _)| *name == s)
        .map(|(_, v)| *v)
        .unwrap_or(fixed_bits::PROP_NULL)
}

pub fn prop_decode(v: u64) -> Option<&'static str> {
    PROP_NAMES.iter()
        .find(|(_, val)| *val == v)
        .map(|(name, _)| *name)
}

pub const TYPE_NAMES: &[(&str, u64)] = &[
    ("integer",  fixed_bits::TYPE_I64),
    ("string",   fixed_bits::TYPE_UTF8),
    ("float",    fixed_bits::TYPE_F64),
    ("boolean",  fixed_bits::TYPE_BOOLEAN),
    ("datetime", fixed_bits::TYPE_DATETIME),
];

pub fn type_encode(s: &str) -> u64 {
    TYPE_NAMES.iter()
        .find(|(name, _)| *name == s)
        .map(|(_, v)| *v)
        .unwrap_or(fixed_bits::TYPE_NULL)
}

pub fn type_decode(v: u64) -> Option<&'static str> {
    TYPE_NAMES.iter()
        .find(|(_, val)| *val == v)
        .map(|(name, _)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_codec() {
        for &(name, val) in CLIENT_NAMES {
            assert_eq!(client_encode(name), val);
            assert_eq!(client_decode(val), Some(name));
        }
    }

    #[test]
    fn test_root_codec() {
        for &(name, val) in ROOT_NAMES {
            assert_eq!(root_encode(name), val);
            assert_eq!(root_decode(val), Some(name));
        }
    }

    #[test]
    fn test_prop_codec() {
        for &(name, val) in PROP_NAMES {
            assert_eq!(prop_encode(name), val);
            assert_eq!(prop_decode(val), Some(name));
        }
    }

    #[test]
    fn test_type_codec() {
        for &(name, val) in TYPE_NAMES {
            assert_eq!(type_encode(name), val);
            assert_eq!(type_decode(val), Some(name));
        }
    }

    #[test]
    fn test_unknown_encode() {
        assert_eq!(client_encode("Unknown"), fixed_bits::CLIENT_NULL);
        assert_eq!(root_encode("_unknown"), fixed_bits::ROOT_NULL);
        assert_eq!(prop_encode("unknown"), fixed_bits::PROP_NULL);
        assert_eq!(type_encode("unknown"), fixed_bits::TYPE_NULL);
    }

    #[test]
    fn test_null_decode() {
        assert_eq!(client_decode(fixed_bits::CLIENT_NULL), None);
        assert_eq!(root_decode(fixed_bits::ROOT_NULL), None);
        assert_eq!(prop_decode(fixed_bits::PROP_NULL), None);
        assert_eq!(type_decode(fixed_bits::TYPE_NULL), None);
    }
}
