#[cfg(feature = "precompile")]
mod inner {
    // precompile requires std: file I/O (std::fs::write) and UTF-8 parsing
    extern crate std;
    use crate::ports::provided::Tree as Value;

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

        /// Parse a YAML byte slice into a `Value` tree.
        pub fn parse(src: &[u8]) -> Result<Value, String> {
            let s = std::str::from_utf8(src)
                .map_err(|e| format!("UTF-8 error: {e}"))?;
            let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(s)
                .map_err(|e| format!("YAML parse error: {e}"))?;
            Ok(yaml_to_tree(yaml))
        }
    }

    fn yaml_to_tree(v: serde_yaml_ng::Value) -> Value {
        match v {
            serde_yaml_ng::Value::Mapping(m) => Value::Mapping(
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
                Value::Sequence(s.into_iter().map(yaml_to_tree).collect())
            }
            serde_yaml_ng::Value::String(s)  => Value::Scalar(s.into_bytes()),
            serde_yaml_ng::Value::Number(n)  => Value::Scalar(n.to_string().into_bytes()),
            serde_yaml_ng::Value::Bool(b)    => Value::Scalar(b.to_string().into_bytes()),
            serde_yaml_ng::Value::Null       => Value::Null,
            _                                => Value::Null,
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
