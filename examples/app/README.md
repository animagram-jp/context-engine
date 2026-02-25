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
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('connection.common')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('connection')
state-engine-test      | === state-engine Integration Tests ===
state-engine-test      | 
state-engine-test      | [connection]
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('connection.common')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('6')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('connection.common')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('connection')
state-engine-test      |   get connection.common loads from Env ... ok
state-engine-test      |   exists connection.common after get ... ok
state-engine-test      | 
state-engine-test      | [session]
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('connection.common')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('6')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::exists('connection.common')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('connection')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::set('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('session')
state-engine-test      |   set and get session.sso_user_id via InMemory store ... ok
state-engine-test      | 
state-engine-test      | [cache.user]
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('5')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('2')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('session')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::set('cache.user')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('cache')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('session')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('46')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('20')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/got('Some')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('2')
state-engine-test      |   set and get cache.user via KVS ... ok
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('cache.user')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('cache')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::set('cache.user.org_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('cache')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.org_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('session')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('46')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('20')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/got('Some')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('2')
state-engine-test      |   set and get leaf key cache.user.org_id ... ok
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('cache.user.org_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('cache')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::set('cache.user.id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('cache')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('session')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('46')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('20')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/got('Some')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('2')


state-engine-test      |   set and get leaf key cache.user.id ... ok
state-engine-test exited with code 0
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('cache.user.id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('cache')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::set('cache.user.org_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('cache')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.org_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('session')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('46')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('20')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/got('Some')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('2')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('cache.user.tenant_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('cache')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.tenant_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('session')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/got('Some')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('2')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::set('cache.user')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('cache')
state-engine-test      |   cache.user.tenant_id resolved via State client from org_id ... ok
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('session')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('46')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('20')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/got('Some')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('2')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::delete('cache.user')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('cache')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('session')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/got('Some')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('2')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::exists('cache.user')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('cache')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('session')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/got('Some')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('2')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::set('cache.user.org_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('cache')
state-engine-test      |   delete cache.user from KVS ... ok
state-engine-test      | 
state-engine-test      | [cache.user DB load]
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.org_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('session')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('46')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('20')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/got('Some')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('2')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::set('cache.user.tenant_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('cache')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.tenant_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('session')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/got('Some')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('2')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::set('connection.tenant')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('connection')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('connection.tenant')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('84')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('36')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/get('cache.user.tenant_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('cache.user.tenant_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('cache')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/got('Some')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::delete('cache.user')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('cache')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('session')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/got('Some')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('2')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('cache.user')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('cache')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('session')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/got('Some')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('2')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('7')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('connection.tenant')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('connection')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('4')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('5')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('session')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/got('Some')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG tokio_postgres::prepare] preparing query s0: SELECT id, sso_org_id FROM users WHERE sso_user_id=1
state-engine-test      | [2026-02-25T22:30:35Z DEBUG tokio_postgres::query] executing statement s0 with parameters: []
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::build_config('3')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('1')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::get('session.sso_user_id')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::manifest] Manifest::load('session')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve/got('Some')
state-engine-test      | [2026-02-25T22:30:35Z DEBUG state_engine::state] State::resolve_value_to_string('2')
state-engine-test      |   get cache.user loads from DB via _load ... ok
state-engine-test      | 
state-engine-test      | 9 passed, 0 failed
```
