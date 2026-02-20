# state-engine

プロセスのための宣言的なステートデータ管理システムです。
プロセス上でステートデータを構造化し、開発者が定義するストアAPIを使って同期可能な状態を保ちます。
YAML DSLで記述された定義に従って振る舞います。

- YAML manifestによる複雑なステートライフサイクルの自動化
- 中間表を必要としないマルチテナントDbアプリケーション
- [## background](#background)記載の再定義されたwebアーキテクチャに基づいて構築

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
