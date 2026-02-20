/// state-engine Sample Application
///
/// Demonstrates state-engine concepts with actual Db/KVS connections.

mod adapters;

use adapters::{InMemoryAdapter, EnvAdapter, KVSAdapter, DbAdapter};
use state_engine::{Manifest, State, Load};
use state_engine::ports::provided::State as StateTrait;

#[tokio::main]
async fn main() {
    println!("=== state-engine Sample App ===\n");

    // 1. Load manifests
    let manifest_path = "./manifest";
    println!("1. Loading manifests from: {}", manifest_path);
    let mut manifest = Manifest::new(manifest_path);
    println!("   - Manifests loaded\n");

    // 2. Setup adapters
    println!("2. Setting up adapters...");
    let in_memory_load = InMemoryAdapter::new();
    let mut in_memory_state = InMemoryAdapter::new();
    let env_client = EnvAdapter::new();
    let mut kvs_client_load = match KVSAdapter::new() {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to create KVS client for Load: {}", e);
            return;
        }
    };
    let mut kvs_client_state = match KVSAdapter::new() {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to create KVS client for State: {}", e);
            return;
        }
    };
    let mut db_client = DbAdapter::new();

    println!("   - InMemory adapters initialized");
    println!("   - Env adapter initialized");
    println!("   - KVS adapters initialized");
    println!("   - Db adapter initialized\n");

    // 3. Setup Load with adapters
    println!("3. Configuring Load module...");
    let load = Load::new()
        .with_env_client(&env_client)
        .with_kvs_client(&mut kvs_client_load)
        .with_db_client(&mut db_client)
        .with_in_memory(&in_memory_load);

    println!("   - Load module configured\n");

    // 4. Create State
    println!("4. Creating State...");
    let mut state = State::new(&mut manifest, load)
        .with_in_memory(&mut in_memory_state)
        .with_kvs_client(&mut kvs_client_state);

    println!("   - State initialized\n");

    // 5. Demo: Get connection config
    println!("5. Demo: Loading connection config from Env...");
    match state.get("connection.common") {
        Some(config) => {
            println!("   Connection config loaded:");
            println!("   {}\n", serde_json::to_string_pretty(&config).unwrap());
        }
        None => {
            println!("   Failed to load connection config\n");
        }
    }

    // 6. Demo: Access individual config values
    println!("6. Demo: Accessing nested values...");
    match state.get("connection.common.host") {
        Some(value) => {
            println!("   connection.common.host: {}\n", value);
        }
        None => {
            println!("   Failed to retrieve host\n");
        }
    }

    // 7. Demo: Check existence
    println!("7. Demo: State::exists()...");
    let exists = state.exists("connection.common.host");
    println!("   connection.common.host exists: {}\n", exists);

    // 8. Demo: Get metadata
    println!("8. Demo: Get metadata...");
    let meta = manifest.get_meta("connection.common");
    if let Some(load_meta) = meta.get("_load") {
        println!("   _load metadata:");
        println!("   {}\n", serde_json::to_string_pretty(load_meta).unwrap());
    }

    println!("=== Sample completed ===");
}
