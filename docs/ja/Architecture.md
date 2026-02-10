# Architecture

## index

```yaml
# modules list
Ports:
  Provided: {Manifest, State}
  Required: {InMemoryClient, DBClient, KVSClient, ENVClient}

Manifest:
State:

Load:

Common:
  DotArrayAccessor:
  PlaceholderResover:
  LogFormat:
```

---

## Ports

ライブラリの外部向けインターフェース定義modules

1. Provided Port

ライブラリ提供moduleのtraits

  1. **Manifest** - YAMLファイルの読み込みと集計をするmodule。'_'始まりのキー以下(メタブロック)を認識し、get()メソッドでは無視したcollectionを返却、getMeta()では親から子に継承と上書きをしながら集計し返却する。収集時、メタブロック内の_load.map.*のキー値は、YAMLファイルのfilename.key1.key2.,....(絶対パス)に変換する。
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
    - 渡される引数: `'key': Manifestの_{store,load}.key:の値`, `value: string(storeブロックのみ)`, `ttl: Manifestの_{store,load}.ttl:の値(オプション)`
    - 想定対象ストア: Key-Valueストア
    - Stateは_store.keyの定義されたkeyからのcollection objを、serialize/desirializeして1つのstringとして格納します。
  3. **DBClient**
    - 必要なメソッド: `fetch()`
    - 渡される引数: `'connection': YAML記載の_{store,load}.connection:の値`, `'table': YAML記載の_{store,load}.table:の値}`, `'columns': YAML記載の_{store,load}.map.*:の値`, `'where_clause': YAML記載の_{store,load}.where:の値`
    - 想定対象ストア: SQLデータベース
    - _load.client: のみに使用対応
  4. **ENVClient**
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
5. **State.cache (インスタンスHashMap) をチェック** ← 最優先
6. ストア (KVS/InMemoryClient) から値を取得
7. データから個別フィールドを抽出
8. **miss時、`Load::handle()` で自動ロード**
9. `_state.type` に従って型キャスト

**自動ロード:**
- 指定されたノードのステートキーがmissした場合、`Load::handle()` で自動取得を試みる
- `Load::handle()` がエラーの場合、`None` を返す

**型キャスト:**
```yaml
tenant_id:
  _state:
    type: integer  # 自動的にintにキャスト
```

---

### State::set('filename.node', value, ttl)

指定されたノードが表すステートに値をセットする。

**動作:**
- 永続ストア (KVS/InMemoryClient) に保存
- State.cache にも保存（高速化のため）
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
- 最初に State.cache をチェック（最速）
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
- `ENV` - 環境変数からロード
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

## error case

- manifestDir/{*.yml,*.yaml}の中に、拡張子違いの2つの同名ファイルが存在する
  - エラータイミング: Manifest moduleが該当2ファイルを読んで題意を検知した時
  - 理由: ドット区切りを階層表現とするManifestは、拡張子を無視するため、該当の同名ファイルを区別出来ないため
  - 備考: 同拡張子の同名ファイルはOSレベルでの非許容を想定して確認していない