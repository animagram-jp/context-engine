/// Minimal DSL value type for manifest parsing.
/// Binding-agnostic — no serde, no std, no alloc beyond Vec/String.
///
/// Callers (crate/, wasi/, js/, php/) are responsible for converting
/// their native format (YAML, JSON, etc.) into DslValue before calling parse().
pub enum DslValue {
    Mapping(Vec<(String, DslValue)>),
    Scalar(String),
    Null,
}
