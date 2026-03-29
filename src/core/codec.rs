use super::fixed_bits;

pub const ROOT_NAMES: &[(&[u8], u64)] = &[
    (b"_load",  fixed_bits::ROOT_LOAD),
    (b"_store", fixed_bits::ROOT_STORE),
    (b"_state", fixed_bits::ROOT_STATE),
];

pub fn root_encode(s: &[u8]) -> u64 {
    ROOT_NAMES.iter()
        .find(|(name, _)| *name == s)
        .map(|(_, v)| *v)
        .unwrap_or(fixed_bits::ROOT_NULL)
}

pub fn root_decode(v: u64) -> Option<&'static [u8]> {
    ROOT_NAMES.iter()
        .find(|(_, val)| *val == v)
        .map(|(name, _)| *name)
}

pub const CLIENT_NAMES: &[(&[u8], u64)] = &[
    (b"State",    fixed_bits::CLIENT_STATE),
    (b"InMemory", fixed_bits::CLIENT_IN_MEMORY),
    (b"Env",      fixed_bits::CLIENT_ENV),
    (b"KVS",      fixed_bits::CLIENT_KVS),
    (b"Db",       fixed_bits::CLIENT_DB),
    (b"HTTP",     fixed_bits::CLIENT_HTTP),
    (b"File",     fixed_bits::CLIENT_FILE),
];

pub fn client_encode(s: &[u8]) -> u64 {
    CLIENT_NAMES.iter()
        .find(|(name, _)| *name == s)
        .map(|(_, v)| *v)
        .unwrap_or(fixed_bits::CLIENT_NULL)
}

pub fn client_decode(v: u64) -> Option<&'static [u8]> {
    CLIENT_NAMES.iter()
        .find(|(_, val)| *val == v)
        .map(|(name, _)| *name)
}

pub const PROP_NAMES: &[(&[u8], u64)] = &[
    (b"type",       fixed_bits::PROP_TYPE),
    (b"key",        fixed_bits::PROP_KEY),
    (b"connection", fixed_bits::PROP_CONNECTION),
    (b"map",        fixed_bits::PROP_MAP),
    (b"ttl",        fixed_bits::PROP_TTL),
    (b"table",      fixed_bits::PROP_TABLE),
    (b"where",      fixed_bits::PROP_WHERE),
    (b"url",        fixed_bits::PROP_URL),
    (b"headers",    fixed_bits::PROP_HEADERS),
];

pub fn prop_encode(s: &[u8]) -> u64 {
    PROP_NAMES.iter()
        .find(|(name, _)| *name == s)
        .map(|(_, v)| *v)
        .unwrap_or(fixed_bits::PROP_NULL)
}

pub fn prop_decode(v: u64) -> Option<&'static [u8]> {
    PROP_NAMES.iter()
        .find(|(_, val)| *val == v)
        .map(|(name, _)| *name)
}

pub const TYPE_NAMES: &[(&[u8], u64)] = &[
    (b"integer",  fixed_bits::TYPE_I64),
    (b"string",   fixed_bits::TYPE_UTF8),
    (b"float",    fixed_bits::TYPE_F64),
    (b"boolean",  fixed_bits::TYPE_BOOLEAN),
    (b"datetime", fixed_bits::TYPE_DATETIME),
];

pub fn type_encode(s: &[u8]) -> u64 {
    TYPE_NAMES.iter()
        .find(|(name, _)| *name == s)
        .map(|(_, v)| *v)
        .unwrap_or(fixed_bits::TYPE_NULL)
}

pub fn type_decode(v: u64) -> Option<&'static [u8]> {
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
        assert_eq!(client_encode(b"Unknown"), fixed_bits::CLIENT_NULL);
        assert_eq!(root_encode(b"_unknown"), fixed_bits::ROOT_NULL);
        assert_eq!(prop_encode(b"unknown"), fixed_bits::PROP_NULL);
        assert_eq!(type_encode(b"unknown"), fixed_bits::TYPE_NULL);
    }

    #[test]
    fn test_null_decode() {
        assert_eq!(client_decode(fixed_bits::CLIENT_NULL), None);
        assert_eq!(root_decode(fixed_bits::ROOT_NULL), None);
        assert_eq!(prop_decode(fixed_bits::PROP_NULL), None);
        assert_eq!(type_decode(fixed_bits::TYPE_NULL), None);
    }
}
