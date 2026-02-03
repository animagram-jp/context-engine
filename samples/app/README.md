# Sample Application

state-engineを使用したサンプルアプリケーションです。

## 概要

このサンプルは、state-engineのコンセプトをNode.jsで実演します。

**注意:** これは概念実証です。実際のプロダクション使用にはRust FFI bindingの実装が必要です。

## セットアップ

### Docker Compose実行（推奨）

PostgreSQL + Redis + アプリを一括起動:

```bash
cd samples/app
docker compose up -d
```

ログ確認:

```bash
docker compose logs -f app
```

停止:

```bash
docker compose down
```

### 環境変数

主な環境変数:
- `DB_HOST` - PostgreSQLホスト（デフォルト: postgres）
- `DB_PORT` - PostgreSQLポート（デフォルト: 5432）
- `DB_DATABASE` - データベース名
- `DB_USERNAME` - DBユーザー名
- `DB_PASSWORD` - DBパスワード
- `REDIS_HOST` - Redisホスト（デフォルト: redis）
- `REDIS_PORT` - Redisポート（デフォルト: 6379）

## 出力例

```
=== state-engine Sample App ===

1. Loading manifests from: /path/to/samples/manifest
   - connection.yml loaded
   - cache.yml loaded

2. Adapters initialized

3. User context set:
   - sso_user_id: user123
   - tenant_id: 42

4. State operations (conceptual):
   connection.common metadata:
   {
     "_state": { "type": "map" },
     "_store": { "client": "InMemory", "key": "connection.common" },
     "_load": { "client": "Env", "map": { ... } }
   }

   cache.user metadata:
   {
     "_state": { "type": "map" },
     "_store": { "client": "KVS", "key": "user:${sso_user_id}", "ttl": 14400 },
     "_load": { "client": "DB", ... }
   }

5. Placeholder resolution:
   Template: user:${sso_user_id}
   Resolved: user:user123

=== Sample completed ===
```

## 構成

```
samples/
  ├── app/
  │   ├── package.json      # Node.js dependencies
  │   ├── index.js          # Main application
  │   └── README.md         # このファイル
  ├── adapters/             # Required Ports implementations
  │   ├── in_memory.js # InMemoryClient
  │   ├── env_client.js     # ENVClient
  │   └── README.md
  └── manifest/             # YAML definitions
      ├── cache.yml         # KVS state definitions
      └── connection.yml    # DB connection definitions
```

## 学習ポイント

1. **YAML定義** - `manifest/`ディレクトリのYAML構造
2. **Adapters実装** - Required Portsの実装パターン
3. **Placeholder解決** - `${variable}`形式のテンプレート処理
4. **State管理** - _state/_store/_loadメタデータの役割

## 次のステップ

実際のプロダクション使用には:

1. **Rust FFI binding** - Node.jsからRustライブラリを呼び出す
2. **完全なAdapter実装** - KVS/DB/API/Expression clients
3. **エラーハンドリング** - 本番環境向けエラー処理
4. **テスト** - 統合テスト・E2Eテスト

## 参考

- [state-engine README](../../README.md)
- [CLAUDE.md](../../CLAUDE.md) - 設計仕様
- [Adapters README](../adapters/README.md) - Required Ports実装ガイド
