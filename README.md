# state-engine

Data labels used by a web system's runtime within a single processing cycle should have their session-context-dependent variations resolved outside of code (e.g., data should be accessible as system_context["session.user"] rather than users[session[user-id]]). state-engine processes for each label, the data retrieval methods that application developers define as a DSL in YAML files. This allows, for example, server/client differences in system_context["session.user.preference"] and multi-tenant differences in context[session.user.tenant] to be resolved appropriately through the data retrieval methods defined in YAML. This OSS is positioned as the foundational technology for the reconstructed web system architecture described in ## background.

- [also README(patch translation for ja-JP )](./docs/ja/README.md)

## Version

| Version | Status  | Date | description |
|---------|---------|------|-------------|
| 0.1   | Released  | 2026-2-12 | initial |
| 0.1.5 | Current   | 2026-3-21  | improve #43 |
| 0.1.6 | Scheduled  | 2026-4-5 | improve #49 #50 |

## Provided Functions

| mod | description | fn |
|-------|------|---------|
| **State** | operates state data following manifest YAMLs | `get()`, `set()`, `delete()`, `exists()` |

## Why state-engine?

**Before:**
```Rust
// Manual cache management
let session_key = format!("user:{}", id);
let user = redis.get(&session_key).or_else(|| {
    let user = db.query("SELECT * FROM users WHERE id=?", id)?;
    redis.set(&session_key, &user, 3600);
    Some(user)
})?;
```

**After:**
```Rust
let user = state.get("session.user")?;
```

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
| `FileClient` | File I/O | as above | [DefaultFileClient](./src/ports/default.rs) |
| `EnvClient` | Environment Variables |  as above | [EnvAdapter](./examples/adapters/env_client.rs) |
| `KVSClient` | Key-Vlue Store | as above | [KVSAdapter](./examples/adapters/kvs_client.rs) |
| `DbClient` | SQL Database | as above | [DbAdapter](./examples/adapters/db_client.rs) |
| `HttpClient` | Http Request | as above | [HttpAdapter](./examples/adapters/http_client.rs) |

- FileClient.get is always used by State to read manifest YAMLs.
- It's not essential to implement all *Client.

3. Initialize State with your adapters and use it.

```rust
use state_engine::State;
use std::sync::Arc;

// Create adapter instances
let in_memory = Arc::new(InMemoryAdapter::new());
let kvs = Arc::new(KVSAdapter::new()?);
let db = Arc::new(DbAdapter::new()?);

// Build State with adapters
let mut state = State::new("./manifest")
    .with_in_memory(in_memory)
    .with_kvs(kvs)
    .with_db(db);

// Use state-engine
let user = state.get("example.user.name")?;
```

Full working example: [examples/app/src/main.rs](./examples/app/src/main.rs)

## Architecture

```
  manifestDir/*.yml
         │ read via FileClient
         ▼
┌─────────────────────────────────────┐
│           State (Public API)        │
└───────┬─────────────────────────────┘
        │ depends on
        ▼
┌─────────────────────────────────────┐
│    Required Ports (App Adapters)    │
├─────────────────────────────────────┤
│  InMemory / File / KVS / DB / HTTP  │
└─────────────────────────────────────┘
        ▲
        │ implement
  Application
```

see for details [Architecture.md](./docs/en/Architecture.md)

## tree

```
state-egnine/
  README.md           # this
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
      connection.yml
      cache.yml
      session.yml
    adapters/
    app/
      docker-compose.yml
      Cargo.toml
      Dockerfile
      db/
      src/
```

## test

Unit tests and integration tests on docker compose

```bash
# unit test
cargo test --features=logging -- --nocapture

# integration tests
cd examples/app && ./run.sh
```

## Background

**reimagined web architecture**

By substituting a portion of human activities with data processing on network-capable computers, we gain benefits such as assurance of verifiability and reduction of physical constraints. The mechanism that realizes this — receiving input as electrical signals through hardware, processing it, and outputting to designated hardware — is called a web system. To realize a web system, it is first necessary to define the conceptual framework it requires in both human language and the language of computer.

```yaml
# computers structure of web system
computer:       "Network-capable nodes in the system."
  server:       "Computers that serves human users."
    fixture:    "Servers that provides continuous network."
    terminal:   "Servers that provides human interfaces."
  orchestrator: "Computers responsible for maintenance of servers. (optional)"
```

## License

Apache-2.0