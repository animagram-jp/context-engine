fn main() {}

// Example Store implementations.
// These are minimal stubs showing how to implement Store and Stores
// for common backing stores under the new unified interface.

use context_engine::required::{Store, Stores, SetOutcome};
use context_engine::provided::Tree;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex};

// ── Memory ────────────────────────────────────────────────────────────────────

pub struct MemoryClient {
    data: Mutex<HashMap<String, Tree>>,
}

impl MemoryClient {
    pub fn new() -> Self {
        Self { data: Mutex::new(HashMap::new()) }
    }
}

impl Store for MemoryClient {
    fn get(&self, key: &[u8], _args: &BTreeMap<&str, Tree>) -> Option<Tree> {
        let k = std::str::from_utf8(key).ok()?;
        self.data.lock().unwrap().get(k).cloned()
    }
    fn set(&self, key: &[u8], args: &BTreeMap<&str, Tree>) -> Option<SetOutcome> {
        let k = std::str::from_utf8(key).ok()?.to_string();
        let value = args.get("value")?.clone();
        let mut data = self.data.lock().unwrap();
        let outcome = if data.contains_key(&k) { SetOutcome::Updated } else { SetOutcome::Created(0) };
        data.insert(k, value);
        Some(outcome)
    }
    fn delete(&self, key: &[u8], _args: &BTreeMap<&str, Tree>) -> bool {
        let Ok(k) = std::str::from_utf8(key) else { return false; };
        self.data.lock().unwrap().remove(k).is_some()
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

impl Store for KvsClient {
    fn get(&self, key: &[u8], _args: &BTreeMap<&str, Tree>) -> Option<Tree> {
        let k = std::str::from_utf8(key).ok()?;
        self.data.lock().unwrap().get(k).cloned()
    }
    fn set(&self, key: &[u8], args: &BTreeMap<&str, Tree>) -> Option<SetOutcome> {
        let k = std::str::from_utf8(key).ok()?.to_string();
        let value = args.get("value")?.clone();
        // args["ttl"] ignored in mock
        let mut data = self.data.lock().unwrap();
        let outcome = if data.contains_key(&k) { SetOutcome::Updated } else { SetOutcome::Created(0) };
        data.insert(k, value);
        Some(outcome)
    }
    fn delete(&self, key: &[u8], _args: &BTreeMap<&str, Tree>) -> bool {
        let Ok(k) = std::str::from_utf8(key) else { return false; };
        self.data.lock().unwrap().remove(k).is_some()
    }
}

// ── Env ───────────────────────────────────────────────────────────────────────
//
// Returns a Mapping of { field_name → env_var_value } using map entries from args.

pub struct EnvClient;

impl Store for EnvClient {
    fn get(&self, _key: &[u8], args: &BTreeMap<&str, Tree>) -> Option<Tree> {
        let map = match args.get("map") {
            Some(Tree::Mapping(pairs)) => pairs,
            _ => return None,
        };
        let pairs: Vec<(Vec<u8>, Tree)> = map.iter()
            .filter_map(|(dst, src)| {
                let env_key = match src {
                    Tree::Scalar(b) => std::str::from_utf8(b).ok()?,
                    _ => return None,
                };
                let value = std::env::var(env_key).ok()
                    .map(|s| Tree::Scalar(s.into_bytes()))
                    .unwrap_or(Tree::Null);
                Some((dst.clone(), value))
            })
            .collect();
        if pairs.is_empty() { None } else { Some(Tree::Mapping(pairs)) }
    }
    fn set(&self, _key: &[u8], _args: &BTreeMap<&str, Tree>) -> Option<SetOutcome> { None }
    fn delete(&self, _key: &[u8], _args: &BTreeMap<&str, Tree>) -> bool { false }
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

impl Store for CommonDbClient {
    fn get(&self, key: &[u8], _args: &BTreeMap<&str, Tree>) -> Option<Tree> {
        let k = std::str::from_utf8(key).ok()?;
        self.data.lock().unwrap().get(k).cloned()
    }
    fn set(&self, key: &[u8], args: &BTreeMap<&str, Tree>) -> Option<SetOutcome> {
        let k = std::str::from_utf8(key).ok()?.to_string();
        let value = args.get("value")?.clone();
        let mut data = self.data.lock().unwrap();
        let outcome = if data.contains_key(&k) { SetOutcome::Updated } else { SetOutcome::Created(0) };
        data.insert(k, value);
        Some(outcome)
    }
    fn delete(&self, key: &[u8], _args: &BTreeMap<&str, Tree>) -> bool {
        let Ok(k) = std::str::from_utf8(key) else { return false; };
        self.data.lock().unwrap().remove(k).is_some()
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

impl Store for TenantDbClient {
    fn get(&self, key: &[u8], _args: &BTreeMap<&str, Tree>) -> Option<Tree> {
        let k = std::str::from_utf8(key).ok()?;
        self.data.lock().unwrap().get(k).cloned()
    }
    fn set(&self, key: &[u8], args: &BTreeMap<&str, Tree>) -> Option<SetOutcome> {
        let k = std::str::from_utf8(key).ok()?.to_string();
        let value = args.get("value")?.clone();
        let mut data = self.data.lock().unwrap();
        let outcome = if data.contains_key(&k) { SetOutcome::Updated } else { SetOutcome::Created(0) };
        data.insert(k, value);
        Some(outcome)
    }
    fn delete(&self, key: &[u8], _args: &BTreeMap<&str, Tree>) -> bool {
        let Ok(k) = std::str::from_utf8(key) else { return false; };
        self.data.lock().unwrap().remove(k).is_some()
    }
}

// ── Stores ────────────────────────────────────────────────────────────────────
//
// store_ids passed to Dsl::compile: &["Memory", "Kvs", "Env", "CommonDb", "TenantDb"]
// → store_id: Memory=1, Kvs=2, Env=3, CommonDb=4, TenantDb=5

pub struct MyStores {
    memory:    MemoryClient,
    kvs:       KvsClient,
    env:       EnvClient,
    common_db: CommonDbClient,
    tenant_db: TenantDbClient,
}

impl MyStores {
    pub fn new() -> Self {
        Self {
            memory:    MemoryClient::new(),
            kvs:       KvsClient::new(),
            env:       EnvClient,
            common_db: CommonDbClient::new(),
            tenant_db: TenantDbClient::new(),
        }
    }

    pub fn memory_set(&self, key: &str, value: context_engine::Tree) {
        self.memory.data.lock().unwrap().insert(key.to_string(), value);
    }

    pub fn memory_clear(&self) {
        self.memory.data.lock().unwrap().clear();
    }

    pub fn tenant_db_set(&self, key: &str, value: context_engine::Tree) {
        self.tenant_db.data.lock().unwrap().insert(key.to_string(), value);
    }
}

impl Stores for MyStores {
    fn store_for(&self, id: u8) -> Option<&dyn Store> {
        match id {
            1 => Some(&self.memory),
            2 => Some(&self.kvs),
            3 => Some(&self.env),
            4 => Some(&self.common_db),
            5 => Some(&self.tenant_db),
            _ => None,
        }
    }
}
