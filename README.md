# state-engine 0.0.1

Declarative state data management for multi-tenant, multi-service systems.
Synchronizes process memory, KVS, and databases using YAML DSL.

- Automates complex state lifecycles through developer-defined YAML manifests.
- Enables multi-tenant DB apps without junction tables.
- Built on a reimagined web architecture (see [## background](#background)).

- [also README(partial ja translation)](./docs/ja/README.md)

## Version

- 0.1.0 scheduled (2026-2-8)

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


## background

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
      provided.rs     # library provides (Manifest, State)
      required.rs     # Library requires (InMemoryClient, DBClient, KVSClient, ENVClient)

    common/           # library common (pure logic)
      dot_array_accessor.rs
      placeholder_resolver.rs
      log_format.rs

    manifest/         # Manifest impl
    state/            # State impl
    load/             # Load module (internal class for State module)

  tests/
    mocks/
    integration/

  samples/
    manifest/         # YAML samples
      connection.yml  # sample 1
      cache.yml       # sample 2
    app/
      index.js
      package.json
    adapters/

```

## Architecture

see [Architecture.md](./docs/en/Architecture.md)

## License

MIT
