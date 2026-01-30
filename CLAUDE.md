# state-engine 開発プロジェクト

state-engine library
- 指定dir以下の任意のyamlファイルを読み込み(manifest/*.yml, manifestクラス)
- アプリソースの呼び出しに従いCRUDを提供する(stateクラス※)
- ライブラリ独自の_prefixメタデータに従ったmulti store, multi source, multi stateを自動維持し(loadクラス※)
- declare-engine(php旧版)に対して、stateはreadmeの通りarchitectureレベルで大きく前進している。

※命名は検討余地あり

## to read

- README.md

# to do

- 

## manifest/*.yml

```yaml
node:
  _state:
    type:
  _store:
    client: {ENV, DB, KVS, Processmermory, API}
    map:
  _load:
    client: {ENV, DB, KVS, Processmermory, API}
    map:
  node:
    _state:
    _store:
    _load:
...
```

## manifest::class

### manifest::get('filename.node')

manifest/*.ymlの_prefixブロック(以下、メタデータブロック)を無視したnode構造の読み出し(指定nodeが最下位なら値(記述が無ければnull), それ以外は配下のnode全てを表現したcollectionデータを返却)。指定nodeがyml上で存在しなければ(miss)、nullを返却する。'filename.node'には'filename.*'の意味で'filename'を指定することが出来る。メタデータブロック以下を指定することもできるが、想定はされていない。

### manifest::getMeta('filename.node')

manifest/*.ymlの指定nodeのメタデータブロックを返却する。この時、該当yml fileのroot nodeから指定nodeまでのメタデータブロックを順に全て読み出し、より子の同キーで上書きしたlistを返却する。指定nodeがyml上で存在しなければ(miss)、nullを返却する。'filename.node'には'filename.*'の意味で'filename'を指定することが出来、この場合はnodeに属さない最上位のメタデータブロックのみがlistに格納されて返却される。

## state::class

### state::get('filename.node')

manifest/*.ymlの指定nodeが表すstate(_stateに記述)を参照し、value、もしくは配下のcollectionsを返却する。指定nodeが表すstate keyがmissした場合、load::handle()にて自動ロードを試行後、再度valueもしくはcollectionsを返却する。load::handle()のerror時はnullを返却する※

※load::handle()のerrorについては、専用のExceptionクラスでdebug情報を返却する

### state:set('filename.node', $value, ?$ttl)

指定nodeの表すstateに対して値をsetする。操作先storeがKVSであれば、ttlのオプションを設定でき、操作先KVS recordのttlが再設定される。無指定では、ttlはymlにデフォルト値が設定されていれば再設定し、無ければ操作時点の値を維持する※

※ ttl挙動は議論の余地あり

### state:delete('filename.node')

指定nodeが表すstateの該当部を{key:value}レコードごと削除する。削除後、同nodeはmiss hitを示す。

### load::handle()

state::get('filename.node')がmiss valueした時、manifest::getMeta('filename.node')の_storeと_loadの記述内容に従って自動loadを行う。一次的な解決が出来ない場合、自己再帰する※

※ 無限再帰によるprocess errorについて、事前のyml静的解析の他、呼出回数のinstance var管理など議論の余地あり

## tree

src/        
  ├── lib.rs                    # ライブラリルート
  ├── common/
  │   └── dot_array_accessor.rs # ドット記法アクセサ
  ├── common.rs                 # commonモジュール
  ├── manifest/ 
  │   └── mod.rs                # Manifest実装 + Provided::Manifest trait実装
  └── ports/
    ├── mod.rs                # Portsモジュール
    ├── provided.rs           # ライブラリが提供するインターフェース
    └── required.rs           # アプリが実装すべきインターフェース  


## note

### declare-engine版yaml / state-engine版yaml 互換性について

**メタデータキーの統一:**
- `source` → `client` (InMemory/Env/KVS/DB/API)
- `_type` → `_state.type` (integer/float/string/boolean/null/list/map)
- `_state.type: map` は子要素がある場合省略可(自明)

**client種別:**
- `Env` - 起動時確定の読み取り専用設定(環境変数・設定ファイル等)
- `InMemory` - 実行時の可変メモリ(request scope等、worker間非共有)
- `KVS` - Key-Value Store(Redis等)
- `DB` - Database
- `API` - 外部API呼び出し