# state-engine 0.1.0

Declarative state data management for multi-tenant, multi-service systems.
Synchronizes process memory, KVS, and databases using YAML DSL.

- Automates complex state lifecycles through developer-defined YAML manifests.
- Enables multi-tenant DB apps without junction tables.
- Built on a reimagined web architecture (see [## Background](#Background)).

- [also README(partial ja translation)](./docs/ja/README.md)

## Version

- 0.1.0 (2026-2-10) scheduled

## Installation

```toml
# Cargo.toml
[dependencies]
state-engine = "0.1"
```

## Quick Start

0. [install state-engine](#Installation)

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

✅ Multi-tenant DB without junction tables
✅ Automatic KVS/DB synchronization
✅ No manual cache invalidation



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
    ja/
      README.md       # ja translation
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

  tests/
    mocks/
    integration/

  samples/
    manifest/         # manifest YAML samples
      connection.yml  # sample 1
      cache.yml       # sample 2
      session.yml     # sample 3

    adapters/

    app/
      index.js
      package.json
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
