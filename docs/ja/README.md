# state-engine

開発者が記述するYAML拡張DSL(ドメイン特化言語)を設計図に、高度要件のステートデータを自動管理するライブラリです。

このライブラリを導入し、段階的に適切なYAMLとAdapterクラスを整備すれば、例えばマルチテナントDBアプリに中間表が不要になります。
state-engineは、[## background](#background)記載の新たなwebシステム構成を発想元として開発されています。

## background

**webシステムの構成再定義**

- computer: (ネットワーク通信機能を要する)コンピューター。
- server: 人間(ユーザー)に奉仕するcomputer
- orchestrator: webシステムを構成するcomputerのうち、システム内部の維持を管理するもの(optional)
- database: 明示的に削除されるまでデータを維持し、terminalやconductorにCRUDを受け付けるserver
- terminal: 人間が直接触るインターフェースを提供するserver. 「端末」
- conductor: databaseとterminalに対してそれぞれ通信し、二者の同期状態を維持するserver(optional)

```yaml
# terms relationship
computer:
  orchestrator:
  server:
    database:
    terminal:
    conductor:
```

## Architecture

[Architecture.md](./Architecture.md) を参照のこと

## License

MIT
