# state-engine 0.0.1

Declarative state data management for multi-tenant, multi-service systems.
Synchronizes process memory, KVS, and databases using YAML DSL.

- Automates complex state lifecycles through developer-defined YAML manifests.
- Enables multi-tenant DB apps without junction tables.
- Built on a reimagined web architecture (see [## background](#background)).

## Version

- 0.1.0 scheduled (2026-2-8)

## Installation

```toml
# Cargo.toml
[dependencies]
state-engine = "0.1"
```

## Quick Start

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
/
  README.md           # this file
  Cargo.toml
  docs/               # guide documents
    en/
    ja/
      README.md       # ja translation
  src/
    ports/            # external interfaces
      provided.rs     # library provides
      required.rs     # Library requires
    common/           # pure logic utility
      dot_array_accessor.rs
      placeholder_resolver.rs
    manifest/         # Manifest source
    state/            # State source
      parameter_builder.rs
    load/             # internal class
  tests/
    mocks/
    integration/

  samples/
    manifest/         # YAML samples
      connection.yml  # sample 1
      cache.yml       # sample 2
    app/              # sample application
      index.js
      package.json
    adapters/         # sample adapters
      in_memory.js
      env_client.js
      README.md
```

## Architecture

see [Architecture.md](./docs/en/Architecture.md)

## Sample Application

see [samples/app/README.md](./samples/app/README.md)

## License

MIT
