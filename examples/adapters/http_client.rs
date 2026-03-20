/// HttpClient implementation (mock for testing)
///
/// Implements the HttpClient Required Port.
/// Returns a fixed health response for any URL.

use serde_json::Value;
use std::collections::HashMap;
use state_engine::ports::required::HttpClient;

pub struct HttpAdapter;

impl HttpClient for HttpAdapter {
    fn get(&self, _url: &str, _headers: Option<&HashMap<String, String>>) -> Option<Value> {
        Some(serde_json::json!({"status": "ok"}))
    }

    fn set(&self, _url: &str, _body: Value, _headers: Option<&HashMap<String, String>>) -> bool {
        true
    }

    fn delete(&self, _url: &str, _headers: Option<&HashMap<String, String>>) -> bool {
        true
    }
}
