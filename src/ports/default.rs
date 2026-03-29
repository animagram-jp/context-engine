use crate::ports::required::FileClient;

pub struct DefaultFileClient;

impl FileClient for DefaultFileClient {
    fn get(&self, path: &str) -> Option<Vec<u8>> {
        std::fs::read(path).ok()
    }
    fn set(&self, path: &str, value: Vec<u8>) -> bool {
        std::fs::write(path, value).is_ok()
    }
    fn delete(&self, path: &str) -> bool {
        std::fs::remove_file(path).is_ok()
    }
}
