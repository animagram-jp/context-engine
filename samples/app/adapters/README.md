# Adapters - Required Ports Implementations

このディレクトリには、state-engineの**Required Ports**の実装例が含まれています。

## Required Ports一覧

### 1. ProcessMemoryClient
**実装:** `process_memory.js`

プロセスメモリ内のKey-Value操作を提供します。

```javascript
const ProcessMemoryAdapter = require('./process_memory');
const pm = new ProcessMemoryAdapter();

pm.set('userkey.sso_user_id', 'user123');
const userId = pm.get('userkey.sso_user_id');
pm.delete('userkey.sso_user_id');
```

### 2. ENVClient
**実装:** `env_client.js`

環境変数へのアクセスを提供します。

```javascript
const ENVAdapter = require('./env_client');
const env = new ENVAdapter();

const dbHost = env.get('DB_HOST');
```

### 3. KVSClient
**実装:** `kvs_client.js` (TODO)

Redis等のKVSへのアクセスを提供します。

```javascript
// Example (not implemented yet)
const KVSAdapter = require('./kvs_client');
const kvs = new KVSAdapter({ host: 'localhost', port: 6379 });

await kvs.set('user:123', { id: 123, name: 'John' }, 3600);
const user = await kvs.get('user:123');
```

### 4. DBClient
**実装:** `db_client.js` (TODO)

データベースへのアクセスを提供します。

```javascript
// Example (not implemented yet)
const DBAdapter = require('./db_client');
const db = new DBAdapter(config);

const user = await db.fetchOne('users', 'id=123');
const users = await db.fetchAll('users', 'org_id=100');
```

### 5. APIClient
**実装:** `api_client.js` (TODO)

外部APIへのアクセスを提供します。

```javascript
// Example (not implemented yet)
const APIAdapter = require('./api_client');
const api = new APIAdapter();

const data = await api.get('https://api.example.com/users/123');
await api.post('https://api.example.com/users', { name: 'John' });
```

### 6. ExpressionClient
**実装:** `expression_client.js` (TODO)

app固有の式評価ロジックを提供します。

```javascript
// Example (not implemented yet)
const ExpressionAdapter = require('./expression_client');
const expr = new ExpressionAdapter();

const result = await expr.evaluate('get_tenant_id_from_org(100)');
```

## 実装ガイド

各adapterは以下の責務を持ちます:

1. **インターフェース準拠** - Rust trait定義に従う
2. **エラーハンドリング** - 適切なエラー処理
3. **非同期対応** - 必要に応じてPromise/async-await使用
4. **テスト可能** - モック可能な設計

## 使用例

```javascript
const ProcessMemory = require('./adapters/process_memory');
const ENV = require('./adapters/env_client');

// Setup
const pm = new ProcessMemory();
const env = new ENV();

// Set user context
pm.set('userkey.sso_user_id', 'user123');
pm.set('userkey.tenant_id', 42);

// Load from environment
const dbHost = env.get('DB_HOST');
```
