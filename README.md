# state-engine 0.1.0

Declarative state data management for multi-tenant, multi-service systems.
Synchronizes process memory, KVS, and databases using YAML DSL.

- Automates complex state lifecycles through developer-defined YAML manifests.
- Enables multi-tenant DB apps without junction tables.
- Built on a reimagined web architecture (see [## Background](#Background)).

- [also README(patch translation for ja-JP )](./docs/ja/README.md)

## Version

| Version | Status | Date | description |
|---------|--------|------|-------------|
| 0.1.0 |  Scheduled | 2026-2-10 | 1st |

## Provided Functions

| Module | description | methods |
|-------|------|---------|
| **Manifest** | reads static YAMLs and returns processed obj | `get()`, `getMeta()` |
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
some-state:
  _state:
    type: integer
  _store:
    client: KVS
    key: "some"
  _load:
    client: DB
    table: some
    where: 'id=1' # Please update this '$(user.id)' to use this library effectively after.
    columns: 
```

| case | sample |
|------|--------|
| cache in KVS | [cache.yml](.examples/manifest/cache.yml) |
| database connection config | [connection.yml](./examples/manifest/connection.yml) |
| request scope | [session.yml](./examples/manifest/session.yml) |

2. Implement some Required Ports for your stores.

| Interface | expected store | methods | sample |
|-----------|----------------|---------|--------|
| `InMemoryClient` | Local Process Memory | `get()` / `set()` / `delete()` | [InMemoryAdapter](./examples/adapters/in_memory.rs) |
| `KVSClient` | Key-Vlue Store | `get()` / `set()` / `delete()` | [InMemoryAdapter](./examples/adapters/kvs_client.rs) |
| `DBClient` | SQL Database | `fetch()` | [InMemoryAdapter](./examples/adapters/db_client.rs) |
| `ENVClient` | Environment Variables |  `get()` | [InMemoryAdapter](./examples/adapters/env_client.rs) |

'DB' and 'Env' will be used only in Loading(Read)
It's not essential to implement all *Client.

3. Initialize State with your adapters and use it.

```rust
use state_engine::{Manifest, State, Load};

// Initialize Manifest
let mut manifest = Manifest::new('./manifest');

// Create adapter instances
let mut in_memory = InMemoryAdapter::new();
let mut kvs = KVSAdapter::new()?;
let db = DBAdapter::new()?;

// Build Load with adapters
let load = Load::new()
    .with_in_memory(&in_memory)
    .with_kvs_client(&mut kvs)
    .with_db_client(&db);

// Build State with adapters
let mut state = State::new(&mut manifest, load)
    .with_in_memory(&mut in_memory)
    .with_kvs_client(&mut kvs);

// Use state-engine
let user = state.get('some-state')?;
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
│    InMemory, KVS, DB, ENV clients   │
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

    common/           # library common (pure logic modules)
      dot_array_accessor.rs
      placeholder_resolver.rs
      log_format.rs

    manifest/         # Manifest impl
    state/            # State impl
    load/             # Load module (internal module)

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
