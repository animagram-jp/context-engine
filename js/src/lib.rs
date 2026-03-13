// JS/WASM binding for state-engine.
//
// Exposes state-engine to JavaScript via wasm-bindgen.
// Manifest files are passed as strings from the JS side (no filesystem access).
//
// TODO: add wasm-bindgen dep and annotate exports with #[wasm_bindgen].

use core::parser::Value;

/// Converts a JSON string from JS into core::parser::Value.
///
/// JS side serializes manifest content to JSON and passes it here.
/// TODO: implement JSON → Value traversal (serde_json as dep, or manual).
pub fn json_to_value(_input: &str) -> Value {
    todo!("deserialize JSON string into Value")
}

/// FileClient implementation that receives file contents from JS.
///
/// JS side reads files and passes contents as strings to Rust.
/// No filesystem access on the Rust side.
pub struct JsFileClient {
    // TODO: hold a wasm_bindgen::JsValue callback or pre-loaded map of filename → content
}

// impl FileClient for JsFileClient {
//     fn get(&self, path: &str) -> Option<String> { todo!() }
//     fn set(&self, _: &str, _: String) -> bool { false }
//     fn delete(&self, _: &str) -> bool { false }
// }
