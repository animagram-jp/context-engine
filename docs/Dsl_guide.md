# DSL guide

## 用語

```
key:            n層マップDSLの最末端value以外の要素
keyword:        keyの名前文字列
field_key:      自身と親祖先のkeywordが'_'で始まらないkey
meta_key:       keywordが'_'始まりのkeyと、その子孫key
leaf_key:       子にkeyを持たず値を持つkey
value:          leaf_keyの値。DSL内で省略された場合はnullが充てられる
path:           単一のfield_keyを表す、'.'区切りkeywordのチェーン
qualified_path: DSL内で一意な完全修飾パス
placeholder:    key参照記述("${path}")。valueのみに適用。
                単独記述時はis_template=falseとして扱い、値をそのままコピーする（string化しない）
template:       placeholderと静的な文字列を混合した動的生成文字列。valueのみに適用。
                is_template=trueとして扱い、解決時にstring化する
called_path:    Context.get()等に渡されるパス文字列
```

## Rules

- YAML document separators (`---`) are not supported
- `${}` (placeholder / template) are only valid inside values

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

`_store` inheritance rule: child's `_store` fields overwrite matching keys; unspecified fields are inherited as-is.

### 2. Placeholder / Template

`${}` paths are qualified to absolute paths at parse time.

**Qualify rule (`qualify_path()`):**
- No `.` → relative; converted to `filename.ancestors.keyword` at parse time
- Contains `.` → treated as absolute, used as-is

```yaml
# Inside tenant.yml under session.user._load
key: "${session.user.id}"     # absolute — used as-is
key: "${id}"                  # relative → tenant.session.user.id
```

**is_template:**
- `${path}` alone → `is_template=false`。値をそのままコピー（string化しない）
- `"prefix:${path}"` etc. → `is_template=true`。全placeholderをContext.get()で解決しstring結合

### 3. _store / _load args

`client:` 以外の全フィールドはimplementor定義の任意args。ライブラリは関知しない。

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

`key:` は予約引数。それ以外はimplementorが`args: &HashMap<&str, Tree>`から取り出して使う。

### 4. map

`_load.map:` でparent field_keyの子fieldにDB列等をマッピングする。

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

map対象のfield_keyは別途leaf宣言が必要。
