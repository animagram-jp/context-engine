use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::str::from_utf8;

use crate::index::Index;
use crate::provided::{Context as ContextTrait, ContextError, StoreError, LoadError, Tree};
use crate::required::{Stores, SetOutcome};

// ── Context ───────────────────────────────────────────────────────────────────

pub struct Context<'r> {
    index:         Arc<Index>,
    stores:        &'r dyn Stores,
    cache_keys:    Vec<u16>,
    cache_vals:    Vec<Tree>,
    called_paths:  BTreeSet<u16>,
    max_recursion: usize,
}

impl<'r> Context<'r> {
    pub fn new(index: Arc<Index>, stores: &'r dyn Stores) -> Self {
        Self {
            index,
            stores,
            cache_keys:    Vec::new(),
            cache_vals:    Vec::new(),
            called_paths:   BTreeSet::new(),
            max_recursion: 20,
        }
    }

    fn cache_get(&self, path_idx: u16) -> Option<&Tree> {
        self.cache_keys.iter()
            .position(|&k| k == path_idx)
            .and_then(|i| self.cache_vals.get(i))
    }

    fn cache_set(&mut self, path_idx: u16, value: Tree) {
        if let Some(i) = self.cache_keys.iter().position(|&k| k == path_idx) {
            self.cache_vals[i] = value;
        } else {
            self.cache_keys.push(path_idx);
            self.cache_vals.push(value);
        }
    }

    fn cache_remove(&mut self, path_idx: u16) {
        if let Some(i) = self.cache_keys.iter().position(|&k| k == path_idx) {
            self.cache_keys[i] = u16::MAX;
            self.cache_vals[i] = Tree::Null;
        }
    }

