# YAML Extended DSL guide

## 用語

- `meta keys`: `_`で始まるkey及び、それ以下のkey群
- `field keys`: `meta keys`では無いkey群
- `leaf keys`: 子keyを持たず値を持つkey群
- `value`: leaf keysの値。YAML内で省略された場合はnullが入る
- `path`: 出発keyから対象keyまで、`.`区切りでkey名を並べたパス表現
- `qualified path`: 出発keyを対象keyの記述された`filename.`とした、一意な完全修飾パス
- `placeholder`: ${path}の形で、指定keyのState.get()の結果を参照する記述形式
- `template`: "user${user_id}"の様に、placeholderを文字列に埋め込む記述形式

## rule

- `---`によるYAML区切りは使用不可
- `placeholder`, `template`はvalue内のみで使用可能

## 基本構造

```yaml
field_key:
  _state: # ステートのメタデータ(オプション)
  _store: # 保存先メタデータ (ファイルルートキーで必須, 子孫キーへ継承)
  _load:  # 自動ロード元メタデータ (オプション)
```

## コアコンセプト

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

### 2. placeholder 解決

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

### 3. クライアント種別

**_store用（保存先）:**
```yaml
_store:
  client: InMemory  # プロセスメモリ
  client: KVS       # Redis, Memcached等
```

**_load用（読込元）:**
```yaml
_load:
  client: State     # 別のStateキーを参照
  client: InMemory  # プロセスメモリ
  client: Env       # 環境変数
  client: KVS       # Redis, Memcached等
  client: Db        # データベース
```

使用する各クライアントのアダプターを実装する必要があります（Required Ports参照）。

#### クライアント固有のパラメータ

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
    yaml_key: "Env_VAR_NAME"
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