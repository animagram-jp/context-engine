// This file includes untranslated text (ja).

# Architecture

## Requirement

- [README.md:3](../README.md#context-engine)
- システムが認識するべき概念を階層構造の名前空間で表現できたとする。この時、名前空間から導かれる全通りの(部分含む)パスが、ランタイムの単一処理スコープで操作する可能性のある値のキーを網羅している。このキー群の値全てを、DSLにて漏れなく取得方法の定義を宣言する。

## Function

- parse: YAMLをパースする
- compile: dslから、n次元疎集合割り出しの最適解である、固定長メモリ位置群のトラバーサルに落とし込むための静的データ群を生成する
- traversal: 上記データ群を保持し、トラバーサルによってメモリ位置群を取得する
- addressing & operation: Manifestに対応した1層mapを保持し、アプリケーションからの呼び出しに応じて値の操作を行う。リクエスト処理スコープインスタンスで行い、リクエストを跨いで保持したい場合は全て_setにて指示する

## Port

| Module | Port | Signature | Description | Filename |
|--------|------|-----------|-------------|----------|
| - | `debug_log!` | `(class, fn $(, arg)*)` | `feature=logging` ログマクロ | debug_log.rs |
| - | `Tree` | - | n次元scalar map型 | provided.rs |
| - | `SetOutcome` | - | `Store::set`が返すCreated(usize)かUpdated | required.rs |
| Tree | `wire` | `(&self) -> Vec<u8>` | Treeをワイヤフォーマットに変換 | tree.rs |
|      | `unwire` | `(bytes: &[u8]) -> Option<Tree>` | ワイヤフォーマットからTreeへ変換 | tree.rs |
| Dsl | `compile` | `(tree: &Tree, store_ids: &[&str]) -> (paths, children, leaves, values, words, map_keys, map_vals, args_keys, args_vals)` | Treeと利用者定義store識別子リストから走査してcompile済みdslを返す | dsl.rs |
|  | `write` | `(src: &[u8], store_ids: &[&str], out_path: &str) -> Result<(), String>` | YAMLファイルパスから.rsを出力[precompile] | dsl.rs |
| Index | `new` | `(paths, children, leaves, values, words, map_keys, map_vals, args_keys, args_vals) -> Index` | compile済みdslからIndexを構築 | index.rs |
|       | `traverse` | `(&self, path: &str) -> Box<[LeafRef]>` | パス文字列からleaf参照リストを返す | index.rs |
|       | `keyword_of` | `(&self, path_id: u16) -> &[u8]` | path_idからkeywordバイト列を返す | index.rs |
|       | `get_meta` | `(&self, leaf: &LeafRef) -> (u8, &[u16], &[u16], &[u16], &[u16], &[u16])` | `_get`ブロックのstore_id含む継承済みDslデータを返す | index.rs |
|       | `set_meta` | `(&self, leaf: &LeafRef) -> (u8, &[u16], &[u16], &[u16], &[u16], &[u16])` | `_set`ブロックのstore_id含む継承済みDslデータを返す | index.rs |
| Context | `new` | `(index: Arc<Index>, stores: &'r dyn Stores) -> Context` | IndexとStoresからContextを構築 | context.rs |
|         | `get` | `(&mut self, key: &str) -> Result<Option<Tree>, ContextError>` | パス文字列から値(cache→_set→_get)を取得して返す | context.rs |
|         | `set` | `(&mut self, key: &str, value: Tree) -> Result<bool, ContextError>` | 値から_setに書き込み、cacheも更新 | context.rs |
|         | `delete` | `(&mut self, key: &str) -> Result<bool, ContextError>` | パスから_setの値を削除し、cacheもnullで更新 | context.rs |
|         | `exists` | `(&mut self, key: &str) -> Result<bool, ContextError>` | パスからcacheか_setに値が存在するか確認し、cacheを更新 | context.rs |
| Store | `get` | `(&self, key: &[u8], args: &BTreeMap<&str, Tree>) -> Option<Tree>` | keyとargsから値を返す | required.rs |
|       | `set` | `(&self, key: &[u8], args: &BTreeMap<&str, Tree>) -> Option<SetOutcome>` | keyとargsから値を保存しSetOutcomeを返す | required.rs |
|       | `delete` | `(&self, key: &[u8], args: &BTreeMap<&str, Tree>) -> bool` | keyとargsから値を削除し成否を返す | required.rs |
| Stores | `store_for` | `(&self, id: u8) -> Option<&dyn Store>` | store_idからStore実装を返す | required.rs |
| DslError | `fmt` | `&self, f: &mut fmt::Formatter<'_> -> fmt::Result` | Dslのエラーを返す | provided.rs |
| LoadError | `fmt` |  | _getクライアント呼び出しエラーを返す | provided.rs |
| StoreError | `fmt` |  | _setクライアント呼び出しエラーを返す | provided.rs |
| ContextError | `fmt` |  | Contextの出力するエラーを返す | provided.rs |


## プライベートfn構成

| Module | fn | Signature | Description | Filename |
|--------|----|-----------|-------------|----------|
| Compiler | `new` | `(store_ids: &[&str]) -> Compiler` | Compiler初期化 | dsl.rs |
|          | `walk_field_key` | `(&mut self, keyword: &[u8], value: &Tree, parent_id: u16, inh_get: Option<&MetaBlock>, inh_set: Option<&MetaBlock>)` | field_keyを再帰処理しpaths/children/leavesを構築 | dsl.rs |
|          | `resolve_meta` | `(&mut self, pairs: &[(Vec<u8>, Tree)], meta_key: &[u8], inherited: Option<&MetaBlock>, current_path_id: u16) -> Option<MetaBlock>` | `_get`/`_set`ブロックを親から継承しつつ現keyで上書きして返す。`store`を上書きした場合は`map`をクリアする | dsl.rs |
|          | `write_leaf` | `(&mut self, path_id: u16, keyword_id: u16, parent_id: u16, value: &Tree, get: Option<&MetaBlock>, set: Option<&MetaBlock>)` | leavesにleafデータを書き込みpaths[path_id]をis_leaf=1で更新 | dsl.rs |
|          | `encode_meta` | `(&mut self, meta: Option<&MetaBlock>) -> (u16, u16, u16, u16, u16, u16)` | MetaBlockを(store_id, key_id, map_key_id, map_val_id, args_key_id, args_val_id)に変換 | dsl.rs |
|          | `encode_value` | `(&mut self, value: &Tree) -> Vec<u16>` | Tree値をvalue fragment u16列に変換 | dsl.rs |
|          | `intern_word` | `(&mut self, s: &[u8]) -> u16` | バイト列をwordsにintern（重複排除）しword_idを返す | dsl.rs |
|          | `resolve_store_id` | `(&self, name: &[u8]) -> u8` | store名をstore_idsで検索し1-based store_idを返す（未登録=0） | dsl.rs |
|          | `alloc_children_slots` | `(&mut self, count: usize) -> usize` | children内にcount個の空スロットを確保しchildren_idを返す | dsl.rs |
|          | `set_child` | `(&mut self, children_id: usize, slot: usize, path_id: u16)` | children_idのslot番目にpath_idを書き込む | dsl.rs |
|          | `finish` | `(self) -> (List<u64>, VariableList<u16>, VariableList<u16>, VariableList<u16>, VariableList<u8>, VariableList<u16>, VariableList<u16>, VariableList<u16>, VariableList<u16>)` | 各フィールドをそのまま返す | dsl.rs |
| Compiler (precompile) | `parse_yaml` | `(src: &[u8]) -> Result<Tree, String>` | YAMLバイト列をTreeにパース | dsl.rs |
|                       | `yaml_value_to_tree` | `(v: serde_yaml_ng::Value) -> Tree` | serde_yaml_ng::ValueをTreeに変換 | dsl.rs |
|                       | `emit_u64_slice` | `(out: &mut String, name: &str, data: &[u64])` | `&[u64]`をRustスタティック宣言として出力 | dsl.rs |
|                       | `emit_u16_slice` | `(out: &mut String, name: &str, data: &[u16])` | `&[u16]`をRustスタティック宣言として出力 | dsl.rs |
|                       | `emit_u8_slice` | `(out: &mut String, name: &str, data: &[u8])` | `&[u8]`をRustスタティック宣言として出力 | dsl.rs |
| Index | `find` | `(&self, path: &str) -> Option<u16>` | '.'区切りパスをルートからたどりpath_idを返す | index.rs |
|       | `find_child` | `(&self, path_id: u16, keyword: &[u8]) -> Option<u16>` | path_idの子の中からkeywordに一致するpath_idを返す | index.rs |
|       | `collect_leaves` | `(&self, path_id: u16, out: &mut Vec<LeafRef>)` | path_id以下の全leafをoutに再帰収集 | index.rs |
|       | `decode_meta` | `(&self, leaf: &LeafRef, kind: MetaKind) -> (u8, BTreeMap<String, Tree>)` | leavesから`_get`または`_set`のstore_idとargsを読み出す | index.rs |
|       | `resolve_static_frags` | `(&self, frags: &[u16]) -> String` | placeholder非含有のfragment列を文字列に結合する | index.rs |
|       | `word_bytes` | `(&self, id: usize) -> &[u8]` | word_idからwordsのバイト列スライスを返す | index.rs |
|       | `vl_slice_u16` | `(&self, vl: &VariableList<u16>, id: usize) -> Option<&[u16]>` | VariableList<u16>のidエントリのスライスを返す | index.rs |
| Tree | `write_value` | `(value: &Tree, buf: &mut Vec<u8>)` | Treeをワイヤフォーマットにシリアライズしbufに追記 | tree.rs |
|      | `read_value` | `(bytes: &[u8]) -> Option<(Tree, &[u8])>` | bytesの先頭からTreeをデシリアライズし残バイトと返す | tree.rs |
|      | `read_u32` | `(bytes: &[u8]) -> Option<(usize, &[u8])>` | bytesの先頭4バイトをu32leとして読みusizeで返す | tree.rs |
|      | `split_at` | `(bytes: &[u8], n: usize) -> Option<(&[u8], &[u8])>` | bytesをn位置で分割しNoneを安全に返す | tree.rs |
| Context | `cache_get` | `(&self, path_id: u16) -> Option<&Tree>` | インスタンスキャッシュからpath_idの値を返す | context.rs |
|         | `cache_set` | `(&mut self, path_id: u16, value: Tree)` | インスタンスキャッシュにpath_idの値を書き込む（上書き） | context.rs |
|         | `cache_remove` | `(&mut self, path_id: u16)` | インスタンスキャッシュのpath_idエントリをNullで無効化 | context.rs |
|         | `guard_recursion` | `(&self, path_id: u16) -> Result<(), ContextError>` | called_pathsの重複・上限超過を検出しエラーを返す | context.rs |
|         | `resolve_leaf` | `(&mut self, path_id: u16, leaf_id: u16, value_id: u16) -> Result<Option<Tree>, ContextError>` | cache→_set→value fragments→_getの順で値を解決しwrite-throughする | context.rs |

## Terms

- key:            n層マップDSLの最末端value以外の要素
- keyword:        keyの名前文字列
- field_key:      自身と親祖先のkeywordが'_'で始まらないkey
- meta_key:       keywordが'_'始まりのkeyと、その子孫key (_get, _set, _state)
- leaf_key:       子にkeyを持たず値を持つkey
- value:          leaf_keyの値。DSL内で省略された場合はnullが充てられる
- path:           単一のfield_keyを表す、'.'区切りkeywordのチェーン
- qualified_path: DSL内で一意な完全修飾パス
- placeholder:    key参照記述("${path}")。valueのみに適用。単独記述時はis_template=falseとして扱い、値をそのままコピーする（string化しない）
- template:       placeholderと静的な文字列を混合した動的生成文字列。valueのみに適用。is_template=trueとして扱い、解決時にstring化する
- called_path:    Context.get()等に渡されるパス文字列

## モジュール仕様

### Store

単一ストアの操作を提供するtrait。`key`,`map`は予約引数として明示し、追加の任意引数は`args`のflatなBTreeMapで渡す。

- `key`: DSL の `_get.identity` / `_set.identity` の値。予約引数。
- `args`: ttl・connection・map 等、ストア種別ごとの任意引数。利用者がimpl内で定義・参照する。
- 内部可変性・スレッド安全性はimplementor側の責任。
- YAML上のkeyword（`"Memory"`, `"Kvs"`, `"TenantDb"`等）は利用者が自由に定義する。

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
4. `_set` store に問い合わせ
5. miss時、`_get` store で自動ロード → write-through to `_set`
6. `Ok(Some(value))` / `Ok(None)` / `Err(ContextError)` を返却

## データ構造仕様

### compile済みdsl

`Dsl::compile`が構築し、`Index`が保持する。

### Limitation

```
path_id:  16 bit (max 65535)
word_id:  15 bit (max 32767)
store_id:  8 bit (max   255)
```

### データ構造一覧

```rust
paths:     List<u64>          // path一覧
children:  VariableList<u16>  // 各pathの子path_id列
leaves:    VariableList<u16>  // leaf固有データ (固定12u16)
values:    VariableList<u16>  // value fragments. each u16: is_placeholder(bit15) | word_id(bits14..0)
words:     VariableList<u8>   // keyword/path/value文字列intern pool. intern=true
map_keys:  VariableList<u16>  // map dst path_id列 (compile時2パス目でword_id→path_idに解決済み)
map_vals:  VariableList<u16>  // map src word_id列
args_keys: VariableList<u16>  // args key word_id列
args_vals: VariableList<u16>  // args val values_id列
```

### path (List<u64>)

paths[0]は常にvirtual root（自己参照）。

| Field               | Bits | Note                                                                      |
|---------------------|------|---------------------------------------------------------------------------|
| is_leaf             |    1 | 0=非leaf, 1=leaf                                                          |
| keyword_id          |   15 | words id                                                                  |
| parent_id           |   16 | paths id                                                                  |
| children_id/leaf_id |   16 | is_leaf=0: children id → 子path_id列 / is_leaf=1: leaves id → leaf data  |
| value_id            |   16 | values id. 0=null                                                         |

### leaf (VariableList<u16>、固定12u16)

0=none。

| Field            | u16s | Note                            |
|------------------|------|---------------------------------|
| get_store_id     |    1 | stores id (u8 as u16)           |
| get_key_id       |    1 | values_id                       |
| set_store_id     |    1 | stores id (u8 as u16)           |
| set_key_id       |    1 | values_id                       |
| get_map_key_id   |    1 | map_keys id → dst path_id列     |
| get_map_val_id   |    1 | map_vals id → src word_id列     |
| get_args_key_id  |    1 | args_keys id → key word_id列    |
| get_args_val_id  |    1 | args_vals id → val values_id列  |
| set_map_key_id   |    1 | map_keys id → dst path_id列     |
| set_map_val_id   |    1 | map_vals id → src word_id列     |
| set_args_key_id  |    1 | args_keys id → key word_id列    |
| set_args_val_id  |    1 | args_vals id → val values_id列  |

### value (VariableList<u16>)

各u16: `is_placeholder(bit15) | word_id(bits14..0)`

- len=0: null
- len=1, is_placeholder=0: 静的文字列
- len=1, is_placeholder=1: 単独placeholder → `Context.get(word)`の値をそのままコピー（型保持）
- len≥2: template → 各fragmentを`Context.get()`で解決しstring結合

## Placeholder Resolution Rules

`${}`内のパスは常に絶対パスとして扱う。self-callingの再帰検出はContextの`guard_recursion`が担保。

## Error Types

**ParseError** (`provided.rs`):
- `FileNotFound(String)`
- `AmbiguousFile(String)`
- `ParseError(String)`

**LoadError** (`provided.rs`):
- `ClientNotFound(String)` — `Stores::store_for()` が None を返した
- `ConfigMissing(String)` — DSL内に必須のconfigキーが欠落
- `NotFound(String)` — storeの呼び出しは成功したがデータが存在しなかった
- `ParseError(String)` — storeレスポンスのパースエラー

**StoreError** (`provided.rs`):
- `ClientNotFound(String)` — `Stores::store_for()` が None を返した
- `ConfigMissing(String)` — DSL内に必須のconfigキーが欠落
- `SerializeError(String)` — シリアライズエラー

**ContextError** (`provided.rs`):
- `ParseFailed(String)`
- `KeyNotFound(String)`
- `RecursionLimitExceeded`
- `StoreFailed(StoreError)`
- `LoadFailed(LoadError)`
