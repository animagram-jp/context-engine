// This file includes untranslated text (ja).

# Architecture

## Requirement

- [README.md:3](../README.md#context-engine)
- システムが認識するべき概念を階層構造の名前空間で表現できたとする。この時、名前空間から導かれる全通りの(部分含む)パスが、ランタイムの単一処理スコープで操作する可能性のある値のキーを網羅している。このキー群の値全てを、DSLにて漏れなく取得方法の定義を宣言する。

## Function

- parse: YAMLをパースする
- compile: dslから、n次元疎集合割り出しの最適解である、固定長メモリ位置群のトラバーサルに落とし込むための静的データ群を生成する
- traversal: 上記データ群を保持し、トラバーサルによってメモリ位置群を取得する
- addressing & operation: Manifestに対応した1層mapを保持し、アプリケーションからの呼び出しに応じて値の操作を行う。リクエスト処理スコープインスタンスで行い、リクエストを跨いで保持したい場合は全て_storeにて指示する

## Port

| Module | Port | Signature | Description | Filename |
|--------|------|-----------|-------------|----------|
| - | `debug_log!` | `(class, fn $(, arg)*)` | `feature=logging` ログマクロ | debug_log.rs |
| - | `Tree` | - | n次元scalar map型 | provided.rs |
| - | `SetOutcome` | - | `Store::set`が返すCreatedかUpdated | required.rs |
| Tree | `wire` | `(&self) -> Vec<u8>` | Treeをワイヤフォーマットに変換 | tree.rs |
|      | `unwire` | `(bytes: &[u8]) -> Option<Tree>` | ワイヤフォーマットからTreeへ変換 | tree.rs |
| Dsl | `compile` | `(tree: &Tree) -> (Box<[u64]>, Box<[u32]>, Box<[u8]>, Box<[u8]>, Box<[u64]>)` | Treeから静的list(paths/children/leaves/interning/interning_idx) を返す | dsl.rs |
|  | `write` | `(src: &[u8], out_path: &str) -> Result<(), String>` | YAMLファイルパスから.rsを出力[precompile] | dsl.rs |
| Index | `new` | `(paths, children, leaves, interning, interning_idx) -> Index` | compile済みdslからIndex構築 | index.rs |
|       | `traverse` | `(&self, path: &str) -> Box<[LeafRef]>` | パス文字列からleaf参照リストを返す | index.rs |
|       | `keyword_of` | `(&self, path_idx: u32) -> &[u8]` | path_idxからkeywordバイト列を返す | index.rs |
|       | `load_args` | `(&self, leaf: &LeafRef) -> (&str, BTreeMap<String, Tree>)` | leafの`_load` client名とargsを返す | index.rs |
|       | `store_args` | `(&self, leaf: &LeafRef) -> (&str, BTreeMap<String, Tree>)` | leafの`_store` client名とargsを返す | index.rs |
| Context | `new` | `(index: Arc<Index>, registry: &'r dyn StoreRegistry) -> Context` | IndexとStoreRegistoryからContextを構築 | context.rs |
|         | `get` | `(&mut self, key: &str) -> Result<Option<Tree>, ContextError>` | パス文字列から値(cache→_store→_load)を取得して返す | context.rs |
|         | `set` | `(&mut self, key: &str, value: Tree) -> Result<bool, ContextError>` | 値から_storeに書き込み、cacheも更新 | context.rs |
|         | `delete` | `(&mut self, key: &str) -> Result<bool, ContextError>` | パスから_storeの値を削除し、cacheもnullで更新 | context.rs |
|         | `exists` | `(&mut self, key: &str) -> Result<bool, ContextError>` | パスからcacheか_storeに値が存在するか確認し、cacheを更新 | context.rs |
| Store | `get` | `&self, key: &str, args: &BTreeMap<&str, Tree> -> Option<Tree>` | keyとdsl記載のmapから値をlistで返す | required.rs |
|             | `set` | `&self, key: &str, args: &BTreeMap<&str, Tree> -> Option<SetOutcome>` | keyとdsl記載のマップから値を保存しSetOutcomeを返す | required.rs |
|             | `delete` | `&self, key: &str, args: &BTreeMap<&str, Tree> -> bool` | keyとdsl記載のマップから値を削除し成否を返す | required.rs |
| StoreRegistry | `store_for` | `&self, keyword: &str -> Option<&dyn Store>` | StoreのkeywordからStoreを返す | required.rs |
| DslError | `fmt` | `&self, f: &mut fmt::Formatter<'_> -> fmt::Result` | Dslのエラーを返す | provided.rs |
| LoadError | `fmt` |  | _loadクライアント呼び出しエラーを返す | provided.rs |
| StoreError | `fmt` |  | _storeクライアント呼び出しエラーを返す | provided.rs |
| ContextError | `fmt` |  | Contextの出力するエラーを返す | provided.rs |


## プライベートfn構成

| Module | fn | Signature | Description | Filename |
|--------|----|-----------|-------------|----------|
| Compiler | `new` | `() -> Compiler` | Compiler初期化 | dsl.rs |
|          | `walk_field_key` | `(&mut self, keyword: &[u8], value: &Tree, inh_load: Option<&MetaBlock>, inh_store: Option<&MetaBlock>)` | field_keyを再帰処理しpaths/children/leavesを構築 | dsl.rs |
|          | `resolve_meta` | `(&mut self, pairs: &[(Vec<u8>, Tree)], meta_key: &[u8], inherited: Option<&MetaBlock>) -> Option<MetaBlock>` | `_load`/`_store`ブロックを親から継承しつつ現keyで上書きして返す | dsl.rs |
|          | `write_leaf` | `(&mut self, path_idx: u32, keyword_idx: u32, value_idx: Option<u32>, load: Option<&MetaBlock>, store: Option<&MetaBlock>)` | leavesにleafデータを書き込みpaths[path_idx]をis_leaf=1で更新 | dsl.rs |
|          | `intern` | `(&mut self, s: &[u8]) -> u32` | バイト列をinterningに追加しinterning_idxを返す（重複排除） | dsl.rs |
|          | `intern_tree_scalar` | `(&mut self, v: &Tree) -> u32` | TreeスカラーまたはNullをinternしてindexを返す | dsl.rs |
|          | `push_u32` | `(&mut self, v: u32)` | u32leをleavesに追記 | dsl.rs |
|          | `finish` | `(self) -> (Box<[u64]>, Box<[u32]>, Box<[u8]>, Box<[u8]>, Box<[u64]>)` | 各Vecをboxed sliceに変換して返す | dsl.rs |
| Compiler (precompile) | `parse_yaml` | `(src: &[u8]) -> Result<Tree, String>` | YAMLバイト列をTreeにパース | dsl.rs |
|                       | `yaml_value_to_tree` | `(v: serde_yaml_ng::Value) -> Tree` | serde_yaml_ng::ValueをTreeに変換 | dsl.rs |
|                       | `emit_u64_slice` | `(out: &mut String, name: &str, data: &[u64])` | `&[u64]`をRustスタティック宣言として出力 | dsl.rs |
|                       | `emit_u32_slice` | `(out: &mut String, name: &str, data: &[u32])` | `&[u32]`をRustスタティック宣言として出力 | dsl.rs |
|                       | `emit_u8_slice` | `(out: &mut String, name: &str, data: &[u8])` | `&[u8]`をRustスタティック宣言として出力 | dsl.rs |
| Index | `find` | `(&self, path: &str) -> Option<u32>` | '.'区切りパスをルートからたどりpath_idxを返す | index.rs |
|       | `find_child` | `(&self, path_idx: u32, keyword: &[u8]) -> Option<u32>` | path_idxの子の中からkeywordに一致するpath_idxを返す | index.rs |
|       | `collect_leaves` | `(&self, path_idx: u32, out: &mut Vec<LeafRef>)` | path_idx以下の全leafをoutに再帰収集 | index.rs |
|       | `decode_meta` | `(&self, path_idx: u32, leaf_offset: u32, kind: MetaKind) -> (&str, BTreeMap<String, Tree>)` | leavesから`_load`または`_store`のclient名とargsを読み出す | index.rs |
|       | `read_u32` | `(&self, off: usize) -> u32` | leavesのoffからu32leを読む | index.rs |
|       | `interning_str` | `(&self, idx: usize) -> &[u8]` | interning_idxのidxからinterningのバイト列スライスを返す | index.rs |
| Tree | `write_value` | `(value: &Tree, buf: &mut Vec<u8>)` | Treeをワイヤフォーマットにシリアライズしbufに追記 | tree.rs |
|      | `read_value` | `(bytes: &[u8]) -> Option<(Tree, &[u8])>` | bytesの先頭からTreeをデシリアライズし残バイトと返す | tree.rs |
|      | `read_u32` | `(bytes: &[u8]) -> Option<(usize, &[u8])>` | bytesの先頭4バイトをu32leとして読みusizeで返す | tree.rs |
|      | `split_at` | `(bytes: &[u8], n: usize) -> Option<(&[u8], &[u8])>` | bytesをn位置で分割しNoneを安全に返す | tree.rs |
| Context | `cache_get` | `(&self, path_idx: u32) -> Option<&Tree>` | インスタンスキャッシュからpath_idxの値を返す | context.rs |
|         | `cache_set` | `(&mut self, path_idx: u32, value: Tree)` | インスタンスキャッシュにpath_idxの値を書き込む（上書き） | context.rs |
|         | `cache_remove` | `(&mut self, path_idx: u32)` | インスタンスキャッシュのpath_idxエントリをNullで無効化 | context.rs |
|         | `guard_recursion` | `(&self, path_idx: u32) -> Result<(), ContextError>` | called_pathsの重複・上限超過を検出しエラーを返す | context.rs |
|         | `resolve_leaf` | `(&mut self, path_idx: u32, leaf_offset: u32) -> Result<Option<Tree>, ContextError>` | cache→_store→_loadの順で値を解決しwrite-throughする | context.rs |

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

## モジュール仕様

### Store

単一ストアの操作を提供するtrait。`key`は予約引数として明示し、追加の任意引数は`args`のflatなBTreeMapで渡す。

- `identity`,`index`: DSL の `_get.{identity,index}` / `_set.{identity,index}` の値。予約引数。
- `args`: ttl・connection・map 等、ストア種別ごとの任意引数。利用者がimpl内で定義・参照する。
- 内部可変性・スレッド安全性はimplementor側の責任。

### StoreRegistry

YAMLの`store:`名称とStoreの対応を管理するtrait。利用者がimplし、Contextに渡す。

```rust
pub trait StoreRegistry {
    fn store_for(&self, keyword: &str) -> Option<&dyn Store>;
}
```

- ライブラリはYAML名称の文字列をそのまま`store_for()`に渡してmatchを回す。
- YAML上の名義（`"Memory"`, `"Kvs"`, `"TenantDb"`等）は利用者が自由に定義する。

### Instance Cache

Contextインスタンス固有のキャッシュ。Storeとは独立。

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

## データ構造仕様

### 静的データ配列

`Dsl::compile`が返す5配列。アプリ起動時に一度だけ構築し、`Index`が保持する。
**読み込むdslの、全(部分含む)path数は、u16(65535個以下)を充てる。**

```
paths:         Box<[u64]>   // pathのlist
children:      Box<[u16]>   // 各pathの子path_idxを連結したu16列
leaves:        Box<[u32]>   // leafのlist。u32刻み
interning:     Box<[u8]>    // 文字列バイト列を連結したバイト列
interning_idx: Box<[u64]>   // interningの文字列境界 interning_idx[u64] のlist
```

### path (u64)

**paths[0]は常にvirtual root**

| Field       | bits | range       |
|-------------|------|-------------|
| is_leaf     |    1 | bit 63      |
| offset      |   16 | bits 62..47 |
| count       |    4 | bits 46..43 |
| padding     |   11 | bits 42..32 |
| parent_idx  |   16 | bits 31..16 |
| keyword_idx |   16 | bits 15..0  |

- `is_leaf=0`: 非leaf path。`children[offset..offset+count]`に子path_idxが並ぶ
- `is_leaf=1`: leaf path。`leaves[offset..]`にleafデータが並ぶ。`count`は未使用
- `parent_idx`: 親path_idx。virtual root(paths[0])は自己参照(0)
- `keyword_idx`: このpathのkeywordのinterning_idx

### child (u16)

| Field     | Bits | Range      |
|-----------|------|------------|
| child_idx |   16 | bits 15..0 | // path_idx

各path所属の始端終端は、`path.offset`と`path.count[3:0]`で決まる。
**1pathあたりの直接子path数は、count[3:0]の4bit制限により最大15。**

### leaf

leaf 1つ分のレイアウト（u32単位）:

| Category    | Field                | Bits | Range                           |
|-------------|----------------------|------|---------------------------------|
| header      | keyword_idx          |   16 | u32[0] bits 31..16              | // interning_idx
| header      | fragment_count       |    8 | u32[0] bits 15..8               | // valueフラグメント数。0=null
| header      | load_map_count       |    8 | u32[0] bits 7..0                | // load.mapエントリ数
| header      | load_args_count      |    8 | u32[1] bits 31..24              | // load.argsエントリ数
| header      | store_map_count      |    8 | u32[1] bits 23..16              | // store.mapエントリ数
| header      | store_args_count     |    8 | u32[1] bits 15..8               | // store.argsエントリ数
| header      | padding              |    8 | u32[1] bits 7..0                |
| header      | load_client_idx      |   16 | u32[2] bits 31..16              | // interning_idx
| header      | load_key_idx         |   16 | u32[2] bits 15..0               | // interning_idx
| header      | store_client_idx     |   16 | u32[3] bits 31..16              | // interning_idx
| header      | store_key_idx        |   16 | u32[3] bits 15..0               | // interning_idx
| fragment×F  | padding              |   15 | u32[4+i] bits 31..17            |
| fragment×F  | is_placeholder       |    1 | u32[4+i] bit 16                 | // 0=static, 1=placeholder
| fragment×F  | idx                  |   16 | u32[4+i] bits 15..0             | // is_placeholder=0: interning_idx / 1: path_idx
| load.map×M0 | dst_idx              |   16 | u32[4+F+i] bits 31..16          | // context path interning_idx
| load.map×M0 | src_idx              |   16 | u32[4+F+i] bits 15..0           | // store column interning_idx
| load.args×A0| key_idx              |   16 | u32[4+F+M0+i] bits 31..16       | // interning_idx
| load.args×A0| val_idx              |   16 | u32[4+F+M0+i] bits 15..0        | // interning_idx
| store.map×M1| dst_idx              |   16 | u32[4+F+M0+A0+i] bits 31..16    | // context path interning_idx
| store.map×M1| src_idx              |   16 | u32[4+F+M0+A0+i] bits 15..0     | // store column interning_idx
| store.args×A1| key_idx             |   16 | u32[4+F+M0+A0+M1+i] bits 31..16 | // interning_idx
| store.args×A1| val_idx             |   16 | u32[4+F+M0+A0+M1+i] bits 15..0  | // interning_idx

// F=fragment_count, M0=load_map_count, A0=load_args_count, M1=store_map_count, A1=store_args_count

**valueの解釈:**
- `fragment_count=0`: null
- `fragment_count=1, is_placeholder=0`: 静的文字列
- `fragment_count=1, is_placeholder=1`: 単独`${path}` → `Context.get(path_idx)`の値をそのままコピー（型保持）
- `fragment_count≥2` または混在: template → 各fragmentを解決しstring結合


### interning_idx ([u64])

| Field  | Bits | Range  |
|--------|------|--------|
| offset |   32 | 63..32 |
| padding|   16 | 31..16 |
| len    |   16 | 15..0  |

**1文字列の最大長はu16(65535バイト以下)を充てる。**

インデックス0は空文字列（virtual rootのkeyword）。

## Placeholder Resolution Rules

`${}`内のパスは常に絶対パスとして扱う。

**実行時の解決:**
- `fragment_count=1, is_placeholder=1`: `Context.get(path_idx)`の値をそのままコピー（string化しない）
- template: 各fragmentを`Context.get()`で解決しstringとして結合

## Error Types

**ParseError** (`ports/provided.rs`):
- `FileNotFound(String)`
- `AmbiguousFile(String)`
- `ParseError(String)`

**LoadError** (`ports/provided.rs`):
- `ClientNotFound(String)` — `StoreRegistry::store_for()` が None を返した
- `ConfigMissing(String)` — DSL内に必須のconfigキーが欠落
- `NotFound(String)` — clientの呼び出しは成功したがデータが存在しなかった
- `ParseError(String)` — clientレスポンスのパースエラー

**StoreError** (`ports/provided.rs`):
- `ClientNotFound(String)` — `StoreRegistry::store_for()` が None を返した
- `ConfigMissing(String)` — DSL内に必須のconfigキーが欠落
- `SerializeError(String)` — シリアライズエラー

**ContextError** (`ports/provided.rs`):
- `ParseFailed(String)`
- `KeyNotFound(String)`
- `RecursionLimitExceeded`
- `StoreFailed(StoreError)`
- `LoadFailed(LoadError)`
