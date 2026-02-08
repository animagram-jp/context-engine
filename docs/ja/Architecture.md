# Architecture

## index

```yaml
Ports:
  Provided: {Manifest, State}
  Required: {InMemoryClient, DBClient, KVSClient, ENVClient}

Common:
  DotArrayAccessor:
  PlaceholderResover:
  LogFormat:

Manifest:
State:
Load:
```

## Ports

ライブラリ インターフェース

1. Provided Ports

ライブラリ提供のトレイト

  1. **Manifest** - YAMLファイルの読み込みと集計モジュール。
  2. State


2. Required Ports

