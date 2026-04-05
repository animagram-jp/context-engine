# Architecture

*warning*: Temporarily, Japanese and English combind in this file.

## ライブラリ要件

- README 3行目 参照
- システムが認識するべき概念を階層構造の名前空間で表現できたとする。この時、名前空間から導かれる全通りの(部分含む)パスが、ランタイムの単一処理スコープで操作する可能性のある値のキーを網羅している。このキー群の値全てを、DSLにて漏れなく取得方法の定義を宣言する。

## 機能構成

- parse & compile: DSLを読み込み、n次元疎集合割り出しの最適解である、固定長メモリ位置群のトラバーサルに落とし込むための静的データ群を生成する
- traversal: 上記データ群を保持し、トラバーサルによってメモリ位置群を取得する
- addressing & operation: Manifestに対応した1層mapを保持し、アプリケーションからの呼び出しに応じて値の操作を行う。リクエスト処理スコープインスタンス。

## モジュール構成

### 実体部

| Mod | Description | Ports | Filename |
|-----|-------------|-------|----------|
| Tree | YAMLファイルを読み込んでProvided::Tree型にパースし、Dsl::compileの出力を実行ファイルに書き込む | write | tree.rs |
| Dsl | Tree型のDSLを読み込み、n次元疎集合割り出しの最適解である、固定長メモリ位置群のトラバーサルに落とし込むための静的データ群を生成する | new, compile | dsl.rs |
| Index | Dsl:compile(DSL)を呼び出し、アドレスリスト(Box<(u64, u32)>)を保持し、トラバーサルによってメモリ位置群を取得する | new, traverse | index.rs |
| Context | コンテクストデータの操作を行うリクエスト処理スコープの実行インスタンス | new, get, set, delete, exists | context.rs |

* Portsはpub fnのこと
* new()であっても、引数はVec等の標準型依存を明示するべき。construct状態は避ける
* Tree::write()は引数にoptionを取って、「Dsl::compileの出力を」をskipしてValueのまま書き込むオプションを追加予定
* Context.new()内の各StoreClientは、Arcで記述する。ClientResistoryを新規導入したので、検討余地あるかも
* context.rsが煩雑になるようなら、内部modとしてLoadとStoreを切り出す必要があるかも。係数明示不足による所有複雑化に注意

### Portモジュール

| Mod | Description | Ports | Filename |
|-----|-------------|------|----------|
| Context         | Contextのtrait  | - | provided.rs |
| StoreClient     | *Clientの基底    | - | required.rs |
| ClientResistory | *Clientの登録用  | - | provided.rs |

### 開発用モジュール

| Mod | Description | Port | Filename |
|-------|------|---------|
| Error | Provided Port | - | provided.rs |
| Log | feature=logging限定のマクロ | fn_log! | provided.rs |

## 用語

```yaml
key: n層マップDSLの最末端value以外の要素
keyword: keyの名前文字列
field_key: 自身と親祖先がkeywordが'_'で始まらないkey
meta_key: keywordが'_'始まりのkeyと、その子孫key
leaf_key: 子にkeyを持たず値を持つkey
value: leaf keysの値。DSL内で省略された場合はnullが充てられる
path: 単一のfield_keyを表す、'.'区切りkeywordのチェーン
qualified_path: DSL内で一意な完全修飾パス
placeholder: key参照記述("${path}")。valueのみに適用
template: placeholderと静的な文字列を混合した、動的生成テンプレート。valueのみに適用
called_path: Stateに渡されるパス文字列
```

## mod:fn詳細仕様

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

## Context Instance Cache

An instance-level cache separate from persistent stores.

**Important:** This is NOT a StoreClient. It is a variable of the State instance itself.

**Purpose:**
1. Speed up duplicate `Context.get()` calls within the same request
2. Reduce access count to stores
3. Avoid duplicate loads

**Lifecycle:**
- State instance created: empty
- During State lifetime: accumulates
- State instance dropped: destroyed (memory released)

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
