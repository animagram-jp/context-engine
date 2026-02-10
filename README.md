# state-engine 0.1.0

Declarative state data management for multi-tenant, multi-service systems.
Synchronizes process memory, KVS, and databases using YAML DSL.

- Automates complex state lifecycles through developer-defined YAML manifests.
- Enables multi-tenant DB apps without junction tables.
- Built on a reimagined web architecture (see [## Background](#Background)).

- [also README(patch translation for ja-JP )](./docs/ja/README.md)

## Version

| Version | Status | Date | description |
| 0.1.0 |  Scheduled | 2026-2-10 |  |

## Provided Functions

| Module | description | methods |
|-------|------|---------|
| **Manifest** | reads static YAMLs and returns processed obj | `get()`, `getMeta()` |
| **State** | operates state data following Manifest | `get()`, `set()`, `delete()`, `exists()` |

## Why state-engine?

**Before:**
```rust
// Manual cache management
let cache_key = format!("user:{}", id);
let user = redis.get(&cache_key).or_else(|| {
    let user = db.query("SELECT * FROM users WHERE id=?", id)?;
    redis.set(&cache_key, &user, 3600);
    Some(user)
})?;
```

**After:**
```rust
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
    key: "user:${id}"
  _load:
    client: DB
    table: users
```

| case | sample |
| cache in KVS|[cache.yml](.samples/manifest/cache.yml)|
| database connection config |[connection.yml](./samples/manifest/connection.yml)|
| request scope |[session.yml](./samples/manifest/session.yml)|

2. Implement crud modules for your stores.

| Interface | expected store | methods | sample |
|-----------|-------|--|--------|
| `InMemoryClient` | Local Process Memory | `get()` / `set()` / `delete()` | [InMemoryAdapter](./samples/Adapters/in_memory.js) |
| `KVSClient` | Key-Vlue Store | `get()` / `set()` / `delete()` | [InMemoryAdapter](./samples/Adapters/in_memory.js) |
| `DBClient` | SQL Database | `fetch()` | [InMemoryAdapter](./samples/Adapters/db_client.js) |
| `ENVClient` | Environment Variables |  `get()` | [InMemoryAdapter](./samples/Adapters/env_client.js) |

'DB' and 'Env' will be used only in Loading(Read)
It's not essential to implement all *Client.

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

  samples/
    manifest/         # manifest YAML samples
      connection.yml  # sample 1
      cache.yml       # sample 2
      session.yml     # sample 3

    adapters/

    app/
      index.js
      package.json

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
