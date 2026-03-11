use crate::ports::required::FileClient;

/// Default FileClient using std::fs. For use in native (crate) environments.
/// WASI/JS bindings should provide their own FileClient implementation.
pub struct DefaultFileClient;

impl FileClient for DefaultFileClient {
    fn get(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }
    fn set(&self, path: &str, value: String) -> bool {
        std::fs::write(path, value).is_ok()
    }
    fn delete(&self, path: &str) -> bool {
        std::fs::remove_file(path).is_ok()
    }
}
