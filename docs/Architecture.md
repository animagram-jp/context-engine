# アーキテクチャ

## ライブラリ要件

- README 3行目参照
- システムが認識するべき概念を階層構造の名前空間で表現できたとする。この時、名前空間から導かれる全通りの(部分含む)パスが、ランタイムの単一処理スコープで操作する可能性のある値のキーを網羅している。このキー群の値全てを、DSLにて漏れなく取得方法の定義を宣言する。

## 機能構成

- parse & compile: DSLを読み込み、n次元疎集合割り出しの最適解である、固定長メモリ位置群のトラバーサルに落とし込むための静的データ群を生成する
- toraversal: 上記データ群を保持し、トラバーサルによってメモリ位置群を取得する
- adressing & operation: Manifestに対応した1層mapを保持し、アプリケーションからの呼び出しに応じて値の操作を行う。リクエスト処理スコープインスタンス。

## モジュール構成

- Dsl: 
- Manifest: fn 
- State

| mod | description | ports |
|-------|------|---------|
| Dsl | DSLを読み込み、n次元疎集合割り出しの最適解である、固定長メモリ位置群のトラバーサルに落とし込むための静的データ群を生成する | new(Vec<(u64, u32)>),compile(&[&Path]) |
| Index | Dsl:compile(DSL)を呼び出し、アドレスリスト(Box<(u64, u32)>)を保持し、トラバーサルによってメモリ位置群を取得する | `toraverse()` |
| Context  | operates state data following manifest YAMLs | `toraverse()` |

- provided modules (library provided)
  1. State

-  required modules (library required*)
  1. InMemoryClient
  2. KVSClient
  3. DbClient
  4. EnvClient
  5. HttpClient
  6. FileClient

- internal modules
  1. core::Manifest
  2. Store
  3. Load
  4. u64(fixed_bits.rs)
  5. Pools & Maps(pool.rs)
  6. parser.rs
  7. LogFormat

*: *_client impl are not essential, optional modules.

---

## provided modules

**State** is the sole public API of the library.

A module performing `get()`/`set()`/`delete()`/`exists()` operations on state data following the `_store`/`_load` blocks defined in manifest YAMLs. `get()` automatically attempts loading on key miss. `set()` does not trigger loading. `delete()` removes the specified key from both store and cache. `exists()` checks key existence without triggering auto-load. It maintains an instance-level cache (`state_values`) separate from persistent stores.

State owns YAML I/O: it reads manifest files via `FileClient` and parses them into `core::Manifest` on first access. `core::Manifest` is an internal no_std struct that owns all bit-record data and provides decode/find/build_config queries. Relative placeholders in values are qualified to absolute paths at parse time. Metadata (`_store`/`_load`/`_state`) is inherited from parent nodes; child overrides parent.

## State

### State::get("filename.node")

Reference the state represented by the specified node, returning value or collections.

Returns: `Result<Option<Value>, StateError>`

**Operation flow:**
1. Check `called_keys` (recursion / limit detection)
2. Load manifest file via `FileClient` (first access only)
3. `core::Manifest::find()` → get key_idx
4. **Check `state_values` (by key_idx)** ← Highest priority
5. `core::Manifest::get_meta()` → get MetaIndices
6. If `_load.client == State`: skip store. Otherwise: retrieve from store (KVS/InMemoryClient)
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
- Save to persistent store (KVS/InMemoryClient)
- Also save to `state_values` (instance cache)
- If store is KVS, TTL can be set

**TTL behavior:**
- `ttl` argument specified → Use specified value
- No `ttl` argument, `_store.ttl` in YAML → Use YAML default
- No `ttl` argument, no `_store.ttl` in YAML → Maintain current value

---

### State::delete("filename.node")

Delete the {key:value} record represented by the specified node.

Returns: `Result<bool, StateError>`

**Behavior:**
- Delete from persistent store (KVS/InMemoryClient)
- Also delete from `state_values` (instance cache)
- After deletion, the node shows miss

---

### State::exists("filename.node")

Check if a key exists without triggering auto-load.

Returns: `Result<bool, StateError>`

