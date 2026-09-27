fn main() {}

// Example Store implementations.
// These are minimal stubs showing how to implement Store and Stores
// for common backing stores under the new unified interface.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Mutex,
};

use context_engine::{
    provided::Tree,
    required::{SetOutcome, Store, Stores},
};

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
        let outcome =
            if data.contains_key(&k) { SetOutcome::Updated } else { SetOutcome::Created(0) };
        data.insert(k, value);
        Some(outcome)
    }
    fn delete(&self, key: &[u8], _args: &BTreeMap<&str, Tree>) -> bool {
        let Ok(k) = std::str::from_utf8(key) else {
            return false;
        };
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
        let outcome =
            if data.contains_key(&k) { SetOutcome::Updated } else { SetOutcome::Created(0) };
        data.insert(k, value);
        Some(outcome)
    }
    fn delete(&self, key: &[u8], _args: &BTreeMap<&str, Tree>) -> bool {
        let Ok(k) = std::str::from_utf8(key) else {
            return false;
        };
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
        let pairs: Vec<(Vec<u8>, Tree)> = map
            .iter()
            .filter_map(|(dst, src)| {
                let env_key = match src {
                    Tree::Scalar(b) => std::str::from_utf8(b).ok()?,
                    _ => return None,
                };
                let value = std::env::var(env_key)
                    .ok()
                    .map(|s| Tree::Scalar(s.into_bytes()))
                    .unwrap_or(Tree::Null);
                Some((dst.clone(), value))
            })
            .collect();
        if pairs.is_empty() { None } else { Some(Tree::Mapping(pairs)) }
    }
    fn set(&self, _key: &[u8], _args: &BTreeMap<&str, Tree>) -> Option<SetOutcome> {
        None
    }
    fn delete(&self, _key: &[u8], _args: &BTreeMap<&str, Tree>) -> bool {
        false
    }
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
        let outcome =
            if data.contains_key(&k) { SetOutcome::Updated } else { SetOutcome::Created(0) };
        data.insert(k, value);
        Some(outcome)
    }
    fn delete(&self, key: &[u8], _args: &BTreeMap<&str, Tree>) -> bool {
        let Ok(k) = std::str::from_utf8(key) else {
            return false;
        };
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
        let outcome =
            if data.contains_key(&k) { SetOutcome::Updated } else { SetOutcome::Created(0) };
        data.insert(k, value);
        Some(outcome)
    }
    fn delete(&self, key: &[u8], _args: &BTreeMap<&str, Tree>) -> bool {
        let Ok(k) = std::str::from_utf8(key) else {
            return false;
        };
        self.data.lock().unwrap().remove(k).is_some()
    }
}

// ── Stores ────────────────────────────────────────────────────────────────────

pub struct MyStores {
    store_ids: std::vec::Vec<std::string::String>,
    memory:    MemoryClient,
    kvs:       KvsClient,
    env:       EnvClient,
    common_db: CommonDbClient,
    tenant_db: TenantDbClient,
}

impl MyStores {
    pub fn new(store_ids: &[&str]) -> Self {
        Self {
            store_ids: store_ids.iter().map(|s| s.to_string()).collect(),
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
        let idx = (id as usize).checked_sub(1)?;
        match self.store_ids.get(idx)?.as_str() {
            "Memory" => Some(&self.memory),
            "Kvs" => Some(&self.kvs),
            "Env" => Some(&self.env),
            "CommonDb" => Some(&self.common_db),
            "TenantDb" => Some(&self.tenant_db),
            _ => None,
        }
    }
}
