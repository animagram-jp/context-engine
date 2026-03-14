use std::collections::HashMap;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;
use state_engine::{FileClient, InMemoryClient};
use state_engine::State;
use serde_json::Value;

struct ManifestFileClient {
    files: HashMap<String, String>,
}

impl FileClient for ManifestFileClient {
    fn get(&self, path: &str) -> Option<String> {
        self.files.get(path).cloned()
    }
    fn set(&self, _: &str, _: String) -> bool { false }
    fn delete(&self, _: &str) -> bool { false }
}

struct MemoryClient {
    data: Mutex<HashMap<String, Value>>,
}

impl MemoryClient {
    fn new() -> Self {
        Self { data: Mutex::new(HashMap::new()) }
    }
}

impl InMemoryClient for MemoryClient {
    fn get(&self, key: &str) -> Option<Value> {
        self.data.lock().unwrap().get(key).cloned()
    }
    fn set(&self, key: &str, value: Value) -> bool {
        self.data.lock().unwrap().insert(key.to_string(), value);
        true
    }
    fn delete(&self, key: &str) -> bool {
        self.data.lock().unwrap().remove(key).is_some()
    }
}

#[wasm_bindgen]
pub struct StateEngine {
    state: State<'static>,
    _memory: Box<MemoryClient>,
}

#[wasm_bindgen]
impl StateEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(manifest_dir: &str, files: js_sys::Object) -> StateEngine {
        let mut map = HashMap::new();
        for entry in js_sys::Object::entries(&files).iter() {
            let pair = js_sys::Array::from(&entry);
            let key = pair.get(0).as_string().unwrap_or_default();
            let val = pair.get(1).as_string().unwrap_or_default();
            map.insert(key, val);
        }

        let memory = Box::new(MemoryClient::new());
        let memory_ref: &'static MemoryClient = unsafe { &*(memory.as_ref() as *const MemoryClient) };

        let state = State::new(manifest_dir)
            .with_file(ManifestFileClient { files: map })
            .with_in_memory(memory_ref);

        StateEngine { state, _memory: memory }
    }

    pub fn get(&mut self, key: &str) -> Result<JsValue, JsValue> {
        self.state.get(key)
            .map(|v| match v {
                Some(val) => JsValue::from_str(&val.to_string()),
                None => JsValue::NULL,
            })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn set(&mut self, key: &str, value: &str, ttl: Option<u32>) -> Result<bool, JsValue> {
        let v: Value = serde_json::from_str(value)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.state.set(key, v, ttl.map(|t| t as u64))
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn delete(&mut self, key: &str) -> Result<bool, JsValue> {
        self.state.delete(key)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn exists(&mut self, key: &str) -> Result<bool, JsValue> {
        self.state.exists(key)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