**Behavior:**
- Check `state_values` (instance cache) first
- Then check persistent store (KVS/InMemoryClient)
- **Does NOT trigger auto-load** (unlike `get()`)
- Returns `Ok(true)` if exists, `Ok(false)` otherwise

**Comparison with get():**
- `get()`: Returns value, triggers auto-load on miss
- `exists()`: Returns boolean, never triggers auto-load

---

## required modules

Application must implement the following traits to handle data stores:

1. **InMemoryClient**
  - expected operations: `get()`/`set()`/`delete()`
  - arguments: `"key":...` from `_{store,load}.key:...` in Manifest
  - expected target: Local process memory

2. **KVSClient**
  - expected operations: `get()`/`set()`/`delete()`
  - trait signature:
    - `fn get(&self, key: &str) -> Option<String>`
    - `fn set(&self, key: &str, value: String, ttl: Option<u64>) -> bool`
    - `fn delete(&self, key: &str) -> bool`
  - arguments: `"key":...` from `_{store,load}.key:...`, `ttl:...` from `_{store,load}.ttl:...`(optional) in Manifest
  - expected target: Key-Value Store (Redis, etc.)
  - **Important**: KVSClient handles String only (primitive type). State layer performs serialize/deserialize:
    - **serialize**: All values → JSON string (preserves type: Number/String/Bool/Null/Array/Object)
    - **deserialize**: JSON string → Value (accurately restores type)

3. **DbClient**
  - expected operations: `get()`/`set()`/`delete()`
  - trait signature:
    - `fn get(&self, connection: &Value, table: &str, columns: &[&str], where_clause: Option<&str>) -> Option<Vec<HashMap<String, Value>>>`
    - `fn set(&self, connection: &Value, table: &str, values: &HashMap<String, Value>, where_clause: Option<&str>) -> bool`
    - `fn delete(&self, connection: &Value, table: &str, where_clause: Option<&str>) -> bool`
  - arguments: `"connection":...`, `"table":...`, `"columns":...` from `_{load}.map.*:...`, `"where_clause":...`(optional)
  - only for `_load.client`

4. **EnvClient**
  - expected operations: `get()`/`set()`/`delete()`
  - arguments: `"key":...` from `_{load}.map.*:...` in Manifest
  - expected target: environment variables
  - only for `_load.client`

5. **HttpClient**
  - expected operations: `get()`/`set()`/`delete()`
  - trait signature:
    - `fn get(&self, url: &str, headers: Option<&HashMap<String, String>>) -> Option<Value>`
    - `fn set(&self, url: &str, body: Value, headers: Option<&HashMap<String, String>>) -> bool`
    - `fn delete(&self, url: &str, headers: Option<&HashMap<String, String>>) -> bool`
  - arguments: `"url":...` from `_{store,load}.url:...`, `"headers":...` from `_{store,load}.headers:...`
  - expected target: HTTP endpoints
  - for both `_store.client` and `_load.client`

6. **FileClient**
  - expected operations: `get()`/`set()`/`delete()`
  - trait signature:
    - `fn get(&self, key: &str) -> Option<String>`
    - `fn set(&self, key: &str, value: String) -> bool`
    - `fn delete(&self, key: &str) -> bool`
  - arguments: `"key":...` from `_{store,load}.key:...` in Manifest
  - expected target: File I/O
  - default impl `DefaultFileClient` is built-in (std::fs based)
  - for both `_store.client` and `_load.client`
  - **always used by State to read manifest YAMLs**

---

## Load::handle()

When `State::get()` misses a value, retrieve data according to `_load` settings.

**Client types:**
- `Env` - Load from environment variables
- `Db` - Load from database
- `KVS` - Load from KVS
- `InMemory` - Load from process memory
- `Http` - Load from HTTP endpoint
- `File` - Load from file
- `State` - Reference another State key directly (does not call Load::handle())

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
- Other clients → Auto-load via `Load::handle()`

**Recursion depth limit:**
- `max_recursion = 20`
- `called_keys: HashSet<String>` tracks keys currently being processed
- On limit exceeded or circular key detected: `Err(StateError::RecursionLimitExceeded)`

---

## State::get() Detailed Flow

