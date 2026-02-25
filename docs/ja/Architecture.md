# Architecture

## index

- provided modules (ライブラリ提供モジュール)
  1. Manifest
  2. State

-  required modules (ライブラリ要求モジュール*)
  1. InMemoryClient
  2. KVSClient
  3. DbClient
  4. EnvClient

- common modules (内部コモンモジュール)
  1. u64(bit.rs)
  2. Pools & Maps(pool.rs)
  3. ParsedManifest(parser.rs)
  4. LogFormat

- internal modules (内部モジュール)
  1. Store
  2. Load

*: いずれもoptional(必須ではない)

---

## Ports

ライブラリの外部向けインターフェース定義modules

1. Provided Port

ライブラリ提供moduleのtraits

  1. **Manifest** - YAMLファイルの読み込みと集計をするmodule。"_"始まりのmeta keysを認識し、get()メソッドでは無視したcollectionを返却、getMeta()では親から子に継承と上書きをしながら集計し返却する。収集時、メタブロック内の_load.map.*のキー値は、YAMLファイルのfilename.key1.key2.,....(絶対パス)に変換する。
  2. **State** - Manifest::getMeta()から取得する_storeブロックの記述に基づいて格納されるステートデータ(state obj)を対象に、`get()` / `set()` / `delete()`操作を行うmodule。`get()`では、key miss hitをトリガーとして、同じく取得した`_load`ブロックの記述に基づいてロード試行を自動的に行う。`set()`は指定のkeyに値をsetする。自動ロードは引き起こさない。`delete()`は指定のkeyと、そのvalue全てを削除する。Stateは、インスタンスメモリの`State.cache`にYAMLファイル記述に従ったcollection型でstate objをキャッシュし、動作中、同期処理を行う。ロードを引き起こさないmiss/hit key判定の`exists()`も提供している。

2. Required Ports

ライブラリ動作時にimpl実装が必要なmoduleのtraits

  1. **InMemoryClient**
    - 必要なメソッド: `get()`/`set()`/`delete()`
    - 渡される引数: `"key": Manifestの_{store,load}.key:の値`
    - 想定対象ストア: ローカルプロセスメモリ
    - 引数の各キーに対応した、プロセスメモリパスをマッピングして下さい。
    - インスタンスメモリのState::cacheにて、_store.clientの値に依らず、キャッシュが常にされている点に留意して下さい。
  2. **KVSClient**
    - 必要なメソッド: `get()`/`set()`/`delete()`
    - traitシグネチャ:
      - `fn get(&self, key: &str) -> Option<String>`
      - `fn set(&mut self, key: &str, value: String, ttl: Option<u64>) -> bool`
      - `fn delete(&mut self, key: &str) -> bool`
    - 渡される引数: `"key": Manifestの_{store,load}.key:の値`, `ttl: Manifestの_{store,load}.ttl:の値(オプション)`
    - 想定対象ストア: Key-Valueストア（Redis等）
    - **重要**: KVSClientはString型のみを扱う（プリミティブ型）。State層がserialize/deserializeを実行:
      - **serialize**: 全ての値 → JSON文字列（型情報を保持: Number/String/Bool/Null/Array/Object）
      - **deserialize**: JSON文字列 → Value（型を正確に復元）
      - **型保持**: JSON形式で型を区別（例: `42` vs `"42"`, `true` vs `"true"`）
      - KVSにはJSON文字列としてデータを保存。個別フィールドは取得後に抽出。
      - 設計意図: YAML構造に忠実でありながら、KVSはプリミティブに保つ。JSON形式でKVSネイティブ型に依存せず型情報を保持。
  3. **DbClient**
    - 必要なメソッド: `fetch()`
    - 渡される引数: `"connection": YAML記載の_{store,load}.connection:の値`, `"table": YAML記載の_{store,load}.table:の値}`, `"columns": YAML記載の_{store,load}.map.*:の値`, `"where_clause": YAML記載の_{store,load}.where:の値`
    - 想定対象ストア: SQLデータベース
    - _load.client: のみに使用対応
  4. **EnvClient**
    - 必要なメソッド: `get()`
    - 渡される引数: `"key": Manifestの_{store,load}.map.*:の値`
    - 想定対象ストア: 環境変数
    - _load.client: のみに使用対応

## Manifest

1. `load(file: &str)` -> `Result<(), ManifestError>`

2. `find(file: &str, path: &str)` -> `Option<u16>`

3. `get_meta(file: &str, path: &str)` -> `MetaIndices`

4. `get_value(file: &str, path: &str)` -> `Vec<(u16, u16)>`

## State

### State::get("filename.node")

指定されたノードが表すステート(state obj)を参照し、値またはcollectionを返却する。

戻り値: `Result<Option<Value>, StateError>`

**動作フロー:**
1. `Manifest::get_meta()` でメタデータを取得
2. `_store` 設定からストア種別を判定 (KVS/InMemory)
3. **state_values (インスタンスキャッシュ) をチェック** ← 最優先
4. ストア (KVS/InMemoryClient) から値を取得
5. **miss時、`Load::handle()` で自動ロード**
6. 値を返却（型キャストは現在未実装）

