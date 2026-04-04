# アーキテクチャ

## ライブラリ要件

- README 3行目参照
- システムが認識するべき概念を階層構造の名前空間で表現できたとする。この時、名前空間から導かれる全通りの(部分含む)パスが、ランタイムの単一処理スコープで操作する可能性のある値のキーを網羅している。このキー群の値全てを、DSLにて漏れなく取得方法の定義を宣言する。

## 機能構成

- parse & compile: DSLを読み込み、n次元疎集合割り出しの最適解である、固定長メモリ位置群のトラバーサルに落とし込むための静的データ群を生成する
- traversal: 上記データ群を保持し、トラバーサルによってメモリ位置群を取得する
- addressing & operation: Manifestに対応した1層mapを保持し、アプリケーションからの呼び出しに応じて値の操作を行う。リクエスト処理スコープインスタンス。

## モジュール構成

| mod | description | ports |
|-------|------|---------|
| Dsl | DSLを読み込み、n次元疎集合割り出しの最適解である、固定長メモリ位置群のトラバーサルに落とし込むための静的データ群を生成する | new(Vec<(u64, u32)>),compile(&[&Path]) |
| Index | Dsl:compile(DSL)を呼び出し、アドレスリスト(Box<(u64, u32)>)を保持し、トラバーサルによってメモリ位置群を取得する | `traverse()` |
| Context  | operates state data following manifest YAMLs | `traverse()` |

- provided modules (library provided)
  1. State

- required modules (library required*)
  1. StoreClient
  2. StoreRegistry

- internal modules
  1. core::Manifest
  2. Store
  3. Load
  4. u64(fixed_bits.rs)
  5. Pools & Maps(pool.rs)
  6. parser.rs
  7. LogFormat

*: optional。FileClientのデフォルト実装のみ内蔵。

---

## provided modules

**State** is the sole public API of the library.

A module performing `get()`/`set()`/`delete()`/`exists()` operations on state data following the `_store`/`_load` blocks defined in manifest YAMLs. `get()` automatically attempts loading on key miss. `set()` does not trigger loading. `delete()` removes the specified key from both store and cache. `exists()` checks key existence without triggering auto-load. It maintains an instance-level cache (`state_values`) separate from persistent stores.

State owns YAML I/O: it reads manifest files via the built-in `DefaultFileClient` and parses them into `core::Manifest` on first access. `core::Manifest` is an internal no_std struct that owns all bit-record data and provides decode/find/build_config queries. Relative placeholders in values are qualified to absolute paths at parse time. Metadata (`_store`/`_load`/`_state`) is inherited from parent nodes; child overrides parent.

## State

### State::get("filename.node")

Reference the state represented by the specified node, returning value or collections.

Returns: `Result<Option<Value>, StateError>`

**Operation flow:**
1. Check `called_keys` (recursion / limit detection)
2. Load manifest file via `DefaultFileClient` (first access only)
3. Traverse intern list with path string → locate key
4. **Check `state_values` (by path)** ← Highest priority
5. `core::Manifest::get_meta()` → get MetaIndices
6. If `_load.client == State`: skip store. Otherwise: `StoreRegistry::client_for(yaml_name)` → `StoreClient::get()`
7. On miss, auto-load via `Load::handle()`
8. Return `Ok(Some(value))` / `Ok(None)` / `Err(StateError)`

**Auto-load:**
- If the state key misses, attempt auto-retrieval via `Load::handle()`
- On error, return `Err(StateError::LoadFailed(LoadError))`

**Note on _state.type:**
```yaml
tenant_id:
  _state:
    type: integer  # Metadata only - validation/casting not yet implemented
```

The `_state.type` field is currently metadata-only and not enforced by State operations.

---

### State::set("filename.node", value, ttl)

Set a value to the state represented by the specified node.

Returns: `Result<bool, StateError>`

**Behavior:**
- Save via `StoreRegistry::client_for(yaml_name)` → `StoreClient::set()`
- Also save to `state_values` (instance cache)
- TTL and other store-specific args are handled by the `StoreClient` impl

---

### State::delete("filename.node")

Delete the {key:value} record represented by the specified node.

Returns: `Result<bool, StateError>`

**Behavior:**
- Delete via `StoreRegistry::client_for(yaml_name)` → `StoreClient::delete()`
- Also delete from `state_values` (instance cache)
- After deletion, the node shows miss

---

### State::exists("filename.node")

Check if a key exists without triggering auto-load.

Returns: `Result<bool, StateError>`

**Behavior:**
- Check `state_values` (instance cache) first
- Then check via `StoreClient::get()`
- **Does NOT trigger auto-load** (unlike `get()`)
- Returns `Ok(true)` if exists, `Ok(false)` otherwise