```
1. called_keys check (recursion / limit detection)
   ↓
2. Load manifest file via FileClient (first access only)
   ↓
3. core::Manifest::find() → get key_idx
   ↓
4. ★ Check state_values (by key_idx) ← Highest priority
   if find_state_value(key_idx).is_some() { return Ok(Some(value)); }
   ↓
5. core::Manifest::get_meta() → get MetaIndices
   ↓
6. _load.client == State → skip store
   otherwise: retrieve from store (KVS/InMemoryClient)
   ↓
7. On miss, auto-load
   ├─→ build_config() resolves placeholders
   ├─→ Load::handle(config)
   │    ├─→ client: Db → DbClient::get()
   │    ├─→ client: KVS → KVSClient::get()
   │    ├─→ client: Env → EnvClient::get()
   │    ├─→ client: InMemory → InMemoryClient::get()
   │    ├─→ client: Http → HttpClient::get()
   │    └─→ client: File → FileClient::get()
   ├─→ Save to persistent store
   └─→ Save to state_values
   ↓
8. Return Ok(Some(value)) / Ok(None) / Err(StateError)
```

---

## state_values (Instance Memory Cache)

The State struct maintains an instance-level cache (`state_values: StateValueList`) separate from persistent stores (KVS/InMemoryClient).

**Important:** This is NOT InMemoryClient. It is a variable of the State instance itself.