**自動ロード:**
- 指定されたノードのステートキーがmissした場合、`Load::handle()` で自動取得を試みる
- `Load::handle()` がエラーの場合、`Err(StateError::LoadFailed)` を返す

**_state.typeについての注意:**
```yaml
tenant_id:
  _state:
    type: integer  # メタデータのみ - 検証/キャストは未実装
```

`_state.type`フィールドは現在メタデータのみで、State操作では強制されません。将来のバージョンで型検証とキャストを実装する可能性があります。

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
- 削除後、そのノードは miss hit を示す

---

### State::exists("filename.node")

自動ロードをトリガーせずに、キーの存在確認を行う。

戻り値: `Result<bool, StateError>`

**動作:**
- 最初に state_values (インスタンスキャッシュ) をチェック
- 次に永続ストア (KVS/InMemoryClient) をチェック
- **自動ロードをトリガーしない** (`get()` とは異なる)
- 存在する場合 `Ok(true)`、それ以外 `Ok(false)` を返す

**ユースケース:**
- 高コスト操作の前の軽量な存在確認
- データベースロードをトリガーせずに条件分岐
- パフォーマンス重視のチェック

**get() との比較:**
- `get()`: 値を返す、miss時に自動ロードをトリガー
- `exists()`: 真偽値を返す、自動ロードは決してトリガーしない

---

## Load::handle()

`State::get()` が値をmissした際、`Manifest::getMeta()` から取得した `_store` と `_load` 設定に従ってデータを取得する。

**クライアント種別:**
- `Env` - 環境変数からロード
- `Db` - データベースからロード
- `KVS` - KVSからロード
- `InMemory` - プロセスメモリからロード
- `State` - 別のStateキーを参照（自己参照）

**State clientの特殊動作:**
```yaml
tenant_id:
  _load:
    client: State
    key: ${org_id}  # State::get("cache.user.org_id")を直接返す
```

`_load.client: State` の場合、`Load::handle()` は呼ばれず、`_load.key` の値（プレースホルダー解決済み）が直接返される。

**設計ルール:**
- `_load` なし → 自動ロードなし、`None` を返す
- `_load.client` なし → 自動ロードなし、`None` を返す
- `_load.client: State` → `_load.key` の値を直接使用（Load::handle()を呼ばない）
- その他のclient → `Load::handle()` で自動ロード

これは、親の `_load.client` を継承せずに、State内の別キーを参照するための明示的な指定である。

**注意:**
- `client == null` はYAML設定ミスとして扱われる
- Load::handle() のmatch文はdefaultケースでnullを返す

**再帰深度制限:**
- `max_recursion = 20`
- `called_keys: HashSet<String>` で処理中のキーを管理
- 上限超過または同一キーの再帰検出時に `Err(StateError::RecursionLimitExceeded)` を返す

---

## State::get() 詳細フロー

```
1. called_keys チェック（再帰・上限検出）
   ↓
2. Manifest::load() → ファイルロード（未ロード時のみ）
   ↓
3. Manifest::find() → key_idx 取得
   ↓
4. ★ state_values をチェック (key_idx) ← 最優先
   if find_state_value(key_idx).is_some() { return Ok(Some(value)); }
   ↓
5. Manifest::get_meta() → MetaIndices 取得
   ↓
6. _load.client == State の場合はストアをスキップ
   それ以外: ストア (KVS/InMemoryClient) から取得
   ↓
7. miss時、自動ロード
   ├─→ build_config() でプレースホルダーを解決
   ├─→ Load::handle(config)
   │    ├─→ client: Db → DbClient::fetch()
   │    ├─→ client: KVS → KVSClient::get()
   │    ├─→ client: Env → EnvClient::get()
   │    └─→ client: InMemory → InMemoryClient::get()
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

`${}` 内のパスは **parse時（`Manifest::load()`）に qualified path へ変換済み**。State実行時に変換処理は行わない。

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

## フィールド抽出

データ取得時、個別フィールドの抽出が必要な場合があります。

**extractField ロジック:**
```Rust
fn extract_field(data: Value, key: &str) -> Value {
    // オブジェクトでない場合、そのまま返す
    if !data.is_object() {
        return data;
    }

    // キーの最後のセグメントを取得
    // cache.user.id → id
    let segments: Vec<&str> = key.split(".").collect();
    let field_name = segments.last().unwrap();

    // ディクショナリからフィールドを抽出
    data.get(field_name).cloned().unwrap_or(Value::Null)
}
```

---

## error case

- manifestDir/{*.yml,*.yaml}の中に、拡張子違いの2つの同名ファイルが存在する
  - エラータイミング: Manifest moduleが該当2ファイルを読んで題意を検知した時
  - 理由: ドット区切りを階層表現とするManifestは、拡張子を無視するため、該当の同名ファイルを区別出来ないため
  - 備考: 同拡張子の同名ファイルはOSレベルでの非許容を想定して確認していない