---

## required modules

### StoreClient

単一ストアの操作を提供するtrait。`key`は予約引数として明示し、追加の任意引数は`args`のflatなHashMapで渡す。

```rust
pub trait StoreClient: Send + Sync {
    fn get(&self, key: &str, args: &HashMap<&str, Value>) -> Option<Value>;
    fn set(&self, key: &str, args: &HashMap<&str, Value>) -> bool;
    fn delete(&self, key: &str, args: &HashMap<&str, Value>) -> bool;
}
```

- `key`: manifest の `_{store,load}.key` の値。予約引数。
- `args`: ttl・connection・headers 等、ストア種別ごとの任意引数。利用者がimpl内で定義・参照する。
- 内部可変性・スレッド安全性はimplementor側の責任。

### StoreRegistry

YAMLの`client:`名称とStoreClientの対応を管理するtrait。利用者がimplし、Stateに渡す。

```rust
pub trait StoreRegistry {
    fn client_for(&self, yaml_name: &str) -> Option<&dyn StoreClient>;
}
```

- ライブラリはYAML名称の文字列をそのまま`client_for()`に渡してmatchを回す。
- YAML上の名義（`"Memory"`, `"KVS"`, `"Db"`等）は利用者が自由に定義する。

**実装例:**
```rust
struct MyStores {
    memory: Arc<MemoryImpl>,
    kvs:    Arc<KvsImpl>,
    db:     Arc<DbImpl>,
}

impl StoreRegistry for MyStores {
    fn client_for(&self, yaml_name: &str) -> Option<&dyn StoreClient> {
        match yaml_name {
            "Memory" => Some(self.memory.as_ref()),
            "KVS"    => Some(self.kvs.as_ref()),
            "Db"     => Some(self.db.as_ref()),
            _        => None,
        }
    }
}
```

---

## Load::handle()

When `State::get()` misses a value, retrieve data according to `_load` settings.

`_load.client` の値を `StoreRegistry::client_for()` に渡し、対応する `StoreClient::get()` を呼ぶ。

**Special behavior for State client:**
```yaml
tenant_id:
  _load:
    client: State
    key: ${org_id}  # Directly returns State::get("cache.user.org_id")
```

When `_load.client: State`, `Load::handle()` is not called; the value of `_load.key` (placeholder already resolved) is returned directly.

**Design rules:**
- No `_load` → No auto-load, return `Ok(None)`
- No `_load.client` → No auto-load, return `Ok(None)`
- `_load.client: State` → Use `_load.key` value directly
- Other clients → `StoreRegistry::client_for(yaml_name)` → `StoreClient::get()`

**Recursion depth limit:**
- `max_recursion = 20`
- `called_keys: HashSet<String>` tracks keys currently being processed
- On limit exceeded or circular key detected: `Err(StateError::RecursionLimitExceeded)`

---

## state_values (Instance Memory Cache)

The State struct maintains an instance-level cache (`state_values`) separate from persistent stores.

**Important:** This is NOT a StoreClient. It is a variable of the State instance itself.

**Purpose:**
1. Speed up duplicate `State::get()` calls within the same request
2. Reduce access count to stores
3. Avoid duplicate loads

**Lifecycle:**
- State instance created: empty
- During State lifetime: accumulates
- State instance dropped: destroyed (memory released)

---

## Placeholder Resolution Rules

`${}` paths are **qualified to absolute paths at parse time** — no conversion happens at State runtime.

**Qualify rule at parse time (`qualify_path()`):**
- Path contains `.` → treated as absolute, used as-is
- No `.` → converted to `filename.ancestors.path`

**Example (`${tenant_id}` in `cache.yml` under `user._load.where`):**
```
qualify_path("tenant_id", "cache", ["user"])
→ "cache.user.tenant_id"
```

**Placeholder resolution at State runtime (`resolve_value_to_string()`):**
- Call `State::get(qualified_path)` to get the value

---

## error case

**ManifestError:**
- `FileNotFound` — manifest file not found in manifest dir
- `AmbiguousFile` — two files with the same name but different extensions (`.yml` and `.yaml`) exist in manifestDir
- `ParseError` — YAML parse failed

**LoadError:**
- `ClientNotFound(String)` — `StoreRegistry::client_for()` returned `None` for the given yaml_name
- `ConfigMissing(String)` — a required config key is missing in the manifest
- `NotFound(String)` — the client call succeeded but returned no data
- `ParseError(String)` — parse error from client response

**StoreError:**
- `ClientNotFound(String)` — `StoreRegistry::client_for()` returned `None` for the given yaml_name
- `ConfigMissing(String)` — a required config key is missing in the manifest
- `SerializeError(String)` — serialize error

