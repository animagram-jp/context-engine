# Architecture

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
| tree | `enum Tree` の wire format serialize / deserialize | serialize, deserialize | tree.rs |
| Dsl | `Tree` のDSLを読み込み、固定長メモリ位置群のトラバーサルに落とし込むための静的データ群を生成する。`feature=precompile` 時は `Dsl::write()` でYAML→静的Rustファイル出力 | compile, write(precompile) | dsl.rs |
| Index | `Dsl::compile` の出力を保持し、トラバーサルによってleaf参照群を取得する | new, traverse | index.rs |
| Context | コンテクストデータの操作を行うリクエスト処理スコープの実行インスタンス | new, get, set, delete, exists | context.rs |

* Portsはpub fnのこと
* new()であっても、引数はVec等の標準型依存を明示するべき。construct状態は避ける

### Portモジュール

| Mod | Description | Filename |
|-----|-------------|----------|
| Context | Contextのtrait、Tree型、各Error型 | ports/provided.rs |
| StoreClient | 単一ストアのadapter trait | ports/required.rs |
| StoreRegistry | client名称→StoreClientのdispatch trait | ports/required.rs |

### 開発用モジュール

| Mod | Description | Filename |
|-----|-------------|----------|
| debug_log | `feature=logging` 限定のデバッグログマクロ・ユーティリティ | debug_log.rs |

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

## mod:fn詳細仕様

### StoreClient

単一ストアの操作を提供するtrait。`key`は予約引数として明示し、追加の任意引数は`args`のflatなBTreeMapで渡す。

```rust
pub trait StoreClient: Send + Sync {
    fn get(&self, key: &str, args: &BTreeMap<&str, Tree>) -> Option<Tree>;
    fn set(&self, key: &str, args: &BTreeMap<&str, Tree>) -> Option<SetOutcome>;
    fn delete(&self, key: &str, args: &BTreeMap<&str, Tree>) -> bool;
}
```

- `key`: DSL の `_load.key` / `_store.key` の値。予約引数。
- `args`: ttl・connection・map 等、ストア種別ごとの任意引数。利用者がimpl内で定義・参照する。
- 内部可変性・スレッド安全性はimplementor側の責任。

### StoreRegistry

YAMLの`client:`名称とStoreClientの対応を管理するtrait。利用者がimplし、Contextに渡す。

```rust
pub trait StoreRegistry {
    fn client_for(&self, yaml_name: &str) -> Option<&dyn StoreClient>;
}
```

- ライブラリはYAML名称の文字列をそのまま`client_for()`に渡してmatchを回す。
- YAML上の名義（`"Memory"`, `"Kvs"`, `"TenantDb"`等）は利用者が自由に定義する。

### Instance Cache

Contextインスタンス固有のキャッシュ。StoreClientとは独立。

- Context生成時: 空
- Context生存中: get/setに応じて蓄積
- Context破棄時: 解放

### Context.get()

指定パスが表す値を返却する。

戻り値: `Result<Option<Tree>, ContextError>`

**動作フロー:**
1. called チェック（再帰・上限検出）
2. `Index::traverse(path)` → LeafRef一覧
3. instance cache をチェック
4. `_store` client に問い合わせ
5. miss時、`_load` client で自動ロード → write-through to `_store`
6. `Ok(Some(value))` / `Ok(None)` / `Err(ContextError)` を返却

## Placeholder Resolution Rules

`${}` paths are always treated as absolute paths.

**Placeholder resolution at runtime:**
- `is_template=false`（単独 `${path}`）: `Context.get(path)` の値をそのままコピー（string化しない）
- `is_template=true`（文字列混在）: 各placeholderを `Context.get()` で解決しstringとして結合

## Error Types

**ParseError** (`ports/provided.rs`):
- `FileNotFound(String)`
- `AmbiguousFile(String)`
- `ParseError(String)`

**LoadError** (`ports/provided.rs`):
- `ClientNotFound(String)` — `StoreRegistry::client_for()` が None を返した
- `ConfigMissing(String)` — DSL内に必須のconfigキーが欠落
- `NotFound(String)` — clientの呼び出しは成功したがデータが存在しなかった
- `ParseError(String)` — clientレスポンスのパースエラー

**StoreError** (`ports/provided.rs`):
- `ClientNotFound(String)` — `StoreRegistry::client_for()` が None を返した
- `ConfigMissing(String)` — DSL内に必須のconfigキーが欠落
- `SerializeError(String)` — シリアライズエラー

**ContextError** (`ports/provided.rs`):
- `ParseFailed(String)`
- `KeyNotFound(String)`
- `RecursionLimitExceeded`
- `StoreFailed(StoreError)`
- `LoadFailed(LoadError)`
