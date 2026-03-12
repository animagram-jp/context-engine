/// Generic value type for manifest parsing.
/// Binding-agnostic — no serde, no std, no alloc beyond Vec/String.
///
/// Callers (crate/, wasi/, js/, php/) are responsible for converting
/// their native format (YAML, JSON, etc.) into Value before calling parse().
pub enum Value {
    Mapping(Vec<(String, Value)>),
    Scalar(String),
    Null,
}
