use core::str::from_utf8;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::dsl::{VALUE_IS_PLACEHOLDER_MASK, VALUE_WORD_ID_MASK};
use crate::index::Index;
use crate::provided::{Context as ContextTrait, ContextError, StoreError, LoadError, Tree};
use crate::debug_log;
use crate::required::{Stores, SetOutcome};

// ── Context ───────────────────────────────────────────────────────────────────

pub struct Context<'r> {
    index:        Arc<Index>,
    stores:       &'r dyn Stores,
    cache_keys:   Vec<u16>,
    cache_vals:   Vec<Tree>,
    called_paths: BTreeSet<u16>,
}

impl<'r> Context<'r> {
    pub fn new(index: Arc<Index>, stores: &'r dyn Stores) -> Self {
        Self {
            index,
            stores,
            cache_keys:   Vec::new(),
            cache_vals:   Vec::new(),
            called_paths: BTreeSet::new(),
        }
    }

    fn cache_get(&self, path_id: u16) -> Option<&Tree> {
        self.cache_keys.iter()
            .position(|&k| k == path_id)
            .and_then(|i| self.cache_vals.get(i))
    }

    fn cache_set(&mut self, path_id: u16, value: Tree) {
        if let Some(i) = self.cache_keys.iter().position(|&k| k == path_id) {
            self.cache_vals[i] = value;
        } else {
            self.cache_keys.push(path_id);
            self.cache_vals.push(value);
        }
    }

    fn cache_remove(&mut self, path_id: u16) {
        if let Some(i) = self.cache_keys.iter().position(|&k| k == path_id) {
            self.cache_keys[i] = u16::MAX;
            self.cache_vals[i] = Tree::Null;
        }
    }

    fn guard_recursion(&self, path_id: u16) -> Result<(), ContextError> {
        if self.called_paths.contains(&path_id) {
            return Err(ContextError::RecursionLimitExceeded);
        }
        Ok(())
    }
}

// ── Context trait impl ────────────────────────────────────────────────────────

impl<'r> ContextTrait for Context<'r> {
    fn get(&mut self, key: &str) -> Result<Option<Tree>, ContextError> {
        debug_log!("Context", "get", key);
        let leaves = self.index.traverse(key);
        if leaves.is_empty() {
            return Err(ContextError::KeyNotFound(key.to_string()));
        }

        if leaves.len() == 1 {
            let leaf = &leaves[0];
            self.guard_recursion(leaf.path_id)?;
            self.called_paths.insert(leaf.path_id);
            let result = self.resolve_leaf(leaf.path_id, leaf.leaf_id, leaf.value_id);
            self.called_paths.remove(&leaf.path_id);
            result
        } else {
            let mut pairs: Vec<(Vec<u8>, Tree)> = Vec::new();
            for leaf in leaves.iter() {
                self.guard_recursion(leaf.path_id)?;
                self.called_paths.insert(leaf.path_id);
                let value = self.resolve_leaf(leaf.path_id, leaf.leaf_id, leaf.value_id)?;
                self.called_paths.remove(&leaf.path_id);
                if let Some(v) = value {
                    let keyword = self.index.keyword_of(leaf.path_id).to_vec();
                    pairs.push((keyword, v));
                }
            }
            Ok(if pairs.is_empty() { None } else { Some(Tree::Mapping(pairs)) })
        }
    }

