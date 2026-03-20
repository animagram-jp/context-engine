# state-engine

Declarative state data management system for a process. 
Structures state data on process and keeps it syncable using your store clients.
It behaves as described in YAML DSL.

- Automates complex state lifecycles through developer-defined YAML manifests.
- Enables multi-tenant DB apps without junction tables.
- Built on a reimagined web architecture (see [## Background](#Background)).

- [also README(patch translation for ja-JP )](./docs/ja/README.md)

## Version

| Version | Status  | Date | description |
|---------|---------|------|-------------|
| 0.1   | Released  | 2026-2-12 | initial | 
| 0.1.4 | Current   | 2026-3-18 | improve #45 | 
| 0.1.5 | Scheduled | 2026-3-22  | improve #46 | 

## Provided Functions

| mod | description | fn |
|-------|------|---------|
| **Manifest** | reads static YAMLs and returns processed obj | `get_value()`, `get_meta()` |
| **State** | operates state data following Manifest | `get()`, `set()`, `delete()`, `exists()` |

## Why state-engine?

**Before:**
```Rust
// Manual cache management
let cache_key = format!("user:{}", id);
let user = redis.get(&cache_key).or_else(|| {
    let user = db.query("SELECT * FROM users WHERE id=?", id)?;
    redis.set(&cache_key, &user, 3600);
    Some(user)
})?;
```

**After:**
```Rust
let user = state.get("cache.user")?;
```

- ✅ Multi-tenant DB without junction tables 
- ✅ Automatic KVS/DB synchronization 
- ✅ No manual cache invalidation 

## Installation

```toml
# Cargo.toml
[dependencies]
state-engine = "0.1"
```

## Quick Start

1. Write a yaml file.

```yaml
# manifest/example.yml
session:
  user-key:
  _state:
    type: integer
  _store:
    client: InMemory
    key: "request-attributes-user-key"
  _load:
    client: InMemory
    key: "request-header-user-key"


user:
  _store:
    client: KVS
    key: "user:${example.session.user-key}"
  _load:
    client: Db
    table: "users"
    where: "id=${example.session.user-key}"
    map:
      name: "name"
  name:
    _state:
      type: string
```

| case | sample |
|------|--------|
| cache in KVS | [cache.yml](./examples/manifest/cache.yml) |
| database connection config | [connection.yml](./examples/manifest/connection.yml) |
| request scope | [session.yml](./examples/manifest/session.yml) |

2. Implement some Required Ports for your stores.

| Interface | expected store | fn | sample |
|-----------|----------------|-----|--------|
| `InMemoryClient` | Local Process Memory | `get()` / `set()` / `delete()` | [InMemoryAdapter](./examples/adapters/in_memory.rs) |
| `EnvClient` | Environment Variables |  as above | [EnvAdapter](./examples/adapters/env_client.rs) |
| `KVSClient` | Key-Vlue Store | as above | [KVSAdapter](./examples/adapters/kvs_client.rs) |
| `DbClient` | SQL Database | as above | [DbAdapter](./examples/adapters/db_client.rs) |
| `HttpClient` | Http Request | as above | [HttpAdapter](./examples/adapters/http_client.rs) |
| `FileClient` | File I/O | as above | [DefaultFileClient](./src/ports/default.rs) |

- FileClient.get is always used by Manifest to read YAMLs.
- It's not essential to implement all *Client.

3. Initialize State with your adapters and use it.

```rust
use state_engine::State;

// Create adapter instances
let in_memory = InMemoryAdapter::new();
let kvs = KVSAdapter::new()?;
let db = DbAdapter::new()?;

// Build State with adapters
let mut state = State::new("./manifest")
    .with_in_memory(&in_memory)
    .with_kvs(&kvs)
    .with_db(&db);

// Use state-engine
let user = state.get("example.user.name")?;
```

Full working example: [examples/app/src/main.rs](./examples/app/src/main.rs)

## Architecture

```
┌─────────────┐  ┌───────────────────┐
│ Application │  │ manifestDir/*.yml │
└──────┬──────┘  └───────────────────┘
       │ uses             ▲ read
       ▼                  │
┌─────────────────────────┴───────────┐
│     Provided Ports (Public API)     │
├─────────────────────────────────────┤
│                                     │
│      State    -->    Manifest       │
│                                     │
└───────┬─────────────────────────────┘
        │ depends on
        ▼
┌─────────────────────────────────────┐
│    Required Ports (App Adapters)    │
├─────────────────────────────────────┤
│    InMemory, KVS, Http,... clients  │
└─────────────────────────────────────┘
```

see for details [Architecture.md](./docs/en/Architecture.md)

## tree

```
./
  README.md
  Cargo.toml

  docs/               # guides
    en/
      Architecture.md
      YAML-guide.md
    ja/
      README.md
      Architecture.md
      YAML-guide.md

  src/
  examples/
    manifest/         # manifest YAML examples
      connection.yml  # sample 1
      cache.yml       # sample 2
      session.yml     # sample 3
    adapters/
    app/
      docker-compose.yml
      Cargo.toml
      Dockerfile
      db/
      src/
        main.rs
        adapters.rs
```

## tests

unit tests, intergeration tests on example app (docker compose) passed

```bashcargo test --features=logging -- --nocapture
cargo test --features=logging -- --nocapture

cd examples/app && ./run.sh
```

## Background

**reimagined web architecture**

```yaml
computer: "A network-capable node in the system."
  orchestrator: "A computer responsible for internal system coordination and maintenance. (optional)"
  server: "A computer that serves human users."
    database: "A server that persists data without an inherent expiration and accepts CRUD operations."
    terminal: "A server that provides a direct human-facing interface."
    conductor: "A server that communicates independently with both a database and terminals, and keeps state data syncable between them. (optional)"
```

## License

MIT