# Architecture

## index

- provided modules (ライブラリ提供モジュール)
  1. Manifest
  2. State

-  required modules (ライブラリ要求モジュール*)
  1. InMemoryClient
  2. KVSClient
  3. DBClient
  4. EnvClient

- common modules (内部コモンモジュール)
  1. DotString
  2. DotMapAccessor
  3. Placeholder
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

  1. **Manifest** - YAMLファイルの読み込みと集計をするmodule。'_'始まりのmeta keysを認識し、get()メソッドでは無視したcollectionを返却、getMeta()では親から子に継承と上書きをしながら集計し返却する。収集時、メタブロック内の_load.map.*のキー値は、YAMLファイルのfilename.key1.key2.,....(絶対パス)に変換する。
  2. **State** - Manifest::getMeta()から取得する_storeブロックの記述に基づいて格納されるステートデータ(state obj)を対象に、`get()` / `set()` / `delete()`操作を行うmodule。`get()`では、key miss hitをトリガーとして、同じく取得した`_load`ブロックの記述に基づいてロード試行を自動的に行う。`set()`は指定のkeyに値をsetする。自動ロードは引き起こさない。`delete()`は指定のkeyと、そのvalue全てを削除する。Stateは、インスタンスメモリの`State.cache`にYAMLファイル記述に従ったcollection型でstate objをキャッシュし、動作中、同期処理を行う。ロードを引き起こさないmiss/hit key判定の`exists()`も提供している。

2. Required Ports

ライブラリ動作時にimpl実装が必要なmoduleのtraits

  1. **InMemoryClient**
    - 必要なメソッド: `get()`/`set()`/`delete()`
    - 渡される引数: `'key': Manifestの_{store,load}.key:の値`
    - 想定対象ストア: ローカルプロセスメモリ
    - 引数の各キーに対応した、プロセスメモリパスをマッピングして下さい。
    - インスタンスメモリのState::cacheにて、_store.clientの値に依らず、キャッシュが常にされている点に留意して下さい。
  2. **KVSClient**
    - 必要なメソッド: `get()`/`set()`/`delete()`
    - traitシグネチャ:
      - `fn get(&self, key: &str) -> Option<String>`
      - `fn set(&mut self, key: &str, value: String, ttl: Option<u64>) -> bool`
      - `fn delete(&mut self, key: &str) -> bool`
    - 渡される引数: `'key': Manifestの_{store,load}.key:の値`, `ttl: Manifestの_{store,load}.ttl:の値(オプション)`
    - 想定対象ストア: Key-Valueストア（Redis等）
    - **重要**: KVSClientはString型のみを扱う（プリミティブ型）。State層がserialize/deserializeを実行:
      - **serialize**: 全ての値 → JSON文字列（型情報を保持: Number/String/Bool/Null/Array/Object）
      - **deserialize**: JSON文字列 → Value（型を正確に復元）
      - **型保持**: JSON形式で型を区別（例: `42` vs `"42"`, `true` vs `"true"`）
      - KVSにはJSON文字列としてデータを保存。個別フィールドは取得後に抽出。
      - 設計意図: YAML構造に忠実でありながら、KVSはプリミティブに保つ。JSON形式でKVSネイティブ型に依存せず型情報を保持。
  3. **DBClient**
    - 必要なメソッド: `fetch()`
    - 渡される引数: `'connection': YAML記載の_{store,load}.connection:の値`, `'table': YAML記載の_{store,load}.table:の値}`, `'columns': YAML記載の_{store,load}.map.*:の値`, `'where_clause': YAML記載の_{store,load}.where:の値`
    - 想定対象ストア: SQLデータベース
    - _load.client: のみに使用対応
  4. **EnvClient**
    - 必要なメソッド: `get()`
    - 渡される引数: `'key': Manifestの_{store,load}.map.*:の値`
    - 想定対象ストア: 環境変数
    - _load.client: のみに使用対応

## Manifest

1. `get(key: &str, default: Option<Value>)` -> Value

2. `getMeta(key: &str)` -> HashMap<String, Value>

3. `get_missing_keys()` -> &[String]

