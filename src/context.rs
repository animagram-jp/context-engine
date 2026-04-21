use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::str::from_utf8;

use crate::index::Index;
use crate::provided::{Context as ContextTrait, ContextError, StoreError, LoadError, Tree};
use crate::required::{StoreRegistry, SetOutcome};

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

        let (keyword, map, args) = self.index.set_args(leaf);
        let store = self.registry.store_for(keyword)
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

        match store.set(store_key, &map, &args_ref) {
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

        let (keyword, map, args) = self.index.set_args(leaf);
        let store = self.registry.store_for(keyword)
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

        let ok = store.delete(store_key, &map, &args_ref);
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

        let (keyword, map, args) = self.index.set_args(leaf);
        let Some(store) = self.registry.store_for(keyword) else {
            return Ok(false);
        };

        let idx_str = alloc::string::ToString::to_string(&leaf.path_idx);
        let store_key = args.get("key")
            .and_then(|v| if let Tree::Scalar(b) = v { from_utf8(b.as_slice()).ok() } else { None })
            .unwrap_or(&idx_str);

        let args_ref: BTreeMap<&str, Tree> = args.iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();

        Ok(store.get(store_key, &map, &args_ref).is_some())
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

        // 2. _set
        let (set_name, set_map, set_args) = self.index.set_args(&leaf_ref);
        if !set_name.is_empty() {
            if let Some(store) = self.registry.store_for(set_name) {
                // DSL key省略時はpath_idxを文字列化してstore keyとする（compile時確定・一意）
                let idx_str = alloc::string::ToString::to_string(&path_idx);
                let store_key = set_args.get("key")
                    .and_then(|v| if let Tree::Scalar(b) = v { from_utf8(b.as_slice()).ok() } else { None })
                    .unwrap_or(&idx_str);
                let args_ref: BTreeMap<&str, Tree> = set_args.iter()
                    .map(|(k, v)| (k.as_str(), v.clone()))
                    .collect();
                if let Some(value) = store.get(store_key, &set_map, &args_ref) {
                    self.cache_set(path_idx, value.clone());
                    return Ok(Some(value));
                }
            }
        }

        // 3. value fragments (static scalar / placeholder / template)
        //
        // Evaluated after _set so that a runtime set() — which writes to _set and
        // cache — is always preferred.  Results are cache_set regardless of fragment
        // kind: Context is request-scoped and does not need to track mid-request changes.
        // Collect fragments into owned data to release the immutable borrow on
        // self.index before calling self.get() (which needs &mut self).
        let frags: Vec<(bool, Vec<u8>)> = self.index.leaf_fragments(&leaf_ref)
            .into_iter()
            .map(|(is_ph, b)| (is_ph, b.to_vec()))
            .collect();
        if !frags.is_empty() {
            let value = if frags.len() == 1 && frags[0].0 {
                // Single placeholder: ${path} — resolve via get() and copy as-is
                // (type-preserving; does not stringify).
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
                // Static scalar or template: concatenate all fragments as strings.
                // Placeholders are resolved via get() and stringified.
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

        // 4. _get
        let (get_name, get_map, get_args) = self.index.get_args(&leaf_ref);
        if get_name.is_empty() {
            return Ok(None);
        }
        let store = self.registry.store_for(get_name)
            .ok_or_else(|| ContextError::LoadFailed(
                LoadError::ClientNotFound(get_name.to_string())
            ))?;
        let key = get_args.get("key").and_then(|v| {
            if let Tree::Scalar(b) = v { from_utf8(b.as_slice()).ok() } else { None }
        }).ok_or_else(|| ContextError::LoadFailed(
            LoadError::ConfigMissing("key".to_string())
        ))?;
        let args_ref: BTreeMap<&str, Tree> = get_args.iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();
        let value = store.get(key, &get_map, &args_ref)
            .ok_or_else(|| ContextError::LoadFailed(
                LoadError::NotFound(key.to_string())
            ))?;

        // write-through to _set if configured
        if !set_name.is_empty() {
            if let Some(set_store) = self.registry.store_for(set_name) {
                let idx_str = alloc::string::ToString::to_string(&path_idx);
                let sk = set_args.get("key")
                    .and_then(|v| if let Tree::Scalar(b) = v { from_utf8(b.as_slice()).ok() } else { None })
                    .unwrap_or(&idx_str);
                let mut sargs: BTreeMap<&str, Tree> = set_args.iter()
                    .map(|(k, v)| (k.as_str(), v.clone()))
                    .collect();
                sargs.insert("value", value.clone());
                set_store.set(sk, &set_map, &sargs);
            }
        }

        self.cache_set(path_idx, value.clone());
        Ok(Some(value))
    }
}