    fn guard_recursion(&self, path_idx: u16) -> Result<(), ContextError> {
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

        if leaves.len() == 1 {
            let leaf = &leaves[0];
            self.guard_recursion(leaf.path_idx)?;
            self.called_paths.insert(leaf.path_idx);
            let result = self.resolve_leaf(leaf.path_idx, leaf.leaf_id, leaf.value_id);
            self.called_paths.remove(&leaf.path_idx);
            result
        } else {
            let mut pairs: Vec<(Vec<u8>, Tree)> = Vec::new();
            for leaf in leaves.iter() {
                self.guard_recursion(leaf.path_idx)?;
                self.called_paths.insert(leaf.path_idx);
                let value = self.resolve_leaf(leaf.path_idx, leaf.leaf_id, leaf.value_id)?;
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

        let (store_id, args) = self.index.set_args(leaf);
        let store = self.stores.store_for(store_id)
            .ok_or_else(|| ContextError::StoreFailed(
                StoreError::ClientNotFound(store_id.to_string())
            ))?;

        let idx_str = alloc::string::ToString::to_string(&leaf.path_idx);
        let store_key = args.get("key")
            .and_then(|v| if let Tree::Scalar(b) = v { from_utf8(b.as_slice()).ok() } else { None })
            .unwrap_or(&idx_str);

        let mut args_ref: BTreeMap<&str, Tree> = args.iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();
        args_ref.insert("value", value.clone());

        match store.set(store_key.as_bytes(), &args_ref) {
            Some(SetOutcome::Created(_)) | Some(SetOutcome::Updated) => {
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

        let (store_id, args) = self.index.set_args(leaf);
        let store = self.stores.store_for(store_id)
            .ok_or_else(|| ContextError::StoreFailed(
                StoreError::ClientNotFound(store_id.to_string())
            ))?;

        let idx_str = alloc::string::ToString::to_string(&leaf.path_idx);
        let store_key = args.get("key")
            .and_then(|v| if let Tree::Scalar(b) = v { from_utf8(b.as_slice()).ok() } else { None })
            .unwrap_or(&idx_str);

        let args_ref: BTreeMap<&str, Tree> = args.iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();

        let ok = store.delete(store_key.as_bytes(), &args_ref);
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

        let (store_id, args) = self.index.set_args(leaf);
        let Some(store) = self.stores.store_for(store_id) else {
            return Ok(false);
        };

        let idx_str = alloc::string::ToString::to_string(&leaf.path_idx);
        let store_key = args.get("key")
            .and_then(|v| if let Tree::Scalar(b) = v { from_utf8(b.as_slice()).ok() } else { None })
            .unwrap_or(&idx_str);

        let args_ref: BTreeMap<&str, Tree> = args.iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();

        Ok(store.get(store_key.as_bytes(), &args_ref).is_some())
    }
}

// ── private helpers ───────────────────────────────────────────────────────────

impl<'r> Context<'r> {
    fn resolve_leaf(&mut self, path_idx: u16, leaf_id: u16, value_id: u16) -> Result<Option<Tree>, ContextError> {
        if let Some(v) = self.cache_get(path_idx) {
            return Ok(Some(v.clone()));
        }

        let leaf_ref = crate::index::LeafRef { path_idx, leaf_id, value_id };

        // _set
        let (set_store_id, set_args) = self.index.set_args(&leaf_ref);
        if set_store_id != 0 {
            if let Some(store) = self.stores.store_for(set_store_id) {
                let idx_str = alloc::string::ToString::to_string(&path_idx);
                let store_key = set_args.get("key")
                    .and_then(|v| if let Tree::Scalar(b) = v { from_utf8(b.as_slice()).ok() } else { None })
                    .unwrap_or(&idx_str);
                let args_ref: BTreeMap<&str, Tree> = set_args.iter()
                    .map(|(k, v)| (k.as_str(), v.clone()))
                    .collect();
                if let Some(value) = store.get(store_key.as_bytes(), &args_ref) {
                    self.cache_set(path_idx, value.clone());
                    return Ok(Some(value));
                }
            }
        }

        // value fragments (static scalar / placeholder / template)
        let frags: Vec<(bool, Vec<u8>)> = self.index.leaf_fragments(&leaf_ref)
            .into_iter()
            .map(|(is_ph, b)| (is_ph, b.to_vec()))
            .collect();
        if !frags.is_empty() {
            let value = if frags.len() == 1 && frags[0].0 {
                let path_str = from_utf8(&frags[0].1)
                    .map_err(|_| ContextError::LoadFailed(
                        crate::provided::LoadError::ConfigMissing("placeholder utf8".to_string())
                    ))?
                    .to_string();
                self.get(&path_str)?
                    .ok_or_else(|| ContextError::LoadFailed(
                        crate::provided::LoadError::NotFound(path_str.clone())
                    ))?
            } else {
                let mut buf = String::new();
                for (is_ph, bytes) in frags {
                    if is_ph {
                        let path_str = from_utf8(&bytes)
                            .map_err(|_| ContextError::LoadFailed(
                                crate::provided::LoadError::ConfigMissing("placeholder utf8".to_string())
                            ))?;
                        match self.get(path_str)? {
                            Some(Tree::Scalar(b)) => {
                                buf.push_str(from_utf8(&b).unwrap_or(""));
                            }
                            Some(_) => {}
                            None => return Err(ContextError::LoadFailed(
                                crate::provided::LoadError::NotFound(path_str.to_string())
                            )),
                        }
                    } else {
                        buf.push_str(from_utf8(&bytes).unwrap_or(""));
                    }
                }
                Tree::Scalar(buf.into_bytes())
            };
            self.cache_set(path_idx, value.clone());
            return Ok(Some(value));
        }

        // _get
        let (get_store_id, get_args) = self.index.get_args(&leaf_ref);
        if get_store_id == 0 {
            return Ok(None);
        }
        let store = self.stores.store_for(get_store_id)
            .ok_or_else(|| ContextError::LoadFailed(
                LoadError::ClientNotFound(get_store_id.to_string())
            ))?;
        let key = get_args.get("key").and_then(|v| {
            if let Tree::Scalar(b) = v { from_utf8(b.as_slice()).ok() } else { None }
        }).ok_or_else(|| ContextError::LoadFailed(
            LoadError::ConfigMissing("key".to_string())
        ))?;
        let args_ref: BTreeMap<&str, Tree> = get_args.iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();
        let value = store.get(key.as_bytes(), &args_ref)
            .ok_or_else(|| ContextError::LoadFailed(
                LoadError::NotFound(key.to_string())
            ))?;

        // write-through to _set
        if set_store_id != 0 {
            if let Some(set_store) = self.stores.store_for(set_store_id) {
                let idx_str = alloc::string::ToString::to_string(&path_idx);
                let sk = set_args.get("key")
                    .and_then(|v| if let Tree::Scalar(b) = v { from_utf8(b.as_slice()).ok() } else { None })
                    .unwrap_or(&idx_str);
                let mut sargs: BTreeMap<&str, Tree> = set_args.iter()
                    .map(|(k, v)| (k.as_str(), v.clone()))
                    .collect();
                sargs.insert("value", value.clone());
                set_store.set(sk.as_bytes(), &sargs);
            }
        }

        self.cache_set(path_idx, value.clone());
        Ok(Some(value))
    }
}
