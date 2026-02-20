# YAML Extended DSL guide

### Basic Structure

```yaml
node_name:
  _state: # Data type definition (optional)
  _store: # Where to save (required at root, inherited by children)
  _load:  # Where to load from (optional)
```

### Core Concept

#### 1. meta key inheritance

Child nodes inherit parent's meta keys, and can override:

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
    where: "id=${user.tenant_id}"  # → State::get('user.tenant_id')
```

**How placeholders are qualified:**

During `Manifest::getMeta()`, relative placeholders are automatically converted to absolute paths:

```yaml
# cache.yml
user:
  org_id:
    _load:
      where: "id=${tenant_id}"  # Relative reference
```

Manifest converts `${tenant_id}` to `${cache.user.tenant_id}` (absolute path).

By the time State sees the placeholder, it's already qualified to an absolute path.

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
  client: DB        # Database
  client: State     # Reference another State key
```

You must implement adapter for each client you use (see Required Ports).

#### Client-Specific Parameters

**_load.client: DB**
```yaml
_load:
  client: DB
  connection: ${connection.tenant}  # (Value) Connection config object or reference
  table: 'users'                    # (string) Table name
  where: 'id=${user.id}'            # (string, optional) WHERE clause
  map:                               # (object, required) Column mapping
    yaml_key: 'db_column'
```

**_load.client: Env**
```yaml
_load:
  client: Env
  map:                               # (object, required) Environment variable mapping
    yaml_key: 'Env_VAR_NAME'
```

**_load.client: State**
```yaml
_load:
  client: State
  key: '${org_id}'                   # (string) Reference to another state key
```

**_store.client: KVS**
```yaml
_store:
  client: KVS
  key: 'user:${id}'                  # (string) Storage key (placeholders allowed)
  ttl: 3600                          # (integer, optional) TTL in seconds
```

**_store.client: InMemory**
```yaml
_store:
  client: InMemory
  key: 'session:${token}'            # (string) Storage key (placeholders allowed)
```

### State Methods

**State::get(key)**
- Retrieves value from cache/store
- Triggers auto-load on miss if `_load` is defined
- Returns the value or None

**State::set(key, value, ttl)**
- Saves value to persistent store and cache
- Does NOT trigger auto-load
- TTL parameter is optional (KVS only)

**State::delete(key)**
- Removes key from both persistent store and cache
- Key will show as miss after deletion

**State::exists(key)**
- Checks if key exists without triggering auto-load
- Returns boolean (true/false)
- Lightweight existence check for conditional logic

### Advanced Examples

```yaml
# example.yml

_store:
  client: # {InMemory, KVS}. Make adapter logic class for each client
_load:
  client: # {Env, InMemory, KVS, DB, State}

node_A:
  _state: # optional, meta key only (type validation not yet implemented)
    type: {integer, float, string, boolean, list, map}
  _store: # required at least in file root. Inherited by child nodes, can be overridden.
    client: {InMemory, KVS}  # Only InMemory and KVS are valid for _store
  _load:
    client: DB
    connection: ${connection.tenant} # reserved ${} means State::get(). State try 'example.node_A.connection.tenant'(relative path) 1st and if not exists, 'connection.tenant'(absolute path) 2nd.
    table: 'table_A'
    map: # It can load multiple nodes once following YAML coding. Be attention for optimization and unintended loading
      node_1: 'node_1'
      node_2: 'node_2'
  node_1:
    _state:
      ...:
    _store:
      ...:
    _load:
      map:
        node

  node_2: # if no need extra data, this is optional
    _state:
      type: string
  node_3:
    _load:
      key: ${node_1} # It means State::get('example.node_A.node_1') (If not exist, State try 'node_1' 2nd)

node_B:
  node_2:
    _load:
      client: DB
      table: 'table-${example.node_A.node_1}' # It means State::get{'example.node_A.node_1'} (State try 'example.node_B.example.node_A.node_1' 1st)
    _store:
...:
```