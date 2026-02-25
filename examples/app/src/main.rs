/// state-engine Sample App - server entry point
///
/// Starts up and waits. Use `state-engine-test` binary to run integration tests.

fn main() {
    println!("state-engine sample app running. Use `state-engine-test` to run integration tests.");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
