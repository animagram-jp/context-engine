# Required Ports Implementations

Implementations of Required Ports are referred to as adapters in Hexagonal Architecture.

**!caution:**
- Do Not Call STATE module in adapters not to cause circular dependency.

## 1. InMemoryAdapter

```Rust
impl InMemoryClient for InMemoryAdapter
```

### 2. KVSAdapter

```Rust
impl KVSClient for KVSAdapter
```

### 3. DBAdapter

```Rust
impl DBClient for DBAdapter
```

## 4. ENVAdapter

```Rust
impl ENVClient for ENVAdapter
```

### 4. DBAdapter

```Rust
impl DBClient for DBAdapter
```

**!important**: 
You can choose 3 ways of getting `connection: &Value` for your `DBAdapter::fetch()` - 
  1: just a string
  2: state-engine resolved collection 
  3: state-engine resolved collection, and just use connection['configKey'], because your app already has connectionConfig stored in your InMemory or KVS.

```yaml
node:
  _load:
    client: DB
    connection: 'connectionName'
```

```yaml
node:
  _load:
    client: DB
    connection: ${connection.tenant} # It means "connection: State::get('connection.tenant')"
```


