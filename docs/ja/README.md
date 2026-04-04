# state-engine

webシステムのランタイムが1回の処理の中で使用するデータのラベルは、セッションコンテクストによる変動を、コード外で処理するべきです(例: users[session[user-id]]では無く、system_context["session.user"]で呼び出せるべき)。state-engineは、アプリ開発者がYAMLファイルにDSLとして定義したデータの取得方法を、ラベルごとに処理します。これにより、例えばsystem_context["session.user.preference"]のサーバー/クライアント差異が、context[session.user.tenant]のマルチテナント差異が、YAML内のデータ取得方法によって、適切に解決されます。このOSSは、[## background](#background)記載の、再構成されたwebシステムアーキテクチャの基盤技術に位置付けられています。

## background

**webシステムの構成再定義**

人々の営みの動作の一部を、ネットワーク機能を持ったコンピューターのデータ処理で代替えすることで、その間の検証可能性の保証と、物理的制約の緩和などの恩恵を受けることができる。これを実現する、ハードウェアを通して電気信号として入力を受け取り、処理後、所定のハードウェア群に出力する仕組みのことを、webシステムと呼ぶ。webシステムの実現には、第一に、システムに必要な概念体系を、人間言語とコンピューターのビット列それぞれで定義することが必要である。

```yaml
# computers structure of web system
computer:       "(ネットワーク通信機能を要する)コンピューター"
  server:       "人間(ユーザー・開発者)に処理能力を提供する"
    fixture:    "継続的な待機により、ネットワーク機能を提供する"
    terminal:   "人間とのインターフェースを提供する。端末。"
  orchestrator: "サーバー群の維持を管理する(optional)"
```

## Architecture

[Architecture.md](./Architecture.md) を参照のこと

## License

MIT
