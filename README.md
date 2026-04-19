# context-engine

Data labels used by a web system's runtime within a single processing cycle should have their session-context-dependent variations resolved outside of code (e.g., `system_context["session.user"]` rather than `users[session[user_id]]`). context-engine processes the data retrieval methods that application developers define as a DSL in YAML files, for each label. This allows server/client differences in `system_context["session.user.preference"]` and multi-tenant differences in `context["session.user.tenant"]` to be resolved appropriately through the methods defined in YAML. This library is a foundational technology for the reconstructed web system architecture (see [Background](#background)).

- [See original text(ja)](#original-text-ja)

---

## Version

| Version | Status    | Date      | Description |
|---------|-----------|-----------|-------------|
| 0.1     | Released  | 2026-2-12 | -           |
| 0.1.5   | Current   | 2026-3-21 | improve #43 |
| 0.1.6   | Scheduled | 2026-4-19 | improve #38 |

---

## Provided Functions

| Mod | Description | fn |
|-------|------|---------|
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
      _load:
        client: Memory
        key: "request.authorization.user.id"
    name:
      _load:
        client: Db
        key: "users.${session.user.id}.name"
```

| Case              | Example |
|-------------------|---------|
| multi-tenant app  | [tenant.yml](./examples/tenant.yml) |

3. Implement `Store` and `StoreRegistry` for your stores.

| Trait           | Description                              | Example |
|-----------------|------------------------------------------|---------|
| `Store`   | `get()` `set()` `delete()`               | [DbClient](./examples/implements.rs) |
| `StoreRegistry` | maps YAML client names to `Store`s | [MyRegistry](./examples/implements.rs) |

4. Precompile your yaml to a rs file.

```bash
cargo run --example precompile --features precompile -- examples/mine.yml src/dsl_compiled.rs
# -- <input.yml: required> <output.rs: optional>
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
┌─────────────┐        ┌────────────────────────────────┐
│ DSL YAMLs   │------->│ Index (app global instance) │
└─────────────┘compile └──────────┬─────────────────────┘
                                  │
                                  ▼
┌─────────────┐        ┌────────────────────────────────┐
│ Application │<-------│ Context(request scope instance)│
└─────────────┘ provide└────────────────────────────────┘
                                  ▲
                                  │
┌─────────────┐        ┌──────────┴─────────────────────┐
│ StoreImpls  │------->│ StoreRegistry (required port)  │
└─────────────┘register└────────────────────────────────┘
```

See for details [Architecture.md](./docs/Architecture.md)

## Tree

```
./
  README.md
  Cargo.toml

  docs/
    Architecture.md
    Dsl_guide.md

  src/
    lib.rs
    ports.rs
    ports/
      provided.rs
      required.rs
    debug_log.rs
    tree.rs
    dsl.rs
    index.rs
    context.rs

  examples/
    tenant.yml
    implements.rs
```

## Test

Passed unit and integration tests

```bash
# unit test
cargo test --features=logging -- --nocapture

# integration test (includes precompile path verification)
cargo run --example precompile --features precompile -- examples/tenant.yml src/dsl_compiled.rs && cargo run --example integration_tests --features precompile
```

---

## License

SPDX-License-Identifier: Apache-2.0
Copyright (c) 2026 Andyou <andyou@animagram.jp>

---

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

---

## Original Text (ja)

### 概要

webシステムのランタイムが1回の処理の中で使用するデータのラベルは、セッションコンテクストによる変動を、コード外で処理するべきです(例: users[session[user-id]]では無く、system_context["session.user"]で呼び出せるべき)。context-engineは、アプリ開発者がYAMLファイルにDSLとして定義したデータの取得方法を、ラベルごとに処理します。これにより、例えばsystem_context["session.user.preference"]のサーバー/クライアント差異が、context[session.user.tenant]のマルチテナント差異が、YAML内のデータ取得方法によって、適切に解決されます。このライブラリは、[背景](#背景)記載の、再構成されたwebシステムアーキテクチャの基盤技術に位置付けられています。

### 背景

**webシステムの構成再定義**

人々の営みの一部を、ネットワーク機能を持ったコンピューターのデータ処理で代替することで、検証可能性の保証や物理的制約の緩和といった恩恵を受けます。これを実現する、ハードウェアを通して電気信号として入力を受け取り、処理後、所定のハードウェア群に出力する仕組みのことをwebシステムと呼びます。webシステムの実現には、まず、システムに必要な概念体系を、人間言語とコンピューターのビット列それぞれで定義する必要があります。

```yaml
# computers structure of web system
computer:       "(ネットワーク通信機能を要する)コンピューター"
  server:       "人間(ユーザー・開発者)に処理能力を提供する"
    fixture:    "継続的な待機により、ネットワーク機能を提供する"
    terminal:   "人間とのインターフェースを提供する。端末。"
  orchestrator: "サーバー群の維持を管理する(optional)"
```
