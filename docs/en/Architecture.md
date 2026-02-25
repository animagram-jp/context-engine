# Architecture

## index

- provided modules (library provided)
  1. Manifest
  2. State

-  required modules (library required*)
  1. InMemoryClient
  2. KVSClient
  3. DbClient
  4. EnvClient

- common modules (internal common modules)
  1. u64(bit.rs)
  2. Pools & Maps(pool.rs)
  3. ParsedManifest(parser.rs)
  4. LogFormat

- internal modules
  1. Store
  2. Load

*: *_client impl are not essential, optional modules.

---

## provided modules

Library provides the following modules to handle YAMLs and state data:

1. **Manifest**

A module reading YAML files and building fixed-length pool structures. It detects `_` prefix keys (meta keys) and separates them from field keys. Relative placeholders in values are qualified to absolute paths at parse time.

  1. `load(file: &str)` -> `Result<(), ManifestError>`

  Load and parse a manifest file by name. Second call for the same file is a no-op.

  2. `find(file: &str, path: &str)` -> `Option<u16>`

  Look up a key record index by dot-separated path within a file.

  3. `get_meta(file: &str, path: &str)` -> `MetaIndices`

  Return meta record indices (`_load`/`_store`/`_state`) for a node. Collects from root to node; child overrides parent.

  4. `get_value(file: &str, path: &str)` -> `Vec<(u16, u16)>`

  Return leaf value indices for field-key children of a node (meta keys and nulls excluded).

  **Metadata inheritance rules:**
  ```yaml
  # cache.yml
  _store:
    client: KVS
    ttl: 3600

  user:
    _store:
      key: "user:${sso_user_id}"  # Inherits client: KVS, overwrites key

    tenant_id:
      # Inherits _store from parent: client: KVS, key: "user:${sso_user_id}", ttl: 3600
  ```

2. **State**

A module performing `get()`/`set()`/`delete()`/`exists()` operations on state data following the `_store`/`_load` blocks from Manifest. `get()` automatically attempts loading on key miss. `set()` does not trigger loading. `delete()` removes the specified key from both store and cache. `exists()` checks key existence without triggering auto-load. It maintains an instance-level cache (`state_values`) separate from persistent stores.

## State

### State::get("filename.node")

Reference the state represented by the specified node, returning value or collections.

Returns: `Result<Option<Value>, StateError>`

**Operation flow:**
1. Check `called_keys` (recursion / limit detection)
2. `Manifest::load()` → load file (first access only)
3. `Manifest::find()` → get key_idx
4. **Check `state_values` (by key_idx)** ← Highest priority
5. `Manifest::get_meta()` → get MetaIndices
6. If `_load.client == State`: skip store. Otherwise: retrieve from store (KVS/InMemoryClient)
7. On miss, auto-load via `Load::handle()`
8. Return `Ok(Some(value))` / `Ok(None)` / `Err(StateError)`

**Auto-load:**
- If the state key misses, attempt auto-retrieval via `Load::handle()`
- On error, return `Err(StateError::LoadFailed)`

**Note on _state.type:**
```yaml
tenant_id:
  _state:
    type: integer  # Metadata only - validation/casting not yet implemented
```

The `_state.type` field is currently metadata-only and not enforced by State operations.

---

### State::set("filename.node", value, ttl)

Set a value to the state represented by the specified node.

Returns: `Result<bool, StateError>`

**Behavior:**
- Save to persistent store (KVS/InMemoryClient)
- Also save to `state_values` (instance cache)
- If store is KVS, TTL can be set

**TTL behavior:**
- `ttl` argument specified → Use specified value
- No `ttl` argument, `_store.ttl` in YAML → Use YAML default
- No `ttl` argument, no `_store.ttl` in YAML → Maintain current value

---

### State::delete("filename.node")

Delete the {key:value} record represented by the specified node.

Returns: `Result<bool, StateError>`

**Behavior:**
- Delete from persistent store (KVS/InMemoryClient)
- Also delete from `state_values` (instance cache)
- After deletion, the node shows miss

---

### State::exists("filename.node")

Check if a key exists without triggering auto-load.

Returns: `Result<bool, StateError>`

**Behavior:**
- Check `state_values` (instance cache) first
- Then check persistent store (KVS/InMemoryClient)
- **Does NOT trigger auto-load** (unlike `get()`)
- Returns `Ok(true)` if exists, `Ok(false)` otherwise

**Comparison with get():**
- `get()`: Returns value, triggers auto-load on miss
- `exists()`: Returns boolean, never triggers auto-load

---

## required modules

Application must implement the following traits to handle data stores:

1. **InMemoryClient**
  - expected operations: `get()`/`set()`/`delete()`
  - arguments: `"key":...` from `_{store,load}.key:...` in Manifest
  - expected target: Local process memory

