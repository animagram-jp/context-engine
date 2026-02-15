// Integration tests module

// Initialize logger for tests when logging feature is enabled
#[cfg(feature = "logging")]
#[ctor::ctor]
fn init() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();
}

mod common_test;
mod manifest_test;
mod state_test;
mod edge_cases_test;
mod placeholder_normalization_test;