**Purpose:**
1. **Speed up duplicate `State::get()` calls within the same request**
2. **Reduce access count to KVS/InMemoryClient**
3. **Avoid duplicate loads** (don't load the same key multiple times)

**Index:**
- Keyed by `key_idx: u16` — globally unique index in KeyList
- Not keyed by store key string

**Save timing:**
- On successful retrieval from store or load in `State::get()`
- On `State::set()`

**Delete timing:**
- On `State::delete()`

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
- Retrieve qualified path from path_map
- Call `State::get(qualified_path)` to get the value

---

## error case

**ManifestError:**
- `FileNotFound` — manifest file not found in manifest dir
- `AmbiguousFile` — two files with the same name but different extensions (`.yml` and `.yaml`) exist in manifestDir. Manifest ignores extensions (dot-separated paths represent hierarchy), so it cannot distinguish the two. Same-extension duplicates are assumed to be prevented at the OS level.
- `ParseError` — YAML parse failed

**LoadError:**
- `ClientNotConfigured` — required client (Env/KVS/DB/HTTP/File) is not set on State
- `ConfigMissing(String)` — a required config key (key/url/table/map/connection) is missing in the manifest
- `NotFound(String)` — the client call succeeded but returned no data
- `ParseError(String)` — JSON parse error from client response

**StoreError:**
- `ClientNotConfigured` — required client (KVS/InMemory/HTTP/File) is not set on State
- `ConfigMissing(String)` — a required config key (key/url/client) is missing in the manifest
- `SerializeError(String)` — JSON serialize error
- `UnsupportedClient(u64)` — unsupported client id in config

---

## Original Text (ja)

### index

- provided modules (ライブラリ提供モジュール)
  1. State

-  required modules (ライブラリ要求モジュール*)
  1. InMemoryClient
  2. KVSClient
  3. DbClient
  4. EnvClient
  5. HttpClient
  6. FileClient

- internal modules (内部モジュール)
  1. core::Manifest
  2. Store
  3. Load
  4. u64(fixed_bits.rs)
  5. Pools & Maps(pool.rs)
  6. parser.rs
  7. LogFormat

*: いずれもoptional(必須ではない)

---

## Ports

ライブラリの外部向けインターフェース定義modules

1. Provided Port

**State** がライブラリ唯一の公開APIです。

manifest YAMLの`_store`/`_load`定義に従い、`get()` / `set()` / `delete()` / `exists()`操作を提供するmoduleです。`get()`はkey missをトリガーに`_load`定義に基づいて自動ロードを試みます。`set()`は自動ロードを引き起こしません。`delete()`はストアとインスタンスキャッシュ両方から削除します。`exists()`は自動ロードを引き起こさずにkey存在確認を行います。

StateはYAML I/Oを担います。`FileClient`経由でmanifestファイルを読み込み、初回アクセス時に`core::Manifest`へparseします。`core::Manifest`はno_stdの内部structで、全bitレコードデータを所有しdecode/find/build_configクエリを提供します。`_store`/`_load`/`_state`メタデータは親から子へ継承され、子が上書きできます。

2. Required Ports

ライブラリ動作時にimpl実装が必要なmoduleのtraits

  1. **InMemoryClient**
    - 必要なメソッド: `get()`/`set()`/`delete()`
    - 渡される引数: `"key": Manifestの_{store,load}.key:の値`
    - 想定対象ストア: ローカルプロセスメモリ
    - インスタンスメモリのState::cacheにて、_store.clientの値に依らず、キャッシュが常にされている点に留意して下さい。
  2. **KVSClient**
    - 必要なメソッド: `get()`/`set()`/`delete()`
    - traitシグネチャ:
      - `fn get(&self, key: &str) -> Option<String>`
      - `fn set(&self, key: &str, value: String, ttl: Option<u64>) -> bool`
      - `fn delete(&self, key: &str) -> bool`
    - 渡される引数: `"key": Manifestの_{store,load}.key:の値`, `ttl: Manifestの_{store,load}.ttl:の値(オプション)`
    - 想定対象ストア: Key-Valueストア（Redis等）
    - **重要**: KVSClientはString型のみを扱う（プリミティブ型）。State層がserialize/deserializeを実行:
      - **serialize**: 全ての値 → JSON文字列（型情報を保持: Number/String/Bool/Null/Array/Object）
      - **deserialize**: JSON文字列 → Value（型を正確に復元）
      - KVSにはJSON文字列としてデータを保存。JSON形式でKVSネイティブ型に依存せず型情報を保持。
  3. **DbClient**
    - 必要なメソッド: `get()`/`set()`/`delete()`
    - traitシグネチャ:
      - `fn get(&self, connection: &Value, table: &str, columns: &[&str], where_clause: Option<&str>) -> Option<Vec<HashMap<String, Value>>>`
      - `fn set(&self, connection: &Value, table: &str, values: &HashMap<String, Value>, where_clause: Option<&str>) -> bool`
      - `fn delete(&self, connection: &Value, table: &str, where_clause: Option<&str>) -> bool`
    - 渡される引数: `"connection": YAML記載の_{load}.connection:の値`, `"table": YAML記載の_{load}.table:の値`, `"columns": YAML記載の_{load}.map.*:の値`, `"where_clause": YAML記載の_{load}.where:の値`
    - 想定対象ストア: SQLデータベース
    - _load.client: のみに使用対応
  4. **EnvClient**
    - 必要なメソッド: `get()`/`set()`/`delete()`
    - 渡される引数: `"key": Manifestの_{load}.map.*:の値`
    - 想定対象ストア: 環境変数
    - _load.client: のみに使用対応
  5. **HttpClient**
    - 必要なメソッド: `get()`/`set()`/`delete()`
    - traitシグネチャ:
      - `fn get(&self, url: &str, headers: Option<&HashMap<String, String>>) -> Option<Value>`
      - `fn set(&self, url: &str, body: Value, headers: Option<&HashMap<String, String>>) -> bool`
      - `fn delete(&self, url: &str, headers: Option<&HashMap<String, String>>) -> bool`
    - 渡される引数: `"url": YAML記載の_{store,load}.url:の値`, `"headers": YAML記載の_{store,load}.headers:の値`
    - 想定対象ストア: HTTPエンドポイント
    - _store/_load両方に使用対応
  6. **FileClient**
    - 必要なメソッド: `get()`/`set()`/`delete()`
    - traitシグネチャ:
      - `fn get(&self, key: &str) -> Option<String>`
      - `fn set(&self, key: &str, value: String) -> bool`
      - `fn delete(&self, key: &str) -> bool`
    - 渡される引数: `"key": Manifestの_{store,load}.key:の値`
    - 想定対象ストア: ファイルI/O
    - デフォルト実装 `DefaultFileClient` を内蔵（std::fsベース）
    - _store/_load両方に使用対応
    - **StateがmanifestのYAML読み込みに常時使用する**

## State

### State::get("filename.node")

指定されたノードが表すステート(state obj)を参照し、値またはcollectionを返却する。

戻り値: `Result<Option<Value>, StateError>`

**動作フロー:**
1. `called_keys` チェック（再帰・上限検出）
2. `FileClient`経由でmanifestファイルをロード（未ロード時のみ）
3. `core::Manifest::find()` → key_idx 取得
4. **state_values (インスタンスキャッシュ) をチェック** ← 最優先
5. `core::Manifest::get_meta()` → MetaIndices 取得
6. `_load.client == State` の場合はストアをスキップ。それ以外: ストア (KVS/InMemoryClient) から取得
7. **miss時、`Load::handle()` で自動ロード**
8. `Ok(Some(value))` / `Ok(None)` / `Err(StateError)` を返却

**自動ロード:**
- 指定されたノードのステートキーがmissした場合、`Load::handle()` で自動取得を試みる
- `Load::handle()` がエラーの場合、`Err(StateError::LoadFailed(LoadError))` を返す

**_state.typeについての注意:**
```yaml
tenant_id:
  _state:
    type: integer  # メタデータのみ - 検証/キャストは未実装
```

`_state.type`フィールドは現在メタデータのみで、State操作では強制されません。

---

### State::set("filename.node", value, ttl)

指定されたノードが表すステートに値をセットする。

戻り値: `Result<bool, StateError>`

**動作:**
- 永続ストア (KVS/InMemoryClient) に保存
- state_values (インスタンスキャッシュ) にも保存
- ストアがKVSの場合、TTLを設定可能

**TTL動作:**
- `ttl` 引数が指定された → 指定値を使用
- `ttl` 引数なし、YAMLに `_store.ttl` あり → YAMLのデフォルト値を使用
- `ttl` 引数なし、YAMLに `_store.ttl` なし → 現在の値を維持

---

### State::delete("filename.node")

指定されたノードが表す {key:value} レコードを削除する。

戻り値: `Result<bool, StateError>`

**動作:**
- 永続ストア (KVS/InMemoryClient) から削除
- state_values (インスタンスキャッシュ) からも削除
- 削除後、そのノードは miss を示す

---

### State::exists("filename.node")

自動ロードをトリガーせずに、キーの存在確認を行う。

戻り値: `Result<bool, StateError>`

**動作:**
- 最初に state_values (インスタンスキャッシュ) をチェック
- 次に永続ストア (KVS/InMemoryClient) をチェック
- **自動ロードをトリガーしない** (`get()` とは異なる)
- 存在する場合 `Ok(true)`、それ以外 `Ok(false)` を返す

**get() との比較:**
- `get()`: 値を返す、miss時に自動ロードをトリガー
- `exists()`: 真偽値を返す、自動ロードは決してトリガーしない

---

## Load::handle()

`State::get()` が値をmissした際、`_load` 設定に従ってデータを取得する。

**クライアント種別:**
- `Env` - 環境変数からロード
- `Db` - データベースからロード
- `KVS` - KVSからロード
- `InMemory` - プロセスメモリからロード
- `Http` - HTTPエンドポイントからロード
- `File` - ファイルからロード
- `State` - 別のStateキーを参照（Load::handle()を呼ばない）

**State clientの特殊動作:**
```yaml
tenant_id:
  _load:
    client: State
    key: ${org_id}  # State::get("cache.user.org_id")を直接返す
```

`_load.client: State` の場合、`Load::handle()` は呼ばれず、`_load.key` の値（プレースホルダー解決済み）が直接返される。

**設計ルール:**
- `_load` なし → 自動ロードなし、`Ok(None)` を返す
- `_load.client` なし → 自動ロードなし、`Ok(None)` を返す
- `_load.client: State` → `_load.key` の値を直接使用（Load::handle()を呼ばない）
- その他のclient → `Load::handle()` で自動ロード

**再帰深度制限:**
- `max_recursion = 20`
- `called_keys: HashSet<String>` で処理中のキーを管理
- 上限超過または同一キーの再帰検出時に `Err(StateError::RecursionLimitExceeded)` を返す

---

## State::get() 詳細フロー

```
1. called_keys チェック（再帰・上限検出）
   ↓
2. FileClient経由でmanifestファイルをロード（未ロード時のみ）
   ↓
3. core::Manifest::find() → key_idx 取得
   ↓
4. ★ state_values をチェック (key_idx) ← 最優先
   if find_state_value(key_idx).is_some() { return Ok(Some(value)); }
   ↓
5. core::Manifest::get_meta() → MetaIndices 取得
   ↓
6. _load.client == State の場合はストアをスキップ
   それ以外: ストア (KVS/InMemoryClient) から取得
   ↓
7. miss時、自動ロード
   ├─→ build_config() でプレースホルダーを解決
   ├─→ Load::handle(config)
   │    ├─→ client: Db → DbClient::get()
   │    ├─→ client: KVS → KVSClient::get()
   │    ├─→ client: Env → EnvClient::get()
   │    ├─→ client: InMemory → InMemoryClient::get()
   │    ├─→ client: Http → HttpClient::get()
   │    └─→ client: File → FileClient::get()
   ├─→ 永続ストアに保存
   └─→ state_values に保存
   ↓
8. Ok(Some(value)) / Ok(None) / Err(StateError) を返却
```

---

## state_values (インスタンスメモリキャッシュ)

State構造体は、永続ストア（KVS/InMemoryClient）とは別に、インスタンスレベルのキャッシュ（`state_values: StateValueList`）を保持します。

**重要:** これはInMemoryClientではありません。Stateインスタンス自体の変数です。

**目的:**
1. **同一リクエスト内での重複`State::get()`呼び出しを高速化**
2. **KVS/InMemoryClientへのアクセス回数を削減**
3. **重複ロードを回避する設計**（同じキーを複数回ロードしない）

**インデックス:**
- `key_idx: u16` — KeyList上のグローバルユニークなindex をキーとして保存
- 永続ストアのキー文字列ではなく、key_idxで引く設計

**保存タイミング:**
- `State::get()`でストアまたはロードから取得成功時
- `State::set()`時

**削除タイミング:**
- `State::delete()`時

**ライフサイクル:**
- Stateインスタンス生成: 空
- State稼働中: 蓄積
- Stateインスタンス破棄: 破棄（メモリ解放）

---

## プレースホルダー解決ルール

`${}` 内のパスは **parse時に qualified path へ変換済み**。State実行時に変換処理は行わない。

**parse時の qualify ルール（`qualify_path()`）:**
- パスに `.` を含む場合 → 絶対パスとみなしそのまま使用
- `.` を含まない場合 → `filename.ancestors.path` に変換

**例（`cache.yml` の `user._load.where` 内 `${tenant_id}`）:**
```
qualify_path("tenant_id", "cache", ["user"])
→ "cache.user.tenant_id"
```

**State実行時のプレースホルダー解決（`resolve_value_to_string()`）:**
- path_map から qualified path を取り出し
- `State::get(qualified_path)` を呼んで値を取得

---

## error case

**ManifestError:**
- `FileNotFound` — manifestディレクトリにファイルが見つからない
- `AmbiguousFile` — manifestDir内に拡張子違いの同名ファイルが2つ存在する（`.yml`と`.yaml`）。ドット区切りを階層表現とするため拡張子を無視し、区別できない。同拡張子の同名ファイルはOSレベルでの非許容を想定。
- `ParseError` — YAMLのパース失敗

**LoadError:**
- `ClientNotConfigured` — 必要なclient（Env/KVS/DB/HTTP/File）がStateに未設定
- `ConfigMissing(String)` — manifest内に必須のconfigキー（key/url/table/map/connection）が欠落
- `NotFound(String)` — clientの呼び出しは成功したがデータが存在しなかった
- `ParseError(String)` — clientレスポンスのJSONパースエラー

**StoreError:**
- `ClientNotConfigured` — 必要なclient（KVS/InMemory/HTTP/File）がStateに未設定
- `ConfigMissing(String)` — manifest内に必須のconfigキー（key/url/client）が欠落
- `SerializeError(String)` — JSONシリアライズエラー
- `UnsupportedClient(u64)` — configに未対応のclient idが指定された
