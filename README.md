# context-engine

[![Crates.io](https://img.shields.io/crates/v/context-engine.svg)](https://crates.io/crates/context-engine)

Data labels used by a web system's runtime within a single processing cycle should have their session-context-dependent variations resolved outside of code (e.g., `system_context["session.user"]` rather than `users[session[user_id]]`). context-engine processes the data retrieval methods that application developers define as a DSL in YAML files, for each label. This allows server/store differences in `system_context["session.user.preference"]` and multi-tenant differences in `context["session.user.tenant"]` to be resolved appropriately through the methods defined in YAML.

---

## Version

| Version | Status    | Date      | Description |
|---------|-----------|-----------|-------------|
| 0.1     | Released  | 2026-2-12 | -           |
| 0.1.6   | Previous  | 2026-4-23 | improve #57 |
| 0.1.7   | Current   | 2026-9-29 | improve #1  |

This project adheres to [Semantic Versioning](https://semver.org/).

---

## Provided Functions

| Mod | Description | fn |
|-----|-------------|----|
| `Context` | operates context | `new/get/set/delete/exists` |

---

## Why context-engine?

**Before:**
```Rust
// Manual cache management
let session_key = format!("user:{}", id);
let user = redis.get(&session_key).or_else(|| {
    let user = db.query("SELECT id, email, name FROM users WHERE id=?", id)?;
    redis.set(&session_key, &user, 3600);
    Some(user)
})?;
```

**After:**
```Rust
let user = state.get("session.user.name")?;
```

---

## Quick Start

1. Add to dependencies.

```toml
# Cargo.toml
[dependencies]
context-engine = "0.1"
```

2. Write a yaml file.

```yaml
# mine.yml
session:
  user:
    id:
      _get:
        store: Memory
        key: "request.authorization.user.id"
    name:
      _get:
        store: Db
        key: "users.${session.user.id}.name"
```

| Case              | Example |
|-------------------|---------|
| multi-tenant app  | [tenant.yml](./examples/tenant.yml) |

3. Implement `Store` and `StoreRegistry` for your stores.

| Trait           | Description                              | Example |
|-----------------|------------------------------------------|---------|
| `Store`   | `get()` `set()` `delete()`               | [TenantDbClient](./examples/implements.rs) |
| `Stores` | maps YAML store names to `Store`s | [MyStore](./examples/implements.rs) |

4. Precompile your yaml to a rs file.

```bash
cargo run --example precompile --features precompile -- examples/mine.yml src/dsl_compiled.rs
# -- <input.yml: required> <output.rs: optional>
# store: values are collected from the yaml in order of first appearance;
# the resulting order (and matching STORE_IDS constant) is printed and baked into the output.
```

5. Initialize Context with your registry.

```rust
use context_engine::{Context, Index};
use std::sync::Arc;

// Include the precompiled static data
include!("generated.rs");

let index = Arc::new(Index::new(
    Box::from(PATHS),
    Box::from(CHILDREN),
    Box::from(LEAVES),
    Box::from(INTERNING),
    Box::from(INTERNING_IDX),
));

let registry = MyRegistry::new();
let mut context = Context::new(index, &registry);

// --- setup completed ---

let user_name = context.get("session.user.name")?;
```

---

## Architecture

```
┌─────────────┐        ┌─────────────────────────────────┐
│ DSL YAML    │------->│ Index (app global instance)     │
└─────────────┘compile └──────────┬──────────────────────┘
                                  │
                                  ▼
┌─────────────┐        ┌─────────────────────────────────┐
│ Application │<-------│ Context (request scope instance)│
└─────────────┘ provide└─────────────────────────────────┘
                                  ▲
                                  │
┌─────────────┐        ┌──────────┴──────────────────────┐
│ StoreImpls  │------->│ Stores (required to impl)       │
└─────────────┘register└─────────────────────────────────┘
```

See for details [Architecture.md](./docs/Architecture.md)

## Test

Passed unit and integration tests

```bash
# unit test
cargo test

# integration test (includes precompile path verification)
RUST_LOG=debug cargo run --example precompile --features precompile -- examples/tenant.yml src/dsl_compiled.rs && RUST_LOG=debug cargo run --example integration_tests --features precompile
```

---

## License

SPDX-License-Identifier: Apache-2.0
Copyright (c) 2026 Andyou <andyou@animagram.jp>