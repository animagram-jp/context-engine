# YAML Extended DSL guide

## terms

- `meta keys`: keys prefixed with `_`, along with all keys nested beneath them
- `field keys`: keys that are not meta keys
- `leaf keys`: keys that hold a value instead of child keys
- `value`: a leaf key's value; equals null when omitted in YAML
- `path`: dot-separated key names leading from a start key to the target key
- `qualified path`: a path starting with `filename.`, uniquely identifying a key across all files
- `placeholder`: notation in the form `${path}` that references the result of `State::get()` for the specified key
- `template`: notation that embeds one or more placeholders into a string, such as `"user:${user_id}"`

## rules

- YAML document separators (`---`) are not supported
- `placeholder` and `template` are only valid inside values

## Basic Structure

```yaml
field_key:
  _state: # Data type definition (optional)
  _store: # Where to save (required at root, inherited by children)
  _load:  # Where to load from (optional)
```

## Core Concepts

### 1. meta key inheritance

Each field key inherits parent's meta keys, and can override:

```yaml
_store:
  client: KVS
  key: "root:${id}"

user:
  _store:
    key: "user:${sso_user_id}"  # Override only key, inherit client: KVS

  tenant_id:
    # Inherits _store from parent (client: KVS, key: user:${sso_user_id})
```

### 2. Placeholder Resolution

State engine resolves `${...}` by calling `State::get()`:

```yaml
tenant:
  _load:
    table: "tenants"
    where: "id=${user.tenant_id}"  # → State::get("user.tenant_id")
```

**Placeholder shorthand:**

Whether a path is absolute or relative is determined by whether it contains `.`:

- No `.` → relative path, automatically qualified to `filename.ancestors.path` at parse time
- Contains `.` → treated as absolute path, used as-is

```yaml
# Inside user.tenant_id in cache.yml
key: "${org_id}"            # → cache.user.org_id (relative)
key: "${cache.user.org_id}" # → cache.user.org_id (absolute, same result)
key: "${session.sso_user_id}" # → session.sso_user_id (cross-file reference)
```

**Limitation:** The shorthand (relative path) cannot contain `.`, so to reference a child of a sibling node, use a fully qualified path:

```yaml
# NG: treated as absolute path, KeyNotFound (no filename prefix)
key: "${user.id}"       # → State::get("user.id")

# OK: use fully qualified path
key: "${cache.user.id}" # → State::get("cache.user.id")
```

### 3. Client Types

**For _store** (where to save):
```yaml
_store:
  client: InMemory  # Process memory
  client: KVS       # Redis, Memcached
```

**For _load** (where to load from):
```yaml
_load:
  client: State     # Reference another State key
  client: InMemory  # Process memory
  client: Env       # Environment variables
  client: KVS       # Redis, Memcached
  client: Db        # Database
```

You must implement an adapter for each client you use (see Required Ports).

#### Client-Specific Parameters

**_store.client: InMemory**
```yaml
_store:
  client: InMemory
  key: "session:${token}"            # (string) Storage key (placeholders allowed)
```

**_load.client: Env**
```yaml
_load:
  client: Env
  map:                               # (object, required) Environment variable mapping
    yaml_key: "ENV_VAR_NAME"
```

**_load.client: State**
```yaml
_load:
  client: State
  key: "${org_id}"                   # (string) Reference to another state key
```

**_store.client: KVS**
```yaml
_store:
  client: KVS
  key: "user:${id}"                  # (string) Storage key (placeholders allowed)
  ttl: 3600                          # (integer, optional) TTL in seconds
```

**_load.client: Db**
```yaml
_load:
  client: Db
  connection: ${connection.tenant}  # (Value) Connection config object or reference
  table: "users"                    # (string) Table name
  where: "id=${user.id}"            # (string, optional) WHERE clause
  map:                               # (object, required) Column mapping
    yaml_key: "db_column"
```

## State Methods

**State::get(key)** -> `Result<Option<Value>, StateError>`
- Retrieves value from instance cache / store
- Triggers auto-load on miss if `_load` is defined
- Returns `Ok(Some(value))` on hit, `Ok(None)` on miss with no load, `Err` on error

**State::set(key, value, ttl)** -> `Result<bool, StateError>`
- Saves value to persistent store and instance cache
- Does NOT trigger auto-load
- TTL parameter is optional (KVS only)

**State::delete(key)** -> `Result<bool, StateError>`
- Removes key from both persistent store and instance cache
- Key will show as miss after deletion

**State::exists(key)** -> `Result<bool, StateError>`
- Checks if key exists without triggering auto-load
- Returns `Ok(true/false)`
- Lightweight existence check for conditional logic
