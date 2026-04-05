// Example StoreClient implementations.
// These are minimal stubs showing how to implement StoreClient and StoreRegistry
// for common backing stores under the new unified interface.

use context_engine::ports::required::{StoreClient, StoreRegistry, SetOutcome};
use context_engine::ports::provided::Tree;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ── Memory ────────────────────────────────────────────────────────────────────

pub struct MemoryClient {
    data: Mutex<HashMap<String, Tree>>,
}

impl MemoryClient {
    pub fn new() -> Self {
        Self { data: Mutex::new(HashMap::new()) }
    }
}

impl StoreClient for MemoryClient {
    fn get(&self, key: &str, _args: &HashMap<&str, Tree>) -> Option<Tree> {
        self.data.lock().unwrap().get(key).cloned()
    }
    fn set(&self, key: &str, args: &HashMap<&str, Tree>) -> Option<SetOutcome> {
        // args["value"] holds the value to store
        let value = args.get("value")?.clone();
        let mut data = self.data.lock().unwrap();
        let outcome = if data.contains_key(key) { SetOutcome::Updated } else { SetOutcome::Created };
        data.insert(key.to_string(), value);
        Some(outcome)
    }
    fn delete(&self, key: &str, _args: &HashMap<&str, Tree>) -> bool {
        self.data.lock().unwrap().remove(key).is_some()
    }
}

// ── KVS (Redis) ───────────────────────────────────────────────────────────────
//
// args["ttl"] — optional, seconds as Scalar

pub struct KvsClient {
    client: Mutex<redis::Client>,
}

impl KvsClient {
    pub fn new(url: &str) -> Result<Self, redis::RedisError> {
        Ok(Self { client: Mutex::new(redis::Client::open(url)?) })
    }
}

impl StoreClient for KvsClient {
    fn get(&self, key: &str, _args: &HashMap<&str, Tree>) -> Option<Tree> {
        let client = self.client.lock().unwrap();
        let mut conn = client.get_connection().ok()?;
        let bytes: Option<Vec<u8>> = redis::cmd("GET").arg(key).query(&mut conn).ok()?;
        // deserialize bytes → Tree (implementor's responsibility)
        bytes.map(|_b| todo!("deserialize"))
    }
    fn set(&self, key: &str, args: &HashMap<&str, Tree>) -> Option<SetOutcome> {
        let client = self.client.lock().unwrap();
        let mut conn = client.get_connection().ok()?;
        let value = args.get("value")?;
        let ttl = args.get("ttl").and_then(|t| match t {
            Tree::Scalar(b) => std::str::from_utf8(b).ok()?.parse::<u64>().ok(),
            _ => None,
        });
        // serialize Tree → bytes (implementor's responsibility)
        let bytes: Vec<u8> = todo!("serialize");
        let result: Result<(), _> = match ttl {
            Some(secs) => redis::cmd("SETEX").arg(key).arg(secs).arg(bytes).query(&mut conn),
            None       => redis::cmd("SET").arg(key).arg(bytes).query(&mut conn),
        };
        result.ok().map(|_| SetOutcome::Created)
    }
    fn delete(&self, key: &str, _args: &HashMap<&str, Tree>) -> bool {
        let client = self.client.lock().unwrap();
        let mut conn = match client.get_connection() { Ok(c) => c, Err(_) => return false };
        let result: Result<i32, _> = redis::cmd("DEL").arg(key).query(&mut conn);
        result.map(|n| n > 0).unwrap_or(false)
    }
}

// ── Env ───────────────────────────────────────────────────────────────────────

pub struct EnvClient;

impl StoreClient for EnvClient {
    fn get(&self, key: &str, _args: &HashMap<&str, Tree>) -> Option<Tree> {
        std::env::var(key).ok().map(|s| Tree::Scalar(s.into_bytes()))
    }
    fn set(&self, _key: &str, _args: &HashMap<&str, Tree>) -> Option<SetOutcome> { None }
    fn delete(&self, _key: &str, _args: &HashMap<&str, Tree>) -> bool { false }
}

// ── StoreRegistry ─────────────────────────────────────────────────────────────

pub struct MyRegistry {
    memory: Arc<MemoryClient>,
    kvs:    Arc<KvsClient>,
    env:    Arc<EnvClient>,
}

impl StoreRegistry for MyRegistry {
    fn client_for(&self, yaml_name: &str) -> Option<&dyn StoreClient> {
        match yaml_name {
            "Memory"   => Some(self.memory.as_ref()),
            "Kvs"      => Some(self.kvs.as_ref()),
            "Env"      => Some(self.env.as_ref()),
            "CommonDb" => todo!("implement CommonDb"),
            "TenantDb" => todo!("implement TenantDb"),
            _          => None,
        }
    }
}
