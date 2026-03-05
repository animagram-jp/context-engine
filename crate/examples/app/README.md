# Sample Application

## how to run

```bash
cd examples/app
docker compose up --build
```

## tree

```
app/
  db/
    001_init.sql
  src/
    main.rs
    adapters.rs
    test_runner.rs
   .env
   Cargo.toml
   Dockerfile
   README.md
```

## expected output

```
example-app  | === state-engine Integration Tests ===
example-app  | 
example-app  | [connection]
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('connection.common')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('connection')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('connection.common')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('6')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  |   get connection.common loads from Env ... ok
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('connection.common')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('connection')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('connection.common')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('6')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::exists('connection.common')
example-app  |   exists connection.common after get ... ok
example-app  | 
example-app  | [session]
example-app  |   set and get session.sso_user_id via InMemory store ... ok
example-app  | 
example-app  | [cache.user]
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('connection')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::set('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('5')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('2')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::set('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('46')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('20')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/got('Some')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('2')
example-app  |   set and get cache.user via KVS ... ok
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::set('cache.user.org_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.org_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('46')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('20')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/got('Some')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('2')
example-app  |   set and get leaf key cache.user.org_id ... ok
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('cache.user.org_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::set('cache.user.id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('46')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('20')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/got('Some')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('2')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('cache.user.id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::set('cache.user.org_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.org_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('46')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('20')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/got('Some')


example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('2')
example-app exited with code 0
example-app  |   set and get leaf key cache.user.id ... ok
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('cache.user.tenant_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.tenant_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/got('Some')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('2')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::set('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  |   cache.user.tenant_id resolved via State client from org_id ... ok
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('46')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('20')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/got('Some')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('2')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::delete('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/got('Some')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('2')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::exists('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/got('Some')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('2')
example-app  |   delete cache.user from KVS ... ok
example-app  | 
example-app  | [cache.user DB load]
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::set('cache.user.org_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.org_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('46')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('20')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/got('Some')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('2')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::set('cache.user.tenant_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.tenant_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/got('Some')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('2')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::set('connection.tenant')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('connection')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('connection.tenant')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('84')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('36')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('cache.user.tenant_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('cache.user.tenant_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/got('Some')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::delete('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/got('Some')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('2')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/got('Some')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('2')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('7')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('connection.tenant')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('connection')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('4')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('5')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/got('Some')
example-app  | [2026-03-04T00:16:04Z DEBUG tokio_postgres::prepare] preparing query s0: SELECT id, sso_org_id FROM users WHERE sso_user_id=1
example-app  | [2026-03-04T00:16:04Z DEBUG tokio_postgres::query] executing statement s0 with parameters: []
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/got('Some')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('2')
example-app  |   get cache.user loads from DB via _load ... ok
example-app  | 
example-app  | [placeholder]
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::set('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('46')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('20')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('49')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('21')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('cache')
example-app  |   set cache.user without session.sso_user_id returns Err ... ok
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('3')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('1')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('46')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('20')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::build_config('49')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::resolve_value_to_string('21')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::state] State::get('session.nonexistent')
example-app  | [2026-03-04T00:16:04Z DEBUG state_engine::manifest] Manifest::load('session')
example-app  |   get cache.user without session.sso_user_id returns Err ... ok
example-app  |   get nonexistent key returns KeyNotFound ... ok
example-app  | 
example-app  | 12 passed, 0 failed
```
