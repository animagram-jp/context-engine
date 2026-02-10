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
  2. **State** - Manifest::getMeta()から取得する_storeブロックの記述に基づいて格納されるステートデータ(state obj)を対象に、`get()` / `set()` / `delete()`操作を行うmodule。`get()`では、key miss hitをトリガーとして、同じく取得した`_load`ブロックの記述に基づいてロード試行を自動的に行う。`set()`は指定のkeyに値をsetする。自動ロードは引き起こさない。`delete()`は指定のkeyと、そのvalue全てを削除する。Stateは、インスタンスメモリの`cahe`にYAMLファイル記述に従ったcollection型でstate objをキャッシュし、動作中、同期処理を行う。

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

1. `get(key: &str)` -> Option<Value>

2. `set(key: &str, value: Value, ttl: Option<u64>)` -> bool

3. `delete(key: &str)` -> bool

4. `exists(key: &str)` -> bool

自動ロードをトリガーしないkey hit/missの判定を返却する

## error case

- manifestDir/{*.yml,*.yaml}の中に、拡張子違いの2つの同名ファイルが存在する
  - エラータイミング: Manifest moduleが該当2ファイルを読んで題意を検知した時
  - 理由: ドット区切りを階層表現とするManifestは、拡張子を無視するため、該当の同名ファイルを区別出来ないため
  - 備考: 同拡張子の同名ファイルはOSレベルでの非許容を想定して確認していない