    fn set(&mut self, key: &str, value: Tree) -> Result<bool, ContextError> {
        debug_log!("Context", "set", key, &crate::debug_log::format_arg(&value));
        let leaves = self.index.traverse(key);
        if leaves.is_empty() {
            return Err(ContextError::KeyNotFound(key.to_string()));
        }
        let leaf = &leaves[0];

        let (store_id, key_frags, _mk, _mv, args_keys, args_vals) = self.index.set_meta(leaf);
        let (key_frags, args_keys, args_vals) = (key_frags.to_vec(), args_keys.to_vec(), args_vals.to_vec());
        let store = self.stores.store_for(store_id)
            .ok_or_else(|| ContextError::StoreFailed(
                StoreError::ClientNotFound(store_id.to_string())
            ))?;

        let id_str = ToString::to_string(&leaf.path_id);
        let store_key = self.resolve_key_frags(&key_frags)?
            .unwrap_or_else(|| id_str.clone());

        let mut owned_args = self.resolve_args(&args_keys, &args_vals)?;
        owned_args.insert("value".to_string(), value.clone());
        let store_args: BTreeMap<&str, Tree> = owned_args.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();

        match store.set(store_key.as_bytes(), &store_args) {
            Some(SetOutcome::Created(_)) | Some(SetOutcome::Updated) => {
                self.cache_set(leaf.path_id, value);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn delete(&mut self, key: &str) -> Result<bool, ContextError> {
        debug_log!("Context", "delete", key);
        let leaves = self.index.traverse(key);
        if leaves.is_empty() {
            return Err(ContextError::KeyNotFound(key.to_string()));
        }
        let leaf = &leaves[0];

        let (store_id, key_frags, _mk, _mv, args_keys, args_vals) = self.index.set_meta(leaf);
        let (key_frags, args_keys, args_vals) = (key_frags.to_vec(), args_keys.to_vec(), args_vals.to_vec());
        let store = self.stores.store_for(store_id)
            .ok_or_else(|| ContextError::StoreFailed(
                StoreError::ClientNotFound(store_id.to_string())
            ))?;

        let id_str = ToString::to_string(&leaf.path_id);
        let store_key = self.resolve_key_frags(&key_frags)?
            .unwrap_or_else(|| id_str.clone());

        let owned_args = self.resolve_args(&args_keys, &args_vals)?;
        let store_args: BTreeMap<&str, Tree> = owned_args.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        let ok = store.delete(store_key.as_bytes(), &store_args);
        if ok {
            self.cache_remove(leaf.path_id);
        }
        Ok(ok)
    }

    fn exists(&mut self, key: &str) -> Result<bool, ContextError> {
        debug_log!("Context", "exists", key);
        let leaves = self.index.traverse(key);
        if leaves.is_empty() {
            return Err(ContextError::KeyNotFound(key.to_string()));
        }
        let leaf = &leaves[0];

        if let Some(v) = self.cache_get(leaf.path_id) {
            debug_log!("Context", "exists", key, "-> cache hit");
            return Ok(!matches!(v, Tree::Null));
        }

        let (store_id, key_frags, _mk, _mv, args_keys, args_vals) = self.index.set_meta(leaf);
        let (key_frags, args_keys, args_vals) = (key_frags.to_vec(), args_keys.to_vec(), args_vals.to_vec());
        let Some(store) = self.stores.store_for(store_id) else {
            return Ok(false);
        };

        let id_str = ToString::to_string(&leaf.path_id);
        let store_key = self.resolve_key_frags(&key_frags)?
            .unwrap_or_else(|| id_str.clone());

        let owned_args = self.resolve_args(&args_keys, &args_vals)?;
        let store_args: BTreeMap<&str, Tree> = owned_args.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        Ok(store.get(store_key.as_bytes(), &store_args).is_some())
    }
}

// ── private helpers ───────────────────────────────────────────────────────────

impl<'r> Context<'r> {
    fn resolve_key_frags(&mut self, key_frags: &[u16]) -> Result<Option<String>, ContextError> {
        if key_frags.is_empty() {
            return Ok(None);
        }
        let mut buf = String::new();
        for &f in key_frags {
            let is_ph   = (f & VALUE_IS_PLACEHOLDER_MASK) != 0;
            let word_id = (f & VALUE_WORD_ID_MASK) as usize;
            let segment = from_utf8(self.index.word_bytes(word_id)).unwrap_or("").to_string();
            if is_ph {
                match self.get(&segment)? {
                    Some(Tree::Scalar(b)) => buf.push_str(from_utf8(&b).unwrap_or("")),
                    Some(_) => {}
                    None => return Err(ContextError::LoadFailed(LoadError::NotFound(segment))),
                }
            } else {
                buf.push_str(&segment);
            }
        }
        Ok(Some(buf))
    }

    fn resolve_args(
        &mut self,
        args_keys: &[u16],
        args_vals: &[u16],
    ) -> Result<BTreeMap<String, Tree>, ContextError> {
        let pairs: Vec<(String, u16)> = args_keys.iter().zip(args_vals.iter())
            .filter_map(|(&key_word_id, &val_values_id)| {
                let k = from_utf8(self.index.word_bytes(key_word_id as usize)).unwrap_or("").to_string();
                if k.is_empty() { return None; }
                Some((k, val_values_id))
            })
            .collect();
        let mut map: BTreeMap<String, Tree> = BTreeMap::new();
        for (k, val_values_id) in pairs {
            let v = if val_values_id == 0 {
                Tree::Null
            } else {
                let frags: Vec<u16> = self.index.values_slice(val_values_id as usize)
                    .unwrap_or(&[]).to_vec();
                match self.resolve_key_frags(&frags) {
                    Ok(Some(s)) => Tree::Scalar(s.into_bytes()),
                    _ => Tree::Null,
                }
            };
            map.insert(k, v);
        }
        Ok(map)
    }

    fn resolve_leaf(&mut self, path_id: u16, leaf_id: u16, value_id: u16) -> Result<Option<Tree>, ContextError> {
        if let Some(v) = self.cache_get(path_id) {
            debug_log!("Context", "resolve_leaf", &alloc::format!("path_id={path_id}"), "-> cache hit");
            return Ok(Some(v.clone()));
        }

        let leaf_ref = crate::index::LeafRef { path_id, leaf_id, value_id };

        let (set_store_id, set_key_frags, _set_map_keys, _set_map_vals, set_args_keys, set_args_vals) = self.index.set_meta(&leaf_ref);
        let set_key_frags: Vec<u16> = set_key_frags.to_vec();
        let set_args_keys: Vec<u16> = set_args_keys.to_vec();
        let set_args_vals: Vec<u16> = set_args_vals.to_vec();
        if set_store_id != 0 {
            if let Some(store) = self.stores.store_for(set_store_id) {
                let id_str = ToString::to_string(&path_id);
                let store_key = self.resolve_key_frags(&set_key_frags)?
                    .unwrap_or_else(|| id_str.clone());
                let owned_args = self.resolve_args(&set_args_keys, &set_args_vals)?;
                let store_args: BTreeMap<&str, Tree> = owned_args.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
                if let Some(value) = store.get(store_key.as_bytes(), &store_args) {
                    debug_log!("Context", "resolve_leaf", &alloc::format!("path_id={path_id}"), "-> _set hit");
                    self.cache_set(path_id, value.clone());
                    return Ok(Some(value));
                }
            }
        }

        let frags: Vec<(bool, Vec<u8>)> = self.index.leaf_fragments(&leaf_ref)
            .into_iter()
            .map(|(is_ph, b)| (is_ph, b.to_vec()))
            .collect();
        if !frags.is_empty() {
            let value = if frags.len() == 1 && frags[0].0 {
                let path_str = from_utf8(&frags[0].1)
                    .map_err(|_| ContextError::LoadFailed(
                        LoadError::ConfigMissing("placeholder utf8".to_string())
                    ))?
                    .to_string();
                self.get(&path_str)?
                    .ok_or_else(|| ContextError::LoadFailed(
                        LoadError::NotFound(path_str.clone())
                    ))?
            } else {
                let mut buf = String::new();
                for (is_ph, bytes) in frags {
                    if is_ph {
                        let path_str = from_utf8(&bytes)
                            .map_err(|_| ContextError::LoadFailed(
                                LoadError::ConfigMissing("placeholder utf8".to_string())
                            ))?;
                        match self.get(path_str)? {
                            Some(Tree::Scalar(b)) => {
                                buf.push_str(from_utf8(&b).unwrap_or(""));
                            }
                            Some(_) => {}
                            None => return Err(ContextError::LoadFailed(
                                LoadError::NotFound(path_str.to_string())
                            )),
                        }
                    } else {
                        buf.push_str(from_utf8(&bytes).unwrap_or(""));
                    }
                }
                Tree::Scalar(buf.into_bytes())
            };
            self.cache_set(path_id, value.clone());
            return Ok(Some(value));
        }

        let (get_store_id, get_key_frags, get_map_keys, get_map_vals, get_args_keys, get_args_vals) = self.index.get_meta(&leaf_ref);
        if get_store_id == 0 {
            return Ok(None);
        }
        let get_key_frags: Vec<u16> = get_key_frags.to_vec();
        let get_map_keys:  Vec<u16> = get_map_keys.to_vec();
        let get_map_vals:  Vec<u16> = get_map_vals.to_vec();
        let get_args_keys: Vec<u16> = get_args_keys.to_vec();
        let get_args_vals: Vec<u16> = get_args_vals.to_vec();

        let store = self.stores.store_for(get_store_id)
            .ok_or_else(|| ContextError::LoadFailed(
                LoadError::ClientNotFound(get_store_id.to_string())
            ))?;

        let id_str = ToString::to_string(&path_id);
        let store_key = self.resolve_key_frags(&get_key_frags)?
            .unwrap_or_else(|| id_str.clone());

        let owned_get_args = self.resolve_args(&get_args_keys, &get_args_vals)?;
        let store_args: BTreeMap<&str, Tree> = owned_get_args.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        let fetched = store.get(store_key.as_bytes(), &store_args)
            .ok_or_else(|| ContextError::LoadFailed(
                LoadError::NotFound(store_key.clone())
            ))?;
        debug_log!("Context", "resolve_leaf", &alloc::format!("path_id={path_id}"), "-> _get hit");

        let value = if !get_map_keys.is_empty() {
            if let Tree::Mapping(fetched_pairs) = &fetched {
                for (&dst_path_id, &src_word_id) in get_map_keys.iter().zip(get_map_vals.iter()) {
                    let src_key = self.index.word_bytes(src_word_id as usize);
                    let fetched_val = fetched_pairs.iter()
                        .find(|(k, _)| k.as_slice() == src_key)
                        .map(|(_, v)| v.clone())
                        .unwrap_or(Tree::Null);
                    self.cache_set(dst_path_id, fetched_val);
                }
            }
            debug_log!("Context", "resolve_leaf", &alloc::format!("path_id={path_id}"), "-> map expanded");
            match self.cache_get(path_id) {
                Some(v) => v.clone(),
                None => return Ok(None),
            }
        } else {
            fetched
        };

        if set_store_id != 0 {
            if let Some(set_store) = self.stores.store_for(set_store_id) {
                let set_store_key = self.resolve_key_frags(&set_key_frags)?
                    .unwrap_or_else(|| id_str.clone());
                let mut sargs_owned = self.resolve_args(&set_args_keys, &set_args_vals)?;
                sargs_owned.insert("value".to_string(), value.clone());
                let sargs: BTreeMap<&str, Tree> = sargs_owned.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
                debug_log!("Context", "resolve_leaf", &alloc::format!("path_id={path_id}"), "-> write-through to _set");
                set_store.set(set_store_key.as_bytes(), &sargs);
                let _ = id_str;
            }
        }

        self.cache_set(path_id, value.clone());
        Ok(Some(value))
    }
}
