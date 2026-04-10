// This file includes contents that untranslated expressions (ja).

# DSL guide

## Terms

- key:            n層マップDSLの最末端value以外の要素
- keyword:        keyの名前文字列
- field_key:      自身と親祖先のkeywordが'_'で始まらないkey
- meta_key:       keywordが'_'始まりのkeyと、その子孫key (_load, _store, _state)
- leaf_key:       子にkeyを持たず値を持つkey
- value:          leaf_keyの値。DSL内で省略された場合はnullが充てられる
- path:           単一のfield_keyを表す、'.'区切りkeywordのチェーン
- qualified_path: DSL内で一意な完全修飾パス
- placeholder:    key参照記述("${path}")。valueのみに適用。単独記述時はis_template=falseとして扱い、値をそのままコピーする（string化しない）
- template:       placeholderと静的な文字列を混合した動的生成文字列。valueのみに適用。is_template=trueとして扱い、解決時にstring化する
- called_path:    Context.get()等に渡されるパス文字列

## Rules

- YAML document separators (`---`) are not supported
- `${}` (placeholder / template) are only valid inside values
- `${}` paths are always treated as absolute paths

## Basic Structure

```yaml
field_key:
  _store: # Where to save (inherited by descendants, overridable)
  _load:  # Where to load from (optional)
  child_key:
    # inherits _store from parent
```

## Core Concepts

### 1. meta_key Inheritance

Each field_key inherits parent meta_keys and can override individual fields:

```yaml
_store:
  client: Kvs
  key: "root:${id}"

user:
  _store:
    key: "user:${user_id}"  # overrides key only; client: Kvs inherited

  name:
    # inherits _store: { client: Kvs, key: "user:${user_id}" }
```

`_store` / `_load` inheritance rule: child's fields overwrite matching keys; unspecified fields are inherited as-is. Inheritance is resolved at compile time — runtime traversal carries no parent state.

### 2. Placeholder / Template

**is_template:**
- `${path}` alone → `is_template=false`。値をそのままコピー（string化しない）
- `"prefix:${path}"` etc. → `is_template=true`。全placeholderをContext.get()で解決しstring結合

### 3. Reserved keywords

| keyword  | scope         | description |
|----------|---------------|-------------|
| `_load`  | meta_key      | load source definition |
| `_store` | meta_key      | store destination definition |
| `_state` | meta_key      | reserved |
| `client` | _load / _store prop | StoreRegistry keyword |
| `key`    | _load / _store prop | reserved arg passed to StoreClient |
| `map`    | _load / _store prop | field mapping definition |

### 4. _store / _load args

`client:` と `key:` 以外の全フィールドはimplementor定義の任意args。ライブラリは関知しない。

```yaml
_store:
  client: Kvs
  key:    "user:${user.id}"  # reserved
  ttl:    3600               # implementor-defined

_load:
  client:     TenantDb
  key:        "users.id.${session.user.id}"  # reserved
  connection: ${connection.tenant_db}        # implementor-defined
  map:
    name:  "name"
    email: "email"
```

implementorは `args: &BTreeMap<&str, Tree>` から任意キーを取り出して使う。

#### _store.key の省略

`_store.key` は省略可能。省略時、ライブラリはそのleafの `path_idx`（compile時確定のu32）を文字列化してstore keyとして使う。

```yaml
session:
  user:
    _store:
      client: Kvs
      ttl: 3600
      # key: 省略 → path_idxが自動的にstore keyになる
    id:
      _load:
        client: Memory
        key: "request.authorization.user.id"
```

**この設計の根拠:**
- `path_idx`は同じYAMLから生成された全インスタンスで同一（compile時確定）
- qualified_pathを文字列で持たなくてもleaf固有の一意キーとして機能する
- 中間パス（`session.user`）経由のgetでも各leafが自身の`path_idx`を使うため衝突しない

`_load.key` は省略不可。

### 5. map

`_load.map:` / `_store.map:` でparent field_keyの子fieldにストア列等をマッピングする。

```yaml
session:
  user:
    _load:
      client: TenantDb
      key:    "users.id.${session.user.id}"
      map:
        name:  "name"
        email: "email"
    name:
    email:
```

- `map` のvalue（`"name"`, `"email"` 等）がargs経由でclientに渡される
- clientはその順序通りに値を取得して返す責務を持つ
- map対象のfield_keyは別途leaf宣言が必要
