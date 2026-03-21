# Architecture

## index

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
