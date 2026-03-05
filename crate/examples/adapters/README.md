# Required Ports Implementations

Implementations of Required Ports are referred to as adapters in Hexagonal Architecture.

**!caution:**
- Do Not Call STATE module in adapters not to cause circular dependency.

## 1. InMemoryAdapter

```Rust
impl InMemoryClient for InMemoryAdapter
```

## 2. KVSAdapter

```Rust
impl KVSClient for KVSAdapter
```

## 3. DbAdapter

```Rust
impl DbClient for DbAdapter
```

## 4. EnvAdapter

```Rust
impl EnvClient for EnvAdapter
```

**!important**: 
You can choose 3 ways of getting `connection: &Value` for your `DbAdapter::fetch()` - 
  1: just a string
  2: state-engine resolved collection 
  3: state-engine resolved collection, and just use connection["configKey"], because your app already has connectionConfig stored in your InMemory or KVS.

```yaml
node:
  _load:
    client: Db
    connection: "connectionName"
```

```yaml
node:
  _load:
    client: Db
    connection: ${connection.tenant} # It means "connection: State::get("connection.tenant")"
```


