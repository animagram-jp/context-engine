# state-engine 0.0.1

Declarative state management for multi-tenant, multi-service systems.
Synchronizes process memory, KVS, and databases using YAML DSL.
開発者が記述するYAML拡張DSL(ドメイン特化言語)を設計図に、高度要件のステートデータを自動管理するライブラリです。

Automates complex state lifecycles through developer-defined YAML manifests.
Enables multi-tenant DB apps without junction tables.
Built on a reimagined web architecture (see [## background](#background)).
このライブラリを導入し、段階的に適切なYAMLとAdapterクラスを整備すれば、例えばマルチテナントDBアプリに中間表が不要になります。
state-engineは、[## background](#background)記載の新たなwebシステム構成を発想元として開発されています。

## Version

- 0.1.0 (2026-2-4)

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

webシステムの構成再定義

よりユーザーの主権を尊重し、資源と責務配置の合理性を追求したwebシステム構成を設計する。

高効率なRust言語とWeb Assembly技術を踏まえて、以下の定義のterminal serverのビジネスロジックへの責務拡大、database serverの認証とCRUD処理への責務拡大を実現。
conductorは中・大規模なシステムにおいてdatabaseとterminalの間を取り持ち、ユニークなDB接続情報などのステートを提供する。

- computer - 電子計算機。ネットワーク通信機能を要するもの。
- server - webシステムを構成するcomputerのうち、機能を人間(ユーザー)に提供するもの
- orchestrator - webシステムを構成するcomputerのうち、システム内部の維持を管理するもの。OPTIONAL
- database - serverのうち、保持期間を定めないデータを維持し、terminalやconductorにCRUDを受け付けるもの
- terminal - serverのうち、人間が直接触るインターフェースを提供するもの。「端末」と同義
- conductor - serverのうち、databaseとterminalの両方に対して相互に通信し、二者の同期通信が成立する状態を維持するもの(OPTIONAL)

```
# 階層図
computer
  orchestrator
  server
    database
    terminal
    conductor
```



## tree

```
/
  README.md
  Cargo.toml          # Rust プロジェクト設定
  docs/               # 各ガイドドキュメント
    DSL-guide.md
  src/
    ports/            # 外部インターフェース定義
      provided.rs     # Manifest, State traits
      required.rs     # Client traits
    common/           # 共通ロジック
      dot_array_accessor.rs
      placeholder_resolver.rs
    manifest/         # YAML読み込み
    state/            # State CRUD実装
      parameter_builder.rs
    load/             # 自動ロード
  tests/
    mocks/
    integration/      # 統合テスト
  samples/            # サンプルコード
    manifest/         # YAML定義サンプル
      connection.yml  # DB接続設定
      cache.yml       # KVSキャッシュ設定
    app/              # Node.js実装サンプル
      index.js        # 使用例
      package.json
    adapters/         # Required Ports実装例
      in_memory.js
      env_client.js
      README.md
```

このライブラリは、YAMLベースの宣言的ステート管理を提供します：

### Provided Ports（提供インターフェース）
- **Manifest** - YAMLファイル読み込み・メタデータ管理
- **State** - 統一CRUD実装（get/set/delete）

### Required Ports（実装必須インターフェース）
- **InMemoryClient** - プロセスメモリ操作
- **ENVClient** - 環境変数取得
- **KVSClient** - KVS操作（Redis等）
- **DBClient** - DB操作
- **APIClient** - 外部API呼び出し

### 内部実装
- **Load** - 自動ロード（_load設定に従いデータ取得）
- **ParameterBuilder** - プレースホルダー値解決
- **PlaceholderResolver** - `${variable}` 形式の置換

## Sample Application

詳細は [samples/app/README.md](./samples/app/README.md) を参照

## License

MIT
