# YAML Extended DSL guide

### Basic Structure

```yaml
node_name:
  _state: # Data type definition (optional)
  _store: # Where to save (required at root, inherited by children)
  _load:  # Where to load from (optional)
```

### Core Concept

#### 1. Metadata inheritance

Child nodes inherit parent's _block, and can override:

```yaml
_store:
  client: KVS
  key: "root:${id}"

user:
  _store:
    key: "user:${sso_user_id}"  # Override only key, inherit client: KVS

  tenant_id:
    # Inherits _store from parent (client: KVS, key: user:${sso_user_id})
```

#### 2. プレースホルダー解決

State engineは`${...}`を`State::get()`呼び出しで解決します:

```yaml
tenant:
  _load:
    table: "tenants"
    where: "id=${user.tenant_id}"  # → State::get('user.tenant_id')
```

**プレースホルダーの正規化方法:**

`Manifest::getMeta()`実行時、相対プレースホルダーは自動的に絶対パスに変換されます:

```yaml
# cache.yml
user:
  org_id:
    _load:
      where: "id=${tenant_id}"  # 相対参照
```

Manifestは`${tenant_id}`を`${cache.user.tenant_id}`（絶対パス）に変換します。

Stateがプレースホルダーを見る時点で、既に絶対パスに正規化されています。

#### 3. クライアント種別

**_store用（保存先）:**
```yaml
_store:
  client: InMemory  # プロセスメモリ
  client: KVS       # Redis, Memcached等
```

**_load用（読込元）:**
```yaml
_load:
  client: ENV       # 環境変数
  client: InMemory  # プロセスメモリ
  client: KVS       # Redis, Memcached等
  client: DB        # データベース
  client: State     # 別のStateキーを参照
```

使用する各クライアントのアダプターを実装する必要があります（Required Ports参照）。

### Stateメソッド

**State::get(key)**
- キャッシュ/ストアから値を取得
- `_load`が定義されている場合、miss時に自動ロードをトリガー
- 値またはNoneを返す

**State::set(key, value, ttl)**
- 永続ストアとキャッシュに値を保存
- 自動ロードはトリガーしない
- ttlパラメータはオプション（KVSのみ）

**State::delete(key)**
- 永続ストアとキャッシュの両方からキーを削除
- 削除後、キーはmissを示す

**State::exists(key)**
- 自動ロードをトリガーせずにキーの存在を確認
- 真偽値（true/false）を返す
- 条件分岐用の軽量な存在確認

### 高度な例

```yaml
# example.yml

_store:
  client: # {InMemory, KVS}. 各クライアント用のアダプタークラスを作成
_load:
  client: # {ENV, InMemory, KVS, DB, State}

node_A:
  _state: # オプション、メタデータのみ（型検証は未実装）
    type: {integer, float, string, boolean, list, map}
  _store: # ファイルルートで最低1つ必要。子ノードに継承され、上書き可能。
    client: {InMemory, KVS}  # _storeで有効なのはInMemoryとKVSのみ
  _load:
    client: DB
    connection: ${connection.tenant} # reserved ${} means State::get(). State try 'example.node_A.connection.tenant'(relative path) 1st and if not exists, 'connection.tenant'(absolute path) 2nd.
    table: 'table_A'
    map: # It can load multiple nodes once following YAML coding. Be attention for optimization and unintended loading
      node_1: 'node_1'
      node_2: 'node_2'
  node_1:
    _state:
      ...:
    _store:
      ...:
    _load:
      map:
        node

  node_2: # if no need extra data, this is optional
    _state:
      type: string
  node_3:
    _load:
      key: ${node_1} # It means State::get('example.node_A.node_1') (If not exist, State try 'node_1' 2nd)

node_B:
  node_2:
    _load:
      client: DB
      table: 'table-${example.node_A.node_1}' # It means State::get{'example.node_A.node_1'} (State try 'example.node_B.example.node_A.node_1' 1st)
    _store:
...:
```