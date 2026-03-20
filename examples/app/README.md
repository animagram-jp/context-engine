# Sample Application

## how to run

```bash
cd examples/app
chmod +x run.sh
./run.sh
```

## tree

```
app/
  db/
    000_create_databases.sh
    001_init.sql
  src/
    adapters.rs
    main.rs
   .env
   Cargo.toml
   Dockerfile
   docker-compose.yml
   README.md
   run.sh
```

## expected output

```
=== state-engine Integration Tests ===

[connection]
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('connection.common')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('connection')
  get connection.common loads from Env ... ok
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('connection.common')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('6')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('connection.common')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('connection')
  exists connection.common after get ... ok

[session]
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('connection.common')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('6')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::exists('connection.common')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('connection')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::set('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
  set and get session.sso_user_id via InMemory store ... ok

[cache.user]
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('5')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('2')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::set('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('55')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('23')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('2')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::set('cache.user.org_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
  set and get cache.user via KVS ... ok
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.org_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('55')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('23')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('2')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('cache.user.org_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::set('cache.user.id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
  set and get leaf key cache.user.org_id ... ok
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('55')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('23')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('2')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('cache.user.id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::set('cache.user.org_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
  set and get leaf key cache.user.id ... ok
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.org_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('55')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('23')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('2')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('cache.user.tenant_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.tenant_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('2')
  cache.user.tenant_id resolved via State client from org_id ... ok
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::set('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('55')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('23')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('2')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::delete('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('2')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::exists('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('2')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::set('cache.user.org_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
  delete cache.user from KVS ... ok

[cache.user DB load]
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.org_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('55')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('23')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('2')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::set('cache.user.tenant_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.tenant_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('2')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::set('connection.tenant')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('connection')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('connection.tenant')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('93')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('39')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('cache.user.tenant_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('cache.user.tenant_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::delete('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('2')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('2')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('7')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('connection.tenant')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('connection')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('4')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('5')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG tokio_postgres::prepare] preparing query s0: SELECT id, sso_org_id FROM users WHERE sso_user_id=1
[2026-03-20T07:13:27Z DEBUG tokio_postgres::query] executing statement s0 with parameters: []
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('2')
  get cache.user loads from DB via _load ... ok

[cache.tenant.health HTTP load]
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::set('cache.user.tenant_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user.tenant_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('55')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('23')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('2')
  get cache.tenant.health loads from HTTP after cache.user is set ... ok

[placeholder]
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('cache.tenant.health')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('cache.tenant.health')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('43')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('44')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('19')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('cache.user.tenant_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('cache.user.tenant_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/got('Some')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('43')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::set('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('55')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('23')
  set cache.user without session.sso_user_id returns Err ... ok
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('58')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('24')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('cache')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('cache.user')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('3')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('1')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve/get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::get_meta('session.sso_user_id')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('55')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('23')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::build_config('58')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::resolve_value_to_string('24')
[2026-03-20T07:13:27Z DEBUG state_engine::state] State::get('session.nonexistent')
[2026-03-20T07:13:27Z DEBUG state_engine::manifest] Manifest::load('session')
  get cache.user without session.sso_user_id returns Err ... ok
  get nonexistent key returns KeyNotFound ... ok

13 passed, 0 failed
```