2. **KVSClient**
  - expected operations: `get()`/`set()`/`delete()`
  - trait signature:
    - `fn get(&self, key: &str) -> Option<String>`
    - `fn set(&mut self, key: &str, value: String, ttl: Option<u64>) -> bool`
    - `fn delete(&mut self, key: &str) -> bool`
  - arguments: `"key":...` from `_{store,load}.key:...`, `ttl:...` from `_{store,load}.ttl:...`(optional) in Manifest
  - expected target: Key-Value Store (Redis, etc.)
  - **Important**: KVSClient handles String only (primitive type). State layer performs serialize/deserialize:
    - **serialize**: All values → JSON string (preserves type: Number/String/Bool/Null/Array/Object)
    - **deserialize**: JSON string → Value (accurately restores type)

3. **DbClient**
  - expected operations: `fetch()`
  - arguments: `"connection":...`, `"table":...`, `"columns":...` from `_{load}.map.*:...`, `"where_clause":...`(optional)
  - only for `_load.client`

4. **EnvClient**
  - expected operations: `get()`
  - arguments: `"key":...` from `_{load}.map.*:...` in Manifest
  - expected target: environment variables
  - only for `_load.client`

---

## Load::handle()

When `State::get()` misses a value, retrieve data according to `_load` settings.

**Client types:**
- `Env` - Load from environment variables
- `Db` - Load from database
- `KVS` - Load from KVS
- `InMemory` - Load from process memory
- `State` - Reference another State key directly (does not call Load::handle())

**Special behavior for State client:**
```yaml
tenant_id:
  _load:
    client: State
    key: ${org_id}  # Directly returns State::get("cache.user.org_id")
```

When `_load.client: State`, `Load::handle()` is not called; the value of `_load.key` (placeholder already resolved) is returned directly.

**Design rules:**
- No `_load` → No auto-load, return `Ok(None)`
- No `_load.client` → No auto-load, return `Ok(None)`
- `_load.client: State` → Use `_load.key` value directly
- Other clients → Auto-load via `Load::handle()`

**Recursion depth limit:**
- `max_recursion = 20`
- `called_keys: HashSet<String>` tracks keys currently being processed
- On limit exceeded or circular key detected: `Err(StateError::RecursionLimitExceeded)`

---

## State::get() Detailed Flow

```
1. called_keys check (recursion / limit detection)
   ↓
2. Manifest::load() → load file (first access only)
   ↓
3. Manifest::find() → get key_idx
   ↓
4. ★ Check state_values (by key_idx) ← Highest priority
   if find_state_value(key_idx).is_some() { return Ok(Some(value)); }
   ↓
5. Manifest::get_meta() → get MetaIndices
   ↓
6. _load.client == State → skip store
   otherwise: retrieve from store (KVS/InMemoryClient)
   ↓
7. On miss, auto-load
   ├─→ build_config() resolves placeholders
   ├─→ Load::handle(config)
   │    ├─→ client: Db → DbClient::fetch()
   │    ├─→ client: KVS → KVSClient::get()
   │    ├─→ client: Env → EnvClient::get()
   │    └─→ client: InMemory → InMemoryClient::get()
   ├─→ Save to persistent store
   └─→ Save to state_values
   ↓
8. Return Ok(Some(value)) / Ok(None) / Err(StateError)
```

---

## state_values (Instance Memory Cache)

The State struct maintains an instance-level cache (`state_values: StateValueList`) separate from persistent stores (KVS/InMemoryClient).

**Important:** This is NOT InMemoryClient. It is a variable of the State instance itself.

**Purpose:**
1. **Speed up duplicate `State::get()` calls within the same request**
2. **Reduce access count to KVS/InMemoryClient**
3. **Avoid duplicate loads** (don't load the same key multiple times)

**Index:**
- Keyed by `key_idx: u16` — globally unique index in KeyList
- Not keyed by store key string

**Save timing:**
- On successful retrieval from store or load in `State::get()`
- On `State::set()`

**Delete timing:**
- On `State::delete()`

**Lifecycle:**
- State instance created: empty
- During State lifetime: accumulates
- State instance dropped: destroyed (memory released)

---

## Placeholder Resolution Rules

`${}` paths are **qualified to absolute paths at parse time (`Manifest::load()`)** — no conversion happens at State runtime.

**Qualify rule at parse time (`qualify_path()`):**
- Path contains `.` → treated as absolute, used as-is
- No `.` → converted to `filename.ancestors.path`

**Example (`${tenant_id}` in `cache.yml` under `user._load.where`):**
```
qualify_path("tenant_id", "cache", ["user"])
→ "cache.user.tenant_id"
```

**Placeholder resolution at State runtime (`resolve_value_to_string()`):**
- Retrieve qualified path from path_map
- Call `State::get(qualified_path)` to get the value

---

## error case

- Two files with the same name but different extensions (`.yml` and `.yaml`) exist in manifestDir
  - Error timing: when `Manifest::load()` detects both files
  - Reason: Manifest ignores extensions (dot-separated paths represent hierarchy), so it cannot distinguish the two
  - Note: same-extension duplicates are assumed to be prevented at the OS level
