# Architecture

## Ports(Library Interface Class)

### Provided

1. Manifest - read-only class from DSL files. It detect _ prefix key (meta block) and ignore at get(), collect at getMeta()
2. State - get/set/delete class to state objects. It has Load function only at get(), Self-Reference and Serialize at get() and set(), Validate at set(), ...

## Method lists

### Manifest::class

#### Manifest::get('filename.node')

manifest/*.ymlの_prefixブロック(以下、メタデータブロック)を無視したnode構造の読み出し(指定nodeが最下位なら値(記述が無ければnull), それ以外は配下のnode全てを表現したcollectionデータを返却)。指定nodeがyml上で存在しなければ(miss)、nullを返却する。'filename.node'には'filename.'の意味で'filename'を指定することが出来る。

#### Manifest::getMeta('filename.node')

manifest/*.ymlの指定nodeのメタデータブロックを返却する。この時、該当yml fileのroot nodeから指定nodeまでのメタデータブロックを順に全て読み出し、より子の同キーで上書きしたlistを返却する。指定nodeがyml上で存在しなければ(miss)、nullを返却する。'filename.node'には'filename.*'の意味で'filename'を指定することが出来、この場合はnodeに属さない最上位のメタデータブロックのみがlistに格納されて返却される。メタデータブロック以下のnodeを指定することもできるが、想定はされていない。

### State::class

#### State::get('filename.node')

manifest/*.ymlの指定nodeが表すstateを参照し、value、もしくは配下のcollectionsを返却する。指定nodeが表すstate keyがmissした場合、placeholderをload::handle()にて自動取得を試行後、再度valueもしくはcollectionsを返却する。load::handle()のerror時はnullを返却する※

※load::handle()のerrorについては、専用のExceptionクラスでdebug情報を返却する

#### State:set('filename.node', $value, ?$ttl)

指定nodeの表すstateに対して値をsetする。操作先storeがKVSであれば、ttlのオプションを設定でき、操作先KVS recordのttlが再設定される。無指定では、ttlはymlにデフォルト値が設定されていれば再設定し、無ければ操作時点の値を維持する※

※ ttl挙動は議論の余地あり

#### State:delete('filename.node')

指定nodeが表すstateの該当部を{key:value}レコードごと削除する。削除後、同nodeはmiss hitを示す。

#### Load::handle()

state::get('filename.node')がmiss valueした時、manifest::getMeta('filename.node')の_storeと_loadの記述内容に従ってデータを取得する。

※ 無限再帰によるprocess errorについて、事前のyml静的解析の他、呼出回数のinstance var管理など議論の余地あり