# YAML Extended DSL guide

### Basic Structure

```yaml
node_name:
  _state: # Data type definition (optional)
  _store: # Where to save (required at root, inherited by children)
  _load:  # Where to load from (optional)
```

### Core Concept

#### 1. Metadata inheritance

Child nodes inherit parent's _block, and can override:

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

Path Resolution Rules:

1. Try relative path first: cache.user.tenant_id
2. If not exists, try absolute path: user.tenant_id

#### 3. Client Types

```yaml
_store:
  client: InMemory  # Process memory
_load:
  client: ENV       # Environment variables
  client: KVS       # Redis, Memcached
  client: DB        # Database
  client: API       # External API
```

You must implement adapter for each client (see Required Ports).

### Advanced Examples

```yaml
# example.yml

_store:
  client: # {InMemory, ENV, KVS, DB, API}. Make adapter logic class for each client
_load:
  client: # {InMemory, ENV, KVS, DB, API}

node_A:
  _state: # optional
    type: {integer, float, string, boolean, list, map} # auto-check at State::set() optionally.
  _store: # required at least in file root. It's succeeded to child nodes and can be overrrided by child's same key.
    client: {InMemory, ENV, KVS, DB, API}
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