# state-engine 0.0.1

YAML拡張DSLに基づき、プロセスメモリ・KVS・DB間の状態を同期し、データのライフサイクル管理を自動化するライブラリです。(A library that automates data lifecycle management—synchronizing state between process memory, KVS, and databases based on a YAML-extended DSL.)

このライブラリは、データのソースとストアを開発者が自由に記述するYAML拡張DSL(ドメイン特化言語)に従い自動制御します。
例えば、ユーザー単位でプロセスメモリのデータやKVSデータを自動管理し、マルチテナントDBアプリを中間表無しに実現できます。これにより、システムの保守性を大きく向上させます。
state-engineは、#background記載の新たなwebシステム構成を発想の基盤に開発されています。

## manifest YAML拡張DSL

```yaml
# node.yml
node_A:
  _state: #
    type: {integer, float, string, boolean, list, map}
  _store:
    client: {InMemory, ENV, KVS, DB, API}
  _load:
    client: {InMemory, ENV, KVS, DB, API}
  node_1:
    _state:
      ...: # 子要素は親を継承して上書き
    _store:
      ...:
    _load:
      ...:

node_B:
  node_2:
    _state:
      table: 'table-${node_A.node_1}'
```

## background - webシステムの構成再定義

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

## Quick Start

```bash
# run test
docker run --rm -v "$(pwd):/app" -w /app rust:1-alpine cargo test
```

## tree

```
/
  README.md           # このファイル
  CLAUDE.md           # 設計仕様書
  Cargo.toml          # Rust プロジェクト設定
  src/                # ライブラリソースコード
    common/           # 共通ユーティリティ
      dot_array_accessor.rs
      placeholder_resolver.rs
    manifest/         # YAML読み込み
    load/             # 自動ロード
    state/            # State CRUD実装
      parameter_builder.rs
    ports/            # インターフェース定義
      provided.rs     # Manifest, State traits
      required.rs     # Client traits
    lib.rs
  tests/              # テストコード
    mocks/            # モック実装
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

## Architecture

詳細は [CLAUDE.md](./CLAUDE.md) を参照

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

Node.jsサンプルアプリケーション:

```bash
cd samples/app
npm install
npm start
```

詳細は [samples/app/README.md](./samples/app/README.md) を参照
