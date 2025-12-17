# declare-engine

宣言性マルチストアステート管理ライブラリ

## list

1. DotArrayAccessor
2. Manifest
3. DBConnection
4. KVStore

## tree

```
declare-engine/
  README.md
  composer.json
  src/
    Ports/
      Required/
        ProcessMemoryClient
        KVSClient
        DBClient
      Provided/
        Manifest
        DBConnection
        KVStore
        UserKey
        Auth
    Common/
      DotArrayAccessor
    Manifest/
      Main
    DBConnection/
      Main
      ConfigManager
    KVStore/
      Main
      Loader
      Scope
    UserKey/
      Main
    Auth/
      Main

  tests/
    Feature/
    Unit/

  samples/
    manifest/
    Adapters/
```

## requirements

- php 8.4+
- psr/log 3.0+ (recommended)

