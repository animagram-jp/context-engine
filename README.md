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
| 0.1.2 | Current   | 2026-2-14 | fix #11 | 
| 0.1.3 | Scheduled | 2026-2-20 | improve #17 | 

## Provided Functions

| Mod | description | fn |
|-------|------|---------|
| **Manifest** | reads static YAMLs and returns processed obj | `get()`, `get_meta()` |
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
# manifest/cache.yml
user:
  _store:
    client: KVS
    key: "user:1"
  _load:
    client: Db
    table: "users"
    where: "id=1"
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
| `KVSClient` | Key-Vlue Store | `get()` / `set()` / `delete()` | [KVSAdapter](./examples/adapters/kvs_client.rs) |
| `DbClient` | SQL Database | `fetch()` | [DbAdapter](./examples/adapters/db_client.rs) |
| `EnvClient` | Environment Variables |  `get()` | [EnvAdapter](./examples/adapters/env_client.rs) |

"Db" and "Env" will be used only in Loading(Read)
It's not essential to implement all *Client.

3. Initialize State with your adapters and use it.

```rust
use state_engine::{State, Load};

// Create adapter instances
let mut in_memory = InMemoryAdapter::new();
let mut kvs = KVSAdapter::new()?;
let db = DbAdapter::new()?;

// Build Load with adapters
let load = Load::new()
    .with_in_memory(&mut in_memory)
    .with_kvs_client(&mut kvs)
    .with_db_client(&db);

// Build State with adapters
let mut state = State::new("./manifest", load)
    .with_in_memory(&mut in_memory)
    .with_kvs_client(&mut kvs);

// Use state-engine
let user = state.get("cache.user.name")?;
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
│    InMemory, KVS, Db, Env clients   │
└─────────────────────────────────────┘
```

see for details [Architecture.md](./docs/en/Architecture.md)

## tree

```
./
  README.md           # this file
  Cargo.toml
  docs/               # guide documents
    en/
      Architecture.md
      YAML-guide.md
    ja/
      README.md
      Architecture.md
      YAML-guide.md
  src/
    ports/            # library external interface traits
      provided.rs     # library provides
      required.rs     # Library requires

    common/           # library common mod (pure logic)
      bit.rs
      pool.rs
      parser.rs
      log_format.rs

    manifest.rs       # Manifest impl
    state.rs          # State impl
    store.rs          # Store internal mod
    load.rs           # Load internal mod

  examples/
    manifest/         # manifest YAML examples
      connection.yml  # sample 1
      cache.yml       # sample 2
      session.yml     # sample 3

    adapters/

    app/
      db/
      src/
      Cargo.toml
      Dockerfile
      docker-compose.yml

  tests/
    mocks/
    integration/
```

## tests

unit tests, intergeration tests and example application test passed

1. cargo test:
```bash
cargo test --features=logging -- --nocapture
```

2. example application test:
```bash
cd examples/app
docker compose up --build
```

## Background

**reimagined web architecture**

- computer: A network-capable node in the system.
- server: A computer that serves human users.
- orchestrator: A computer responsible for internal system coordination and maintenance. (optional)
- database: A server that persists data without an inherent expiration and accepts CRUD operations.
- terminal: A server that provides a direct human-facing interface.
- conductor: A server that communicates independently with both a database and terminals,
  and maintains a synchronized state between them. (optional)

```yaml
# terms relationship
computer:
  orchestrator:
  server:
    database:
    terminal:
    conductor:
```

## License

MIT