`get(), getMeta()`がインスタンスメモリに記録したmiss keyのlist(missingKeys)を返却する

4. `clear_missing_keys()`

インスタンスメモリのmissingKeysを空にする

## State

### State::get('filename.node')

指定されたノードが表すステート(state obj)を参照し、値またはcollectionを返却する。

**動作フロー:**
1. `Manifest::getMeta()` でメタデータを取得
2. `_store` 設定からストア種別を判定 (KVS/InMemory)
3. プレースホルダーを解決 (`${session.sso_user_id}` など)
4. ストアキーを構築
5. **State.cache (インスタンスcollection object) をチェック** ← 最優先
6. ストア (KVS/InMemoryClient) から値を取得
7. データから個別フィールドを抽出
8. **miss時、`Load::handle()` で自動ロード**
9. 値を返却（型キャストは現在未実装）

**自動ロード:**
- 指定されたノードのステートキーがmissした場合、`Load::handle()` で自動取得を試みる
- `Load::handle()` がエラーの場合、`None` を返す

**_state.typeについての注意:**
```yaml
tenant_id:
  _state:
    type: integer  # メタデータのみ - 検証/キャストは未実装
```

`_state.type`フィールドは現在メタデータのみで、State操作では強制されません。将来のバージョンで型検証とキャストを実装する可能性があります。

---

### State::set('filename.node', value, ttl)

指定されたノードが表すステートに値をセットする。

**動作:**
- 永続ストア (KVS/InMemoryClient) に保存
- State.cache にも保存
- ストアがKVSの場合、TTLを設定可能

**TTL動作:**
- `ttl` 引数が指定された → 指定値を使用
- `ttl` 引数なし、YAMLに `_store.ttl` あり → YAMLのデフォルト値を使用
- `ttl` 引数なし、YAMLに `_store.ttl` なし → 現在の値を維持

---

### State::delete('filename.node')

指定されたノードが表す {key:value} レコードを削除する。

**動作:**
- 永続ストア (KVS/InMemoryClient) から削除
- State.cache からも削除
- 削除後、そのノードは miss hit を示す

---

### State::exists('filename.node')

自動ロードをトリガーせずに、キーの存在確認を行う。

**動作:**
- 最初に State.cache をチェック
- 次に永続ストア (KVS/InMemoryClient) をチェック
- **自動ロードをトリガーしない** (`get()` とは異なる)
- 真偽値を返す (存在する場合true、それ以外false)

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
- `DB` - データベースからロード
- `KVS` - KVSからロード
- `InMemory` - プロセスメモリからロード
- `State` - 別のStateキーを参照（自己参照）

**State clientの特殊動作:**
```yaml
tenant_id:
  _load:
    client: State
    key: ${org_id}  # State::get('cache.user.org_id')を直接返す
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
- MAX_RECURSION = 10
- 再帰呼び出し毎にカウンターをインクリメント
- 超過時にエラーをスロー
- finallyブロックでカウンターをデクリメント

---

## State::get() 詳細フロー

```
1. Manifest::getMeta(key) → メタデータ取得
   ↓
2. _state から型情報を取得
   ↓
3. _store から保存先を取得 (client: KVS/InMemory)
   ↓
4. store_config内のプレースホルダーを解決
   ↓
5. ★ インメモリキャッシュをチェック (絶対キー) ← 最優先
   if cache.contains_key(key) { return; }
   ↓
6. storeKeyを構築
   ↓
7. ストアから値を取得 (getFromStore)
   ↓
8. データから個別フィールドを抽出
   ↓
9. miss時、自動ロード
   ├─→ Load::handle(loadConfig)
   │    ├─→ client: DB → DBClient::fetchOne/fetchAll()
   │    ├─→ client: KVS → KVSClient::get()
   │    ├─→ client: Env → EnvClient::get()
   │    ├─→ client: InMemory → InMemoryClient::get()
   │    └─→ client: State → 指定キー値を直接返す（再帰）
   ├─→ 永続ストアに保存 (setToStore)
   └─→ インメモリキャッシュに保存
   ↓
