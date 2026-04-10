use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::str::from_utf8;

use crate::index::Index;
use crate::ports::provided::{Context as ContextTrait, ContextError, StoreError, LoadError, Tree};
use crate::ports::required::{StoreRegistry, SetOutcome};

// ── Context ───────────────────────────────────────────────────────────────────

/// Request-scoped context instance. Wraps an `Index` and a `StoreRegistry` to resolve DSL paths.
pub struct Context<'r> {
    index:          Arc<Index>,
    registry:       &'r dyn StoreRegistry,
    cache_keys:     Vec<u32>,       // path_idx
    cache_vals:     Vec<Tree>,      // parallel to cache_keys
    called_paths:   BTreeSet<u32>,
    max_recursion:  usize,
}

impl<'r> Context<'r> {
    pub fn new(index: Arc<Index>, registry: &'r dyn StoreRegistry) -> Self {
        Self {
            index,
            registry,
            cache_keys:    Vec::new(),
            cache_vals:    Vec::new(),
            called_paths:   BTreeSet::new(),
            max_recursion: 20,
        }
    }

    fn cache_get(&self, path_idx: u32) -> Option<&Tree> {
        self.cache_keys.iter()
            .position(|&k| k == path_idx)
            .and_then(|i| self.cache_vals.get(i))
    }

    fn cache_set(&mut self, path_idx: u32, value: Tree) {
        if let Some(i) = self.cache_keys.iter().position(|&k| k == path_idx) {
            self.cache_vals[i] = value;
        } else {
            self.cache_keys.push(path_idx);
            self.cache_vals.push(value);
        }
    }

    fn cache_remove(&mut self, path_idx: u32) {
        if let Some(i) = self.cache_keys.iter().position(|&k| k == path_idx) {
            self.cache_keys[i] = u32::MAX;
            self.cache_vals[i] = Tree::Null;
        }
    }

    fn guard_recursion(&self, path_idx: u32) -> Result<(), ContextError> {
        if self.called_paths.len() >= self.max_recursion || self.called_paths.contains(&path_idx) {
            return Err(ContextError::RecursionLimitExceeded);
        }
        Ok(())
    }
}

// ── Context trait impl ────────────────────────────────────────────────────────

impl<'r> ContextTrait for Context<'r> {
    fn get(&mut self, key: &str) -> Result<Option<Tree>, ContextError> {
        let leaves = self.index.traverse(key);
        if leaves.is_empty() {
            return Err(ContextError::KeyNotFound(key.to_string()));
        }

        // single leaf → return value directly
        // multiple leaves → return Mapping of leaf results
        if leaves.len() == 1 {
            let leaf = &leaves[0];
            self.guard_recursion(leaf.path_idx)?;
            self.called_paths.insert(leaf.path_idx);

            let result = self.resolve_leaf(leaf.path_idx, leaf.leaf_offset);

            self.called_paths.remove(&leaf.path_idx);
            result
        } else {
            let mut pairs: Vec<(Vec<u8>, Tree)> = Vec::new();
            for leaf in leaves.iter() {
                self.guard_recursion(leaf.path_idx)?;
                self.called_paths.insert(leaf.path_idx);

                let value = self.resolve_leaf(leaf.path_idx, leaf.leaf_offset)?;

                self.called_paths.remove(&leaf.path_idx);
                if let Some(v) = value {
                    let keyword = self.index.keyword_of(leaf.path_idx).to_vec();
                    pairs.push((keyword, v));
                }
            }
            Ok(if pairs.is_empty() { None } else { Some(Tree::Mapping(pairs)) })
        }
    }

    fn set(&mut self, key: &str, value: Tree) -> Result<bool, ContextError> {
        let leaves = self.index.traverse(key);
        if leaves.is_empty() {
            return Err(ContextError::KeyNotFound(key.to_string()));
        }
        let leaf = &leaves[0];

        let (keyword, map, args) = self.index.store_args(leaf);
        let client = self.registry.client_for(keyword)
            .ok_or_else(|| ContextError::StoreFailed(
                StoreError::ClientNotFound(keyword.to_string())
            ))?;

        let idx_str = alloc::string::ToString::to_string(&leaf.path_idx);
        let store_key = args.get("key")
            .and_then(|v| if let Tree::Scalar(b) = v { from_utf8(b.as_slice()).ok() } else { None })
            .unwrap_or(&idx_str);

        let mut args_ref: BTreeMap<&str, Tree> = args.iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();
        args_ref.insert("value", value.clone());

        match client.set(store_key, &map, &args_ref) {
            Some(SetOutcome::Created) | Some(SetOutcome::Updated) => {
                self.cache_set(leaf.path_idx, value);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn delete(&mut self, key: &str) -> Result<bool, ContextError> {
        let leaves = self.index.traverse(key);
        if leaves.is_empty() {
            return Err(ContextError::KeyNotFound(key.to_string()));
        }
        let leaf = &leaves[0];

        let (keyword, map, args) = self.index.store_args(leaf);
        let client = self.registry.client_for(keyword)
            .ok_or_else(|| ContextError::StoreFailed(
                StoreError::ClientNotFound(keyword.to_string())
            ))?;

        let idx_str = alloc::string::ToString::to_string(&leaf.path_idx);
        let store_key = args.get("key")
            .and_then(|v| if let Tree::Scalar(b) = v { from_utf8(b.as_slice()).ok() } else { None })
            .unwrap_or(&idx_str);

        let args_ref: BTreeMap<&str, Tree> = args.iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();

        let ok = client.delete(store_key, &map, &args_ref);
        if ok {
            self.cache_remove(leaf.path_idx);
        }
        Ok(ok)
    }

    fn exists(&mut self, key: &str) -> Result<bool, ContextError> {
        let leaves = self.index.traverse(key);
        if leaves.is_empty() {
            return Err(ContextError::KeyNotFound(key.to_string()));
        }
        let leaf = &leaves[0];

        if let Some(v) = self.cache_get(leaf.path_idx) {
            return Ok(!matches!(v, Tree::Null));
        }

        let (keyword, map, args) = self.index.store_args(leaf);
        let Some(client) = self.registry.client_for(keyword) else {
            return Ok(false);
        };

        let idx_str = alloc::string::ToString::to_string(&leaf.path_idx);
        let store_key = args.get("key")
            .and_then(|v| if let Tree::Scalar(b) = v { from_utf8(b.as_slice()).ok() } else { None })
            .unwrap_or(&idx_str);

        let args_ref: BTreeMap<&str, Tree> = args.iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();

        Ok(client.get(store_key, &map, &args_ref).is_some())
    }
}

// ── private helpers ───────────────────────────────────────────────────────────

impl<'r> Context<'r> {
    fn resolve_leaf(&mut self, path_idx: u32, leaf_offset: u32) -> Result<Option<Tree>, ContextError> {
        // 1. instance cache
        if let Some(v) = self.cache_get(path_idx) {
            return Ok(Some(v.clone()));
        }

        let leaf_ref = crate::index::LeafRef { path_idx, parent_idx: 0, leaf_offset };

        // 2. _store
        let (store_name, store_map, store_args) = self.index.store_args(&leaf_ref);
        if !store_name.is_empty() {
            if let Some(client) = self.registry.client_for(store_name) {
                // DSL key省略時はpath_idxを文字列化してstore keyとする（compile時確定・一意）
                let idx_str = alloc::string::ToString::to_string(&path_idx);
                let store_key = store_args.get("key")
                    .and_then(|v| if let Tree::Scalar(b) = v { from_utf8(b.as_slice()).ok() } else { None })
                    .unwrap_or(&idx_str);
                let args_ref: BTreeMap<&str, Tree> = store_args.iter()
                    .map(|(k, v)| (k.as_str(), v.clone()))
                    .collect();
                if let Some(value) = client.get(store_key, &store_map, &args_ref) {
                    self.cache_set(path_idx, value.clone());
                    return Ok(Some(value));
                }
            }
        }

        // 3. _load
        let (load_name, load_map, load_args) = self.index.load_args(&leaf_ref);
        if load_name.is_empty() {
            return Ok(None);
        }
        let client = self.registry.client_for(load_name)
            .ok_or_else(|| ContextError::LoadFailed(
                LoadError::ClientNotFound(load_name.to_string())
            ))?;
        let key = load_args.get("key").and_then(|v| {
            if let Tree::Scalar(b) = v { from_utf8(b.as_slice()).ok() } else { None }
        }).ok_or_else(|| ContextError::LoadFailed(
            LoadError::ConfigMissing("key".to_string())
        ))?;
        let args_ref: BTreeMap<&str, Tree> = load_args.iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();
        let value = client.get(key, &load_map, &args_ref)
            .ok_or_else(|| ContextError::LoadFailed(
                LoadError::NotFound(key.to_string())
            ))?;

        // write-through to _store if configured
        if !store_name.is_empty() {
            if let Some(store_client) = self.registry.client_for(store_name) {
                let idx_str = alloc::string::ToString::to_string(&path_idx);
                let sk = store_args.get("key")
                    .and_then(|v| if let Tree::Scalar(b) = v { from_utf8(b.as_slice()).ok() } else { None })
                    .unwrap_or(&idx_str);
                let mut sargs: BTreeMap<&str, Tree> = store_args.iter()
                    .map(|(k, v)| (k.as_str(), v.clone()))
                    .collect();
                sargs.insert("value", value.clone());
                store_client.set(sk, &store_map, &sargs);
            }
        }

        self.cache_set(path_idx, value.clone());
        Ok(Some(value))
    }
}
