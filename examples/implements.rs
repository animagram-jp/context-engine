fn main() {}

// Example StoreClient implementations.
// These are minimal stubs showing how to implement StoreClient and StoreRegistry
// for common backing stores under the new unified interface.

use context_engine::ports::required::{StoreClient, StoreRegistry, SetOutcome};
use context_engine::ports::provided::Tree;
use std::collections::{BTreeMap, HashMap};
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
    fn get(&self, key: &str, _args: &BTreeMap<&str, Tree>) -> Option<Tree> {
        self.data.lock().unwrap().get(key).cloned()
    }
    fn set(&self, key: &str, args: &BTreeMap<&str, Tree>) -> Option<SetOutcome> {
        let value = args.get("value")?.clone();
        let mut data = self.data.lock().unwrap();
        let outcome = if data.contains_key(key) { SetOutcome::Updated } else { SetOutcome::Created };
        data.insert(key.to_string(), value);
        Some(outcome)
    }
    fn delete(&self, key: &str, _args: &BTreeMap<&str, Tree>) -> bool {
        self.data.lock().unwrap().remove(key).is_some()
    }
}

// ── KVS (Redis-like mock) ─────────────────────────────────────────────────────
//
// args["ttl"] — optional, seconds as Scalar

pub struct KvsClient {
    data: Mutex<HashMap<String, Tree>>,
}

impl KvsClient {
    pub fn new() -> Self {
        Self { data: Mutex::new(HashMap::new()) }
    }
}

impl StoreClient for KvsClient {
    fn get(&self, key: &str, _args: &BTreeMap<&str, Tree>) -> Option<Tree> {
        let bytes = self.data.lock().unwrap().get(key).cloned()?;
        // In real impl: unwire bytes → Tree
        Some(bytes)
    }
    fn set(&self, key: &str, args: &BTreeMap<&str, Tree>) -> Option<SetOutcome> {
        let value = args.get("value")?.clone();
        // args["ttl"] ignored in mock
        let mut data = self.data.lock().unwrap();
        let outcome = if data.contains_key(key) { SetOutcome::Updated } else { SetOutcome::Created };
        data.insert(key.to_string(), value);
        Some(outcome)
    }
    fn delete(&self, key: &str, _args: &BTreeMap<&str, Tree>) -> bool {
        self.data.lock().unwrap().remove(key).is_some()
    }
}

// ── Env ───────────────────────────────────────────────────────────────────────
//
// args contains map.* values as env var names (in order).
// Returns a Mapping of { env_var_name → env_var_value }.

pub struct EnvClient;

impl StoreClient for EnvClient {
    fn get(&self, _key: &str, args: &BTreeMap<&str, Tree>) -> Option<Tree> {
        let pairs: Vec<(Vec<u8>, Tree)> = args.iter()
            .filter_map(|(&k, v)| {
                let env_key = match v {
                    Tree::Scalar(b) => std::str::from_utf8(b).ok()?,
                    _ => return None,
                };
                let value = std::env::var(env_key).ok()
                    .map(|s| Tree::Scalar(s.into_bytes()))
                    .unwrap_or(Tree::Null);
                Some((k.as_bytes().to_vec(), value))
            })
            .collect();
        if pairs.is_empty() { None } else { Some(Tree::Mapping(pairs)) }
    }
    fn set(&self, _key: &str, _args: &BTreeMap<&str, Tree>) -> Option<SetOutcome> { None }
    fn delete(&self, _key: &str, _args: &BTreeMap<&str, Tree>) -> bool { false }
}

// ── CommonDb (mock) ───────────────────────────────────────────────────────────

pub struct CommonDbClient {
    data: Mutex<HashMap<String, Tree>>,
}

impl CommonDbClient {
    pub fn new() -> Self {
        Self { data: Mutex::new(HashMap::new()) }
    }
}

impl StoreClient for CommonDbClient {
    fn get(&self, key: &str, _args: &BTreeMap<&str, Tree>) -> Option<Tree> {
        self.data.lock().unwrap().get(key).cloned()
    }
    fn set(&self, key: &str, args: &BTreeMap<&str, Tree>) -> Option<SetOutcome> {
        let value = args.get("value")?.clone();
        let mut data = self.data.lock().unwrap();
        let outcome = if data.contains_key(key) { SetOutcome::Updated } else { SetOutcome::Created };
        data.insert(key.to_string(), value);
        Some(outcome)
    }
    fn delete(&self, key: &str, _args: &BTreeMap<&str, Tree>) -> bool {
        self.data.lock().unwrap().remove(key).is_some()
    }
}

// ── TenantDb (mock) ───────────────────────────────────────────────────────────

pub struct TenantDbClient {
    data: Mutex<HashMap<String, Tree>>,
}

impl TenantDbClient {
    pub fn new() -> Self {
        Self { data: Mutex::new(HashMap::new()) }
    }
}

impl StoreClient for TenantDbClient {
    fn get(&self, key: &str, _args: &BTreeMap<&str, Tree>) -> Option<Tree> {
        self.data.lock().unwrap().get(key).cloned()
    }
    fn set(&self, key: &str, args: &BTreeMap<&str, Tree>) -> Option<SetOutcome> {
        let value = args.get("value")?.clone();
        let mut data = self.data.lock().unwrap();
        let outcome = if data.contains_key(key) { SetOutcome::Updated } else { SetOutcome::Created };
        data.insert(key.to_string(), value);
        Some(outcome)
    }
    fn delete(&self, key: &str, _args: &BTreeMap<&str, Tree>) -> bool {
        self.data.lock().unwrap().remove(key).is_some()
    }
}

// ── StoreRegistry ─────────────────────────────────────────────────────────────

pub struct MyRegistry {
    memory:    Arc<MemoryClient>,
    kvs:       Arc<KvsClient>,
    env:       Arc<EnvClient>,
    common_db: Arc<CommonDbClient>,
    tenant_db: Arc<TenantDbClient>,
}

impl MyRegistry {
    pub fn new() -> Self {
        Self {
            memory:    Arc::new(MemoryClient::new()),
            kvs:       Arc::new(KvsClient::new()),
            env:       Arc::new(EnvClient),
            common_db: Arc::new(CommonDbClient::new()),
            tenant_db: Arc::new(TenantDbClient::new()),
        }
    }

    pub fn memory_set(&self, key: &str, value: context_engine::Tree) {
        self.memory.data.lock().unwrap().insert(key.to_string(), value);
    }

    pub fn memory_clear(&self) {
        self.memory.data.lock().unwrap().clear();
    }
}

impl StoreRegistry for MyRegistry {
    fn client_for(&self, keyword: &str) -> Option<&dyn StoreClient> {
        match keyword {
            "Memory"   => Some(self.memory.as_ref()),
            "Kvs"      => Some(self.kvs.as_ref()),
            "Env"      => Some(self.env.as_ref()),
            "CommonDb" => Some(self.common_db.as_ref()),
            "TenantDb" => Some(self.tenant_db.as_ref()),
            _          => None,
        }
    }
}
