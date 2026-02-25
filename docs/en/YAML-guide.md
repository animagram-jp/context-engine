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

### Basic Structure

```yaml
node_name:
  _state: # Data type definition (optional)
  _store: # Where to save (required at root, inherited by children)
  _load:  # Where to load from (optional)
```

### Core Concept

#### 1. meta key inheritance

Child nodes inherit parent"s meta keys, and can override:

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

#### 2. Placeholder Resolution

State engine resolves ${...} by calling State::get():

```yaml
tenant:
  _load:
    table: "tenants"
    where: "id=${user.tenant_id}"  # → State::get("user.tenant_id")
```

**How placeholders are qualified:**

At parse time (`Manifest::load()`), relative placeholders are automatically converted to absolute paths:

```yaml
# cache.yml
user:
  org_id:
    _load:
      where: "id=${tenant_id}"  # Relative reference
```

Manifest converts `${tenant_id}` to `${cache.user.tenant_id}` (absolute path).

By the time State resolves the placeholder, it is already a qualified absolute path.

#### 3. Client Types

**For _store** (where to save):
```yaml
_store:
  client: InMemory  # Process memory
  client: KVS       # Redis, Memcached
```

**For _load** (where to load from):
```yaml
_load:
  client: Env       # Environment variables
  client: InMemory  # Process memory
  client: KVS       # Redis, Memcached
  client: Db        # Database
  client: State     # Reference another State key
```

You must implement adapter for each client you use (see Required Ports).

#### Client-Specific Parameters

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

**_load.client: Env**
```yaml
_load:
  client: Env
  map:                               # (object, required) Environment variable mapping
    yaml_key: "Env_VAR_NAME"
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

**_store.client: InMemory**
```yaml
_store:
  client: InMemory
  key: "session:${token}"            # (string) Storage key (placeholders allowed)
```

### State Methods

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

### Advanced Examples

```yaml
# example.yml

_store:
  client: # {InMemory, KVS}. Make adapter logic class for each client
_load:
  client: # {Env, InMemory, KVS, Db, State}

node_A:
  _state: # optional, meta key only (type validation not yet implemented)
    type: {integer, float, string, boolean, list, map}
  _store: # required at least in file root. Inherited by child nodes, can be overridden.
    client: {InMemory, KVS}  # Only InMemory and KVS are valid for _store
  _load:
    client: Db
    connection: ${connection.tenant} # ${} means State::get(). Qualified to absolute path at parse time.
    table: "table_A"
    map: # multiple fields can be loaded at once. Be careful about optimization and unintended loading.
      node_1: "node_1"
      node_2: "node_2"
  node_1:
    _state:
      ...:
    _store:
      ...:

  node_2: # optional if no extra meta is needed
    _state:
      type: string
  node_3:
    _load:
      key: ${node_1} # qualified to "example.node_A.node_1" at parse time → State::get("example.node_A.node_1")

node_B:
  node_2:
    _load:
      client: Db
      table: "table-${example.node_A.node_1}" # contains '.', treated as absolute path → State::get("example.node_A.node_1")
    _store:
...:
```