---

## Original Text (ja)

### index

- provided modules (ライブラリ提供モジュール)
  1. State

- required modules (ライブラリ要求モジュール*)
  1. StoreClient
  2. StoreRegistry

- internal modules (内部モジュール)
  1. core::Manifest
  2. Store
  3. Load
  4. u64(fixed_bits.rs)
  5. Pools & Maps(pool.rs)
  6. parser.rs
  7. LogFormat

*: いずれもoptional(必須ではない)。FileClientのデフォルト実装のみ内蔵。

---

## Ports

ライブラリの外部向けインターフェース定義modules

1. Provided Port

**State** がライブラリ唯一の公開APIです。

manifest YAMLの`_store`/`_load`定義に従い、`get()` / `set()` / `delete()` / `exists()`操作を提供するmoduleです。`get()`はkey missをトリガーに`_load`定義に基づいて自動ロードを試みます。`set()`は自動ロードを引き起こしません。`delete()`はストアとインスタンスキャッシュ両方から削除します。`exists()`は自動ロードを引き起こさずにkey存在確認を行います。

2. Required Ports

ライブラリ動作時にimpl実装が必要なtraits

**StoreClient**

単一ストアのget/set/deleteを提供するtrait。`key`は予約引数。`args`にttl等の任意引数をflatなHashMapで渡す。内部可変性はimplementor側の責任。

**StoreRegistry**

YAMLの`client:`文字列と`StoreClient`の対応を管理するtrait。利用者がimplしてStateに渡す。ライブラリ側はYAML名を`client_for()`に渡してdispatchする。YAML上の名義は利用者が自由に定義できる。

## State

### State::get("filename.node")

指定されたノードが表すステートを参照し、値またはcollectionを返却する。

戻り値: `Result<Option<Value>, StateError>`

**動作フロー:**
1. `called_keys` チェック（再帰・上限検出）
2. `DefaultFileClient`経由でmanifestファイルをロード（未ロード時のみ）
3. intern listをパス文字列で検索・トラバース → key位置を特定
4. **state_values (インスタンスキャッシュ) をチェック** ← 最優先
5. `core::Manifest::get_meta()` → MetaIndices 取得
6. `_load.client == State` の場合はストアをスキップ。それ以外: `StoreRegistry::client_for(yaml_name)` → `StoreClient::get()`
7. **miss時、`Load::handle()` で自動ロード**
8. `Ok(Some(value))` / `Ok(None)` / `Err(StateError)` を返却

---

### State::set("filename.node", value, ttl)

指定されたノードが表すステートに値をセットする。

戻り値: `Result<bool, StateError>`

**動作:**
- `StoreRegistry::client_for(yaml_name)` → `StoreClient::set()` で保存
- state_values (インスタンスキャッシュ) にも保存
- ttl等のストア固有引数はStoreClient impl側で管理

---

### State::delete("filename.node")

指定されたノードが表す {key:value} レコードを削除する。

戻り値: `Result<bool, StateError>`

---

### State::exists("filename.node")

自動ロードをトリガーせずに、キーの存在確認を行う。

戻り値: `Result<bool, StateError>`

---

## Load::handle()

`State::get()` が値をmissした際、`_load` 設定に従ってデータを取得する。

`_load.client` の値を `StoreRegistry::client_for()` に渡し、対応する `StoreClient::get()` を呼ぶ。

**設計ルール:**
- `_load` なし → 自動ロードなし、`Ok(None)` を返す
- `_load.client` なし → 自動ロードなし、`Ok(None)` を返す
- `_load.client: State` → `_load.key` の値を直接使用
- その他のclient → `StoreRegistry::client_for(yaml_name)` → `StoreClient::get()`

**再帰深度制限:**
- `max_recursion = 20`
- 上限超過または同一キーの再帰検出時に `Err(StateError::RecursionLimitExceeded)` を返す

---

## error case

**ManifestError:**
- `FileNotFound` — manifestディレクトリにファイルが見つからない
- `AmbiguousFile` — manifestDir内に拡張子違いの同名ファイルが2つ存在する
- `ParseError` — YAMLのパース失敗

**LoadError:**
- `ClientNotFound(String)` — `StoreRegistry::client_for()` が None を返した
- `ConfigMissing(String)` — manifest内に必須のconfigキーが欠落
- `NotFound(String)` — clientの呼び出しは成功したがデータが存在しなかった
- `ParseError(String)` — clientレスポンスのパースエラー

**StoreError:**
- `ClientNotFound(String)` — `StoreRegistry::client_for()` が None を返した
- `ConfigMissing(String)` — manifest内に必須のconfigキーが欠落
- `SerializeError(String)` — シリアライズエラー
