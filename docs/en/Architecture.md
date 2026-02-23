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
  1. DotString
  2. DotMapAccessor
  3. Placeholder
  4. LogFormat

- internal modules
  1. Store
  2. Load

*: *_client impl are not esseintial, optional modules. 

---

## provided modules

Library provides the following modules to handle YAMLs and state data:

1. **Manifest**

A module reading YAML files and returning processed obj. It detects `_` prefix keys (meta keys) and ignores them at `get()`, collects them at `getMeta()`. It converts the key values _load.map.* in the metablock to `'filename.key1.key2.,...,.*'` (absolute path).

  1. Manifest::get('filename.node')

  Read node structure from manifest/*.yml, ignoring `_` prefix keys (meta keys).

  **Behavior:**
  - If the specified node is a leaf, return its value (or null if not defined)
  - Otherwise, return a collection representing all child nodes
  - If the node doesn't exist in YAML (miss), return null

  **Key specification:**
  - `'filename.node'` - Normal specification
  - `'filename'` - Means `'filename.'`, retrieves the entire file root

  2. Manifest::getMeta('filename.node')

  Return metadata blocks for the specified node.

  **Behavior:**
  - Read all metadata blocks from file root to the specified node in order
  - Return a list (map) with child keys overwriting parent keys
  - If the node doesn't exist in YAML (miss), return null

  **Key specification:**
  - `'filename.node'` - Return metadata inherited/overwritten up to the specified node
  - `'filename'` - Means `'filename.*'`, returns only top-level metadata blocks

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

A module performing `get()`/`set()`/`delete()`/`exists()` operations on state data (state obj) stored following the `_store` block provided by Manifest. The `get()` automatically attempts loading based on the description in the `_load` block definition, triggered by key miss hits. The `set()` does not trigger loading, but just set a value obj. `delete()` removes the specified key and all its associated values. The `exists()` checks key existence without triggering auto-load (lightweight check). It caches the state in the instance memory, `State.cache`, as a collection following the structure that YAMLs defined, and keep synced with the state through operation.

## State

### State::get('filename.node')

Reference the state represented by the specified node, returning value or collections.

**Operation flow:**
1. Get metadata via `Manifest::getMeta()`
2. Determine store type from `_store` config (KVS/InMemory)
3. Resolve placeholders (`${session.sso_user_id}`, etc.)
4. Build store key
5. **Check State.cache (a single collection object)** ← Priority
6. Retrieve from store (KVS/InMemoryClient)
7. Extract individual field from data
8. **On miss, auto-load via `Load::handle()`**
9. Return value (no type casting currently implemented)

**Auto-load:**
- If the specified node's state key misses, attempt auto-retrieval via `Load::handle()`
- On `Load::handle()` error, return `None`

**Note on _state.type:**
```yaml
tenant_id:
  _state:
    type: integer  # Metadata only - validation/casting not yet implemented
```

The `_state.type` field is currently metadata-only and not enforced by State operations. Future versions may implement type validation and casting.

---

### State::set('filename.node', value, ttl)

Set a value to the state represented by the specified node.

**Behavior:**
- Save to persistent store (KVS/InMemoryClient)
- Also save to State.cache (for speed)
- If store is KVS, TTL can be set

**TTL behavior:**
- `ttl` argument specified → Use specified value
- No `ttl` argument, `_store.ttl` in YAML → Use YAML default
- No `ttl` argument, no `_store.ttl` in YAML → Maintain current value

---

### State::delete('filename.node')

Delete the {key:value} record represented by the specified node.

**Behavior:**
- Delete from persistent store (KVS/InMemoryClient)
- Also delete from State.cache
- After deletion, the node shows miss hit

---

### State::exists('filename.node')

Check if a key exists without triggering auto-load.

**Behavior:**
- Check State.cache first (fastest)
- Then check persistent store (KVS/InMemoryClient)
- **Does NOT trigger auto-load** (unlike `get()`)
- Returns boolean (true if exists, false otherwise)

**Use case:**
- Lightweight existence check before expensive operations
- Conditional logic without triggering database loads
- Performance-sensitive checks

**Comparison with get():**
- `get()`: Returns value, triggers auto-load on miss
- `exists()`: Returns boolean, never triggers auto-load

---

## required modules

Application must implement the following traits to handle data stores:

1. **InMemoryClient**
  - expected operations: `get()`/`set()`/`delete()`
  - arguments: `'key':...` from `_{store,load}.key:...` in Manifest
  - expected target: Local process memory
  - please mapping eache key arguments to your any memory path
  - remind of State.cache instance memory State always caching regardless of client type.

2. **KVSClient**
  - expected operations: `get()`/`set()`/`delete()`
  - trait signature:
    - `fn get(&self, key: &str) -> Option<String>`
    - `fn set(&mut self, key: &str, value: String, ttl: Option<u64>) -> bool`
    - `fn delete(&mut self, key: &str) -> bool`
  - arguments: `'key':...` from `_{store,load}.key:...`, `ttl:...` from `_{store,load}.ttl:...`(optional) in Manifest
  - expected target: Key-Value Store (Redis, etc.)
  - **Important**: KVSClient handles String only (primitive type). State layer performs serialize/deserialize:
    - **serialize**: All values → JSON string (preserves type information: Number/String/Bool/Null/Array/Object)
    - **deserialize**: JSON string → Value (accurately restores type)
    - **Type preservation**: JSON format distinguishes types (e.g., `42` vs `"42"`, `true` vs `"true"`)
    - KVS stores data as JSON strings. Individual fields are extracted after retrieval.
    - Design intent: Stay faithful to YAML structure while keeping KVS primitive. JSON format ensures type information is preserved without depending on KVS-native types.

3. **DbClient**
  - expected operations: `fetch()`
  - arguments: `'connection':...` from `_{store,load}.connection:...`, `'table':...` from  `_{store,load}.table:...}`, `'columns':...` from `_{store,load}.map.*:...`, `'where_clause':...` from `_{store,load}.where:...`(optional) in Manifest
  - only for _load.client

4. **EnvClient**
  - expected operations: `get()`
  - arguments: `'key':...` from `_{store,load}.map.*:...` in Manifest
  - expected target: environment variables
  - only for _load.client

---

## Load::handle()

When `State::get()` misses a value, retrieve data according to `_store` and `_load` settings from `Manifest::getMeta()`.

**Client types:**
- `Env` - Load from environment variables
- `Db` - Load from database
- `KVS` - Load from KVS
- `InMemory` - Load from process memory
- `State` - Reference another State key (self-reference)

**Special behavior for State client:**
```yaml
tenant_id:
  _load:
    client: State
    key: ${org_id}  # Directly returns State::get('cache.user.org_id')
```

When `_load.client: State`, `Load::handle()` is not called; instead, the value of `_load.key` (with placeholders resolved) is returned directly.

**Design rules:**
- No `_load` → No auto-load, return `None`
- No `_load.client` → No auto-load, return `None`
- `_load.client: State` → Use `_load.key` value directly (don't call Load::handle())
- Other clients → Auto-load via `Load::handle()`

This is an explicit designation to reference another key within State without inheriting the parent's `_load.client`.

**Note:**
- `client == null` is treated as YAML misconfiguration
- The Load::handle() match statement returns null in the default case

**Recursion depth limit:**
- MAX_RECURSION = 10
- Counter incremented with each recursive call
- Throws error when exceeded
- Counter decremented in finally block

---

## State::get() Detailed Flow

```
1. Manifest::getMeta(key) → Get metadata
   ↓
2. Get type info from _state
   ↓
3. Get storage destination from _store (client: KVS/InMemory)
   ↓
4. Resolve placeholders in store_config
   ↓
5. ★ Check in-memory cache (absolute key) ← Highest priority
   if cache.contains_key(key) { return; }
   ↓
6. Build storeKey
   ↓
7. Retrieve value from store (getFromStore)
   ↓
8. Extract individual field from data
   ↓
9. On miss, auto-load
   ├─→ Load::handle(loadConfig)
   │    ├─→ client: Db → DbClient::fetchOne/fetchAll()
   │    ├─→ client: KVS → KVSClient::get()
   │    ├─→ client: Env → EnvClient::get()
   │    ├─→ client: InMemory → InMemoryClient::get()
   │    └─→ client: State → Return specified key value directly (recursion)
   ├─→ Save to persistent store (setToStore)
   └─→ Save to in-memory cache
   ↓
10. Return value
```

---

## State.cache (Instance Memory Cache)

The State struct maintains an instance-level cache (`cache: Value`) separate from persistent stores (KVS/InMemoryClient).

**Important:** This is NOT InMemoryClient. It's a variable of the State instance itself.

**Purpose:**
1. **Speed up duplicate `State::get()` calls within the same request**
2. **Reduce access count to KVS/InMemoryClient**
3. **Design to avoid duplicate loads** (don't load the same key multiple times)

**Check order (important):**
```Rust
// State::get() flow
1. Get metadata
2. Get type info from _state
3. Get storage destination from _store
4. Resolve placeholders
5. ★ Check State.cache (absolute key) ← First check
   if self.cache.contains_key(key) {
       return cast_type(self.cache[key], key);
   }
6. Build storeKey
7. Retrieve from persistent store (KVS/InMemoryClient)
8. On miss, auto-load → After loading, save to State.cache
```

**Cache key:**
- Saved as **absolute path** (`cache.user.tenant_id`)
- Dot notation as-is

**Save timing:**
- On successful load in `State::get()`: `self.cache.insert(key, extracted)`
- On `State::set()`: `self.cache.insert(key, value)`

**Delete timing:**
- On `State::delete()`: `self.cache.remove(key)`

**Lifecycle:**
- State instance creation: Empty
- During State lifetime: Accumulates
- State instance drop: Destroyed (memory released)

**Important design intent:**
- State.cache is checked with higher priority than persistent stores (KVS/InMemoryClient)
- This realizes a design that comprehensively handles external stores
- Even with multiple accesses to the same data, only 1 store access + N HashMap accesses are needed

---

## Placeholder Resolution Rules

Placeholder resolution priority.

**Resolution order:**
1. **Same dictionary reference (relative path)**: `${org_id}` → `cache.user.org_id`
2. **Absolute path**: `${org_id}` → `org_id`

**Example (contextKey: 'cache.user.tenant_id._load.key'):**
```
// Extract dictionary scope
dictScope = 'cache.user'; // Up to before meta key (_load)

// 1. Search within the same dictionary
scopedKey = 'cache.user.org_id';
value = self.get(scopedKey); // → State::get('cache.user.org_id')
if value.is_some() { return value; }

// 2. Search absolute path
return self.get('org_id'); // → State::get('org_id')
```

**Note:**
- Dictionary scope is traced up to meta keys (`_load`, `_store`, etc.) or the last field
- Assumes `cache.user` is a dictionary, `org_id`/`tenant_id` are fields

---

## Field Extraction

When retrieving data, individual fields may need to be extracted.

**extractField logic:**
```Rust
fn extract_field(data: Value, key: &str) -> Value {
    // If not an object, return as-is
    if !data.is_object() {
        return data;
    }

    // Get last segment of key
    // cache.user.id → id
    let segments: Vec<&str> = key.split('.').collect();
    let field_name = segments.last().unwrap();

    // Extract field from dictionary
    data.get(field_name).cloned().unwrap_or(Value::Null)
}
```

---

## Internal Implementation

### Placeholder

Pure string processing (no dependencies).

**Methods:**
- `extract_placeholders(template)` - Extract variable names from template
- `replace(template, params)` - Replace with values
- `resolve_typed(value, resolver)` - Recursively resolve placeholders in JSON value

**Type preservation:**
- Single placeholder and entire string is `${...}` format → Preserve type
- Multiple or within string placeholders → String replacement

### DotMapAccessor

Provides array access with dot notation.

**Methods:**
- `get(data, path)` - Get value with dot notation
- Example: `get(data, "user.profile.name")`