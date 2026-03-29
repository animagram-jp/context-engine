/// HttpClient implementation (mock for testing)
///
/// Implements the HttpClient Required Port.
/// Returns a fixed health response for any URL.

use state_engine::Value;
use state_engine::ports::required::HttpClient;

pub struct HttpAdapter;

impl HttpClient for HttpAdapter {
    fn get(&self, _url: &str, _headers: Option<&[(Vec<u8>, Vec<u8>)]>) -> Option<Value> {
        Some(Value::Mapping(vec![
            (b"status".to_vec(), Value::Scalar(b"ok".to_vec())),
        ]))
    }

    fn set(&self, _url: &str, _body: Value, _headers: Option<&[(Vec<u8>, Vec<u8>)]>) -> bool {
        true
    }

    fn delete(&self, _url: &str, _headers: Option<&[(Vec<u8>, Vec<u8>)]>) -> bool {
        true
    }
}
