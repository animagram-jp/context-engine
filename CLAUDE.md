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

**libraryが感知するメタデータ:**
- `_state` - 型定義（type, keys, values等）
- `_store` - 保存先定義（client, key, ttl等）
- `_load` - 読込元定義（client, connection, table, map等）

**基本構造:**
```yaml
node:
  _state:
    type: map  # integer/float/string/boolean/null/list/map
  _store:
    client: KVS  # Env/InMemory/KVS/DB/API
    key: "namespace.${variable}"
    ttl: 3600  # KVS使用時のみ（optional）
  _load:
    client: DB  # Env/InMemory/KVS/DB/API
    connection: common
    table: "table_name"
    where: "id=${variable}"
    map:
      field: "column"

  child_node:
    _state:
      type: string
```

**設計原則:**
- すべてのlibrary制御メタデータは `_state/_store/_load` 内に格納
- `_key`, `_keyPrefix`, `_ttl` 等をトップレベルに配置しない
- app固有の補助メタデータが必要なら `_app` 以下に格納（library非感知）

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

## Ports - インターフェース設計

### Provided Ports (ライブラリが提供)

1. **Manifest** - YAMLファイル管理
   - `get(key, default)` - データ取得（メタデータ除外）
   - `get_meta(key)` - メタデータ取得（継承あり）

2. **State** - 統一CRUD実装（唯一の外部インターフェース）
   - `get(key)` - 状態取得（miss時自動ロード）
   - `set(key, value, ttl)` - 状態設定
   - `delete(key)` - 状態削除

### Required Ports (app側が実装)

1. **InMemoryClient** - プロセスメモリ操作
2. **KVSClient** - KVS操作（Redis等）
3. **DBClient** - DB操作
4. **ENVClient** - 環境変数取得
5. **APIClient** - 外部API呼び出し
6. **ExpressionClient** - 式評価（app固有ロジック）
7. **DBConnectionConfigConverter** - DB接続設定変換

## tree

src/
  ├── lib.rs                    # ライブラリルート
  ├── common/
  │   ├── dot_array_accessor.rs # ドット記法アクセサ
  │   └── placeholder_resolver.rs # プレースホルダー処理
  ├── common.rs                 # commonモジュール
  ├── manifest/
  │   └── mod.rs                # Manifest実装
  ├── state/
  │   └── mod.rs                # State実装（TODO）
  ├── load/
  │   └── mod.rs                # Load実装（TODO）
  └── ports/
      ├── mod.rs                # Portsモジュール
      ├── provided.rs           # Provided Ports (Manifest, State)
      └── required.rs           # Required Ports (各種Client)


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

## placeholder規則

**採用形式: `${variable}`**

業界標準の `${variable}` 形式を採用する。

**設計原則:**
- `${}` は予約語として扱う（エスケープサポートは簡略化）
- YAML DSLとして割り切る（通常の文字列としての `${}` 使用は想定外）
- プレースホルダー解決は各clientの責務
- 再帰置換は防止する（置換後の値が再度置換されない）

**使用例:**
```yaml
user:
  _key: "user:${sso_user_id}"
  tenant_id:
    _load:
      client: DB
      table: users
      where: "id=${user_id}"
```

**実装方針:**
- commonモジュールに純粋なロジックとしてPlaceholderResolver実装
- 依存関係を持たない文字列処理ユーティリティ
- 値の解決は呼び出し側の責務

**PlaceholderResolver採用検討ポイント:**

1. **API設計**
   - `extractPlaceholders(template: &str) -> Vec<String>` - テンプレートから変数名を抽出
   - `replace(template: &str, params: &HashMap<String, String>) -> String` - 値で置換
   - `replace_in_map(values: HashMap, params: HashMap) -> HashMap` - マップ内再帰置換

2. **置換アルゴリズム**
   - PHP版は `strtr()` で再帰置換を防止（`${a}` → `${b}` → `${b}` で停止）
   - Rust版でも同等の挙動を実装する必要あり
   - 未定義のプレースホルダーは置換せずそのまま残す

3. **エスケープ簡略化の判断**
   - `${}` を予約語として扱う
   - エスケープ記法（`\${var}`等）はサポートしない
   - YAML DSL専用として割り切り、通常の文字列としての `${}` 使用は想定外
   - `$` 単体や `{}`単体は問題なし（`${}`形式のみマッチ）

4. **型保持**
   - 配列内の文字列以外の型（integer/boolean/null）は保持
   - 文字列のみ置換対象とする

5. **責務分担**
   - **PlaceholderResolver**: 純粋な文字列処理（依存なし）
   - **ParameterBuilder**: 値の解決ロジック（UserKey, InMemory等へのアクセス）
   - **State/Load**: PlaceholderResolverを呼び出して実行時に置換

6. **パフォーマンス考慮**
   - 置換マップを事前構築（O(m), m=プレースホルダー数）
   - 配列の再帰処理での効率的な実装（不要なcloneを避ける）

## 責務分離の原則

### 各層の責務

| クラス | 責務 | 扱うもの |
|--------|------|----------|
| **Manifest** | YAMLメタデータ管理 | ファイル読み込み、メタデータ継承、ドット記法アクセス |
| **State** | CRUD統一インターフェース | get/set/delete、自動ロード呼び出し、型変換 |
| **Load** | 自動ロード専用 | _load設定に従った各種clientからのデータ取得 |

### データフロー

```
state::get('filename.node')
  ↓
manifest::getMeta() で _state/_store/_load を取得
  ↓
_store から値を取得（hit: 返却, miss: 次へ）
  ↓
load::handle() で _load に従い自動ロード
  ↓
ロード成功 → _store に保存して返却
```

### 設計意図

- **_state**: 「何を」保存するか（型定義）
- **_load**: 「どこから」読むか（ソース定義）
- **_store**: 「どこへ」書くか（保存先定義）

この分離により、任意のソース→任意のストアの組み合わせを自由に記述可能。

# todo

- declare-eのclient無し_load.keyでloadを呼ばない実装取り込み