10. 値を返却
```

---

## State.cache (インスタンスメモリキャッシュ)

State構造体は、永続ストア（KVS/InMemoryClient）とは別に、インスタンスレベルのキャッシュ（`cache: Value`）を保持します。

**重要:** これはInMemoryClientではありません。Stateインスタンス自体の変数です。

**目的:**
1. **同一リクエスト内での重複`State::get()`呼び出しを高速化**
2. **KVS/InMemoryClientへのアクセス回数を削減**
3. **重複ロードを回避する設計**（同じキーを複数回ロードしない）

**チェック順序（重要）:**
```Rust
// State::get() フロー
1. メタデータ取得
2. _state から型情報を取得
3. _store から保存先を取得
4. プレースホルダー解決
5. ★ State.cache をチェック (絶対キー) ← 最初にチェック
   if self.cache.contains_key(key) {
       return self.cache[key];
   }
6. storeKey構築
7. 永続ストア (KVS/InMemoryClient) から取得
8. miss時、自動ロード → ロード後、State.cacheに保存
```

**キャッシュキー:**
- **絶対パス**で保存 (`cache.user.tenant_id`)
- ドット記法そのまま

**保存タイミング:**
- `State::get()`でロード成功時: `self.cache.insert(key, extracted)`
- `State::set()`時: `self.cache.insert(key, value)`

**削除タイミング:**
- `State::delete()`時: `self.cache.remove(key)`

**ライフサイクル:**
- Stateインスタンス生成: 空
- State稼働中: 蓄積
- Stateインスタンス破棄: 破棄（メモリ解放）

**重要な設計意図:**
- State.cacheは永続ストア（KVS/InMemoryClient）より高優先でチェックされる
- これにより外部ストアを包括的に扱う設計を実現
- 同一データへの複数アクセスでも、1回のストアアクセス + N回のHashMapアクセスで済む

---

## プレースホルダー解決ルール

プレースホルダー解決の優先順位。

**解決順序:**
1. **同一ディクショナリ参照（相対パス）**: `${org_id}` → `cache.user.org_id`
2. **絶対パス**: `${org_id}` → `org_id`

**例（contextKey: 'cache.user.tenant_id._load.key'）:**
```
// ディクショナリスコープを抽出
dictScope = 'cache.user'; // メタキー(_load)より前まで

// 1. 同一ディクショナリ内を検索
scopedKey = 'cache.user.org_id';
value = self.get(scopedKey); // → State::get('cache.user.org_id')
if value.is_some() { return value; }

// 2. 絶対パスを検索
return self.get('org_id'); // → State::get('org_id')
```

**注意:**
- ディクショナリスコープはメタキー（`_load`, `_store`等）または最後のフィールドまで辿る
- `cache.user`がディクショナリ、`org_id`/`tenant_id`がフィールドという想定

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
    let segments: Vec<&str> = key.split('.').collect();
    let field_name = segments.last().unwrap();

    // ディクショナリからフィールドを抽出
    data.get(field_name).cloned().unwrap_or(Value::Null)
}
```

---

## 内部実装

### Placeholder

純粋な文字列処理（依存関係なし）。

**メソッド:**
- `extract_placeholders(template)` - テンプレートから変数名を抽出
- `replace(template, params)` - 値で置換
- `resolve_typed(value, resolver)` - JSON値内のプレースホルダーを再帰的に解決

**型保持:**
- 単一プレースホルダーかつ文字列全体が`${...}`形式 → 型を保持
- 複数または文字列内プレースホルダー → 文字列置換

### DotMapAccessor

ドット記法での配列アクセスを提供。

**メソッド:**
- `get(data, path)` - ドット記法で値を取得
- 例: `get(data, "user.profile.name")`

---

## error case

- manifestDir/{*.yml,*.yaml}の中に、拡張子違いの2つの同名ファイルが存在する
  - エラータイミング: Manifest moduleが該当2ファイルを読んで題意を検知した時
  - 理由: ドット区切りを階層表現とするManifestは、拡張子を無視するため、該当の同名ファイルを区別出来ないため
  - 備考: 同拡張子の同名ファイルはOSレベルでの非許容を想定して確認していない