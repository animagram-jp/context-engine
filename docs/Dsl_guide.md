# DSL guide

## terms

- `meta keys`: keys prefixed with `_`, along with all keys nested beneath them
- `field keys`: keys that are not meta keys
- `leaf keys`: keys that hold a value instead of child keys
- `value`: a leaf key's value; equals null when omitted in YAML
- `path`: dot-separated key names leading from a start key to the target key
- `qualified path`: a path starting with `filename.`, uniquely identifying a key across all files
- `placeholder`: notation in the form `${path}` that references the result of `State::get()` for the specified key
- `template`: notation that embeds one or more placeholders into a string, such as `"user:${user_id}"`

## rules

- YAML document separators (`---`) are not supported
- `placeholder` and `template` are only valid inside values

## Basic Structure

```yaml
field_key:
  _state: # Data type definition (optional)
  _store: # Where to save (required at root, inherited by children)
  _load:  # Where to load from (optional)
```

## Core Concepts

### 1. meta key inheritance

Each field key inherits parent's meta keys, and can override:

```yaml
_store:
  client: KVS
  key: "root:${id}"

user:
  _store:
    key: "user:${sso_user_id}"  # Override only key, inherit client: KVS

  tenant_id:
    # Inherits _store from parent (client: KVS, key: user:${sso_user_id})
```

### 2. Placeholder Resolution

State engine resolves `${...}` by calling `State::get()`:

```yaml
tenant:
  _load:
    table: "tenants"
    where: "id=${user.tenant_id}"  # → State::get("user.tenant_id")
```

**Placeholder shorthand:**

Whether a path is absolute or relative is determined by whether it contains `.`:

- No `.` → relative path, automatically qualified to `filename.ancestors.path` at parse time
- Contains `.` → treated as absolute path, used as-is

```yaml
# Inside user.tenant_id in cache.yml
key: "${org_id}"            # → cache.user.org_id (relative)
key: "${cache.user.org_id}" # → cache.user.org_id (absolute, same result)
key: "${session.sso_user_id}" # → session.sso_user_id (cross-file reference)
```

**Limitation:** The shorthand (relative path) cannot contain `.`, so to reference a child of a sibling node, use a fully qualified path:

```yaml
# NG: treated as absolute path, KeyNotFound (no filename prefix)
key: "${user.id}"       # → State::get("user.id")

# OK: use fully qualified path
key: "${cache.user.id}" # → State::get("cache.user.id")
```

### 3. Client Types

**For _store** (where to save):
```yaml
_store:
  client: InMemory  # Process memory
  client: KVS       # Redis, Memcached
  client: HTTP      # HTTP endpoint
```

**For _load** (where to load from):
```yaml
_load:
  client: State     # Reference another State key
  client: InMemory  # Process memory
  client: Env       # Environment variables
  client: KVS       # Redis, Memcached
  client: Db        # Database
  client: HTTP      # HTTP endpoint
```

You must implement an adapter for each client you use (see Required Ports).

#### Client-Specific Parameters

**_store.client: InMemory**
```yaml
_store:
  client: InMemory
  key: "session:${token}"            # (string) Storage key (placeholders allowed)
```

**_load.client: Env**
```yaml
_load:
  client: Env
  map:                               # (object, required) Environment variable mapping
    yaml_key: "ENV_VAR_NAME"
```

**_load.client: State**
```yaml
_load:
  client: State
  key: "${org_id}"                   # (string) Reference to another state key
```

**_store.client: KVS**
```yaml
_store:
  client: KVS
  key: "user:${id}"                  # (string) Storage key (placeholders allowed)
  ttl: 3600                          # (integer, optional) TTL in seconds
```

**_load.client: Db**
```yaml
_load:
  client: Db
  connection: ${connection.tenant}  # (Value) Connection config object or reference
  table: "users"                    # (string) Table name
  where: "id=${user.id}"            # (string, optional) WHERE clause
  map:                               # (object, required) Column mapping
    yaml_key: "db_column"
```

**_store.client: HTTP / _load.client: HTTP**
```yaml
_store:
  client: HTTP
  url: "https://api.example.com/state/${id}"  # (string) Endpoint URL (placeholders allowed)
  headers:                                     # (object, optional) Request headers
    Authorization: "Bearer ${token}"

_load:
  client: HTTP
  url: "https://api.example.com/data/${id}"   # (string) Endpoint URL (placeholders allowed)
  headers:                                     # (object, optional) Request headers
    Authorization: "Bearer ${token}"
  map:                                         # (object, optional) Field extraction from response
    yaml_key: "response_field"
```

## State Methods

**State::get(key)** -> `Result<Option<Value>, StateError>`
- Retrieves value from instance cache / store
- Triggers auto-load on miss if `_load` is defined
- Returns `Ok(Some(value))` on hit, `Ok(None)` on miss with no load, `Err` on error

**State::set(key, value, ttl)** -> `Result<bool, StateError>`
- Saves value to persistent store and instance cache
- Does NOT trigger auto-load
- TTL parameter is optional (KVS only)

**State::delete(key)** -> `Result<bool, StateError>`
- Removes key from both persistent store and instance cache
- Key will show as miss after deletion

**State::exists(key)** -> `Result<bool, StateError>`
- Checks if key exists without triggering auto-load
- Returns `Ok(true/false)`
- Lightweight existence check for conditional logic

---

## Original Text (ja)

### 用語

- `meta keys`: `_`で始まるkey及び、それ以下のkey群
- `field keys`: `meta keys`では無いkey群
- `leaf keys`: 子keyを持たず値を持つkey群
- `value`: leaf keysの値。YAML内で省略された場合はnullが入る
- `path`: 出発keyから対象keyまで、`.`区切りでkey名を並べたパス表現
- `qualified path`: 出発keyを対象keyの記述された`filename.`とした、一意な完全修飾パス
- `placeholder`: ${path}の形で、指定keyのState.get()の結果を参照する記述形式
- `template`: "user${user_id}"の様に、placeholderを文字列に埋め込む記述形式

### rule

- `---`によるYAML区切りは使用不可
- `placeholder`, `template`はvalue内のみで使用可能

### 基本構造

```yaml
field_key:
  _state: # ステートのメタデータ(オプション)
  _store: # 保存先メタデータ (ファイルルートキーで必須, 子孫キーへ継承)
  _load:  # 自動ロード元メタデータ (オプション)
```

### コアコンセプト

### 1. meta key 継承

Each field key inherit parent's meta keys, and can override:

```yaml
_store:
  client: KVS
  key: "root:${id}"

user:
  _store:
    key: "user:${sso_user_id}"  # キーが上書きされる, client: KVSは継承

  tenant_id:
    # client: KVS, key: user:${sso_user_id}を継承
```

#### 2. placeholder 解決

State engineは`${...}`を`State::get()`呼び出しで解決します:

```yaml
tenant:
  _load:
    table: "tenants"
    where: "id=${user.tenant_id}"  # → State::get("user.tenant_id")
```

**placeholderの省略記法:**

Manifestは`${tenant_id}`を`${cache.user.tenant_id}`（絶対パス）に変換します。

`${path}` のパスは、`.` を含むかどうかで絶対/相対が決まります:

- `.` を含まない → 相対パス。parse時に `filename.ancestors.path` へ自動修飾
- `.` を含む → 絶対パスとみなし、そのまま使用

```yaml
# cache.yml の user.tenant_id 内
key: "${org_id}"           # → cache.user.org_id（相対）
key: "${cache.user.org_id}" # → cache.user.org_id（絶対、同じ結果）
key: "${session.sso_user_id}" # → session.sso_user_id（別ファイル参照）
```

**制約:** 省略記法（相対パス）では `.` を使えないため、兄弟ノードの子を参照する場合は完全修飾パスで記述してください。

```yaml
# NG: user.id と書くと絶対パスとみなされ、意図しない参照になる
key: "${user.id}"       # → State::get("user.id") ← ファイル名なし、KeyNotFound

# OK: 完全修飾パスで記述する
key: "${cache.user.id}" # → State::get("cache.user.id")
```

#### 3. クライアント種別

**_store用（保存先）:**
```yaml
_store:
  client: InMemory  # プロセスメモリ
  client: KVS       # Redis, Memcached等
  client: HTTP      # HTTPエンドポイント
```

**_load用（読込元）:**
```yaml
_load:
  client: State     # 別のStateキーを参照
  client: InMemory  # プロセスメモリ
  client: Env       # 環境変数
  client: KVS       # Redis, Memcached等
  client: Db        # データベース
  client: HTTP      # HTTPエンドポイント
```

使用する各クライアントのアダプターを実装する必要があります（Required Ports参照）。

##### クライアント固有のパラメータ

**_store.client: InMemory**
```yaml
_store:
  client: InMemory
  key: "session:${token}"            # (string) ストレージキー（プレースホルダー可）
```

**_load.client: Env**
```yaml
_load:
  client: Env
  map:                               # (object, required) 環境変数マッピング
    yaml_key: "ENV_VAR_NAME"
```

**_load.client: State**
```yaml
_load:
  client: State
  key: "${org_id}"                   # (string) 別のStateキーへの参照
```

**_store.client: KVS**
```yaml
_store:
  client: KVS
  key: "user:${id}"                  # (string) ストレージキー（プレースホルダー可）
  ttl: 3600                          # (integer, optional) TTL（秒）
```

**_load.client: Db**
```yaml
_load:
  client: Db
  connection: ${connection.tenant}  # (Value) 接続設定オブジェクトまたは参照
  table: "users"                    # (string) テーブル名
  where: "id=${user.id}"            # (string, optional) WHERE句
  map:                               # (object, required) カラムマッピング
    yaml_key: "db_column"
```

**_store.client: HTTP / _load.client: HTTP**
```yaml
_store:
  client: HTTP
  url: "https://api.example.com/state/${id}"  # (string) エンドポイントURL
  headers:                                     # (object, optional) リクエストヘッダー
    Authorization: "Bearer ${token}"

_load:
  client: HTTP
  url: "https://api.example.com/data/${id}"   # (string) エンドポイントURL
  headers:                                     # (object, optional) リクエストヘッダー
    Authorization: "Bearer ${token}"
  map:                                         # (object, optional) レスポンスからのフィールド抽出
    yaml_key: "response_field"
```