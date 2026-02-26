/// state-engine Integration Test Runner
///
/// Runs integration tests against real Db/KVS/Env backends.
/// Requires docker compose services (postgres, redis) to be running.
///
/// Usage: cargo run --bin state-engine-test

mod adapters;

use adapters::{InMemoryAdapter, EnvAdapter, KVSAdapter, DbAdapter};
use state_engine::{State, Load};
use state_engine::ports::required::InMemoryClient;

fn make_state<'a>(
    env_client: &'a EnvAdapter,
    kvs_load: &'a mut KVSAdapter,
    kvs_state: &'a mut KVSAdapter,
    db_client: &'a mut DbAdapter,
    in_memory_load: &'a mut InMemoryAdapter,
    in_memory_state: &'a mut InMemoryAdapter,
) -> State<'a> {
    let load = Load::new()
        .with_env_client(env_client)
        .with_kvs_client(kvs_load)
        .with_db_client(db_client)
        .with_in_memory(in_memory_load);

    State::new("./manifest", load)
        .with_in_memory(in_memory_state)
        .with_kvs_client(kvs_state)
}

fn run_tests() -> (usize, usize) {
    let mut passed = 0;
    let mut failed = 0;

    macro_rules! test {
        ($name:expr, $body:block) => {{
            print!("  {} ... ", $name);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body));
            match result {
                Ok(()) => { println!("ok"); passed += 1; }
                Err(_) => { println!("FAILED"); failed += 1; }
            }
        }};
    }

    // =========================================================================
    // connection: Env load → InMemory store
    // =========================================================================
    println!("\n[connection]");

    test!("get connection.common loads from Env", {
        let env = EnvAdapter::new();
        let mut kl = KVSAdapter::new().unwrap();
        let mut ks = KVSAdapter::new().unwrap();
        let mut db = DbAdapter::new();
        let mut iml = InMemoryAdapter::new();
        let mut ims = InMemoryAdapter::new();
        let mut state = make_state(&env, &mut kl, &mut ks, &mut db, &mut iml, &mut ims);

        let result = state.get("connection.common").unwrap();
        assert!(result.is_some(), "connection.common should be loaded from Env");
        let obj = result.unwrap();
        assert!(obj.get("host").is_some());
        assert!(obj.get("database").is_some());
    });

    test!("exists connection.common after get", {
        let env = EnvAdapter::new();
        let mut kl = KVSAdapter::new().unwrap();
        let mut ks = KVSAdapter::new().unwrap();
        let mut db = DbAdapter::new();
        let mut iml = InMemoryAdapter::new();
        let mut ims = InMemoryAdapter::new();
        let mut state = make_state(&env, &mut kl, &mut ks, &mut db, &mut iml, &mut ims);

        state.get("connection.common").unwrap();
        assert!(state.exists("connection.common").unwrap());
    });

    // =========================================================================
    // session: InMemory set/get
    // =========================================================================
    println!("\n[session]");

    test!("set and get session.sso_user_id via InMemory store", {
        let env = EnvAdapter::new();
        let mut kl = KVSAdapter::new().unwrap();
        let mut ks = KVSAdapter::new().unwrap();
        let mut db = DbAdapter::new();
        let mut iml = InMemoryAdapter::new();
        let mut ims = InMemoryAdapter::new();
        let mut state = make_state(&env, &mut kl, &mut ks, &mut db, &mut iml, &mut ims);

        assert!(state.set("session.sso_user_id", serde_json::json!(42), None).unwrap());
        let got = state.get("session.sso_user_id").unwrap();
        assert_eq!(got, Some(serde_json::json!(42)));
    });

    // =========================================================================
    // cache.user: KVS set/get/delete
    // =========================================================================
    println!("\n[cache.user]");

    test!("set and get cache.user via KVS", {
        let env = EnvAdapter::new();
        let mut kl = KVSAdapter::new().unwrap();
        let mut ks = KVSAdapter::new().unwrap();
        let mut db = DbAdapter::new();
        let mut iml = InMemoryAdapter::new();
        let mut ims = InMemoryAdapter::new();
        // session.sso_user_id needed for KVS key resolution (both load and store side)
        iml.set("request-attributes-user-key", serde_json::json!(1));
        ims.set("request-attributes-user-key", serde_json::json!(1));
        let mut state = make_state(&env, &mut kl, &mut ks, &mut db, &mut iml, &mut ims);

        let user = serde_json::json!({"id": 1, "org_id": 100, "tenant_id": 10});
        assert!(state.set("cache.user", user.clone(), Some(3600)).unwrap());
        let got = state.get("cache.user").unwrap();
        assert_eq!(got, Some(user));
    });

    test!("set and get leaf key cache.user.org_id", {
        let env = EnvAdapter::new();
        let mut kl = KVSAdapter::new().unwrap();
        let mut ks = KVSAdapter::new().unwrap();
        let mut db = DbAdapter::new();
        let mut iml = InMemoryAdapter::new();
        let mut ims = InMemoryAdapter::new();
        iml.set("request-attributes-user-key", serde_json::json!(1));
        ims.set("request-attributes-user-key", serde_json::json!(1));
        let mut state = make_state(&env, &mut kl, &mut ks, &mut db, &mut iml, &mut ims);

        assert!(state.set("cache.user.org_id", serde_json::json!(100), None).unwrap());
        let got = state.get("cache.user.org_id").unwrap();
        assert_eq!(got, Some(serde_json::json!(100)));
    });

    test!("set and get leaf key cache.user.id", {
        let env = EnvAdapter::new();
        let mut kl = KVSAdapter::new().unwrap();
        let mut ks = KVSAdapter::new().unwrap();
        let mut db = DbAdapter::new();
        let mut iml = InMemoryAdapter::new();
        let mut ims = InMemoryAdapter::new();
        iml.set("request-attributes-user-key", serde_json::json!(1));
        ims.set("request-attributes-user-key", serde_json::json!(1));
        let mut state = make_state(&env, &mut kl, &mut ks, &mut db, &mut iml, &mut ims);

        assert!(state.set("cache.user.id", serde_json::json!(1), None).unwrap());
        let got = state.get("cache.user.id").unwrap();
        assert_eq!(got, Some(serde_json::json!(1)));
    });

    test!("cache.user.tenant_id resolved via State client from org_id", {
        // tenant_id._load.client = State, key = ${org_id}
        // org_id をsetすると tenant_id は org_id の値から解決される
        let env = EnvAdapter::new();
        let mut kl = KVSAdapter::new().unwrap();
        let mut ks = KVSAdapter::new().unwrap();
        let mut db = DbAdapter::new();
        let mut iml = InMemoryAdapter::new();
        let mut ims = InMemoryAdapter::new();
        iml.set("request-attributes-user-key", serde_json::json!(1));
        ims.set("request-attributes-user-key", serde_json::json!(1));
        let mut state = make_state(&env, &mut kl, &mut ks, &mut db, &mut iml, &mut ims);

        // org_id をsetしておく（tenant_id解決の元になる）
        let ttl = Some(14400);
        assert!(state.set("cache.user.org_id", serde_json::json!(100), ttl).unwrap());

        // tenant_id は State client経由で org_id の値(100)を返す
        let got = state.get("cache.user.tenant_id").unwrap();
        assert_eq!(got, Some(serde_json::json!(100)));
    });

    test!("delete cache.user from KVS", {
        let env = EnvAdapter::new();
        let mut kl = KVSAdapter::new().unwrap();
        let mut ks = KVSAdapter::new().unwrap();
        let mut db = DbAdapter::new();
        let mut iml = InMemoryAdapter::new();
        let mut ims = InMemoryAdapter::new();
        iml.set("request-attributes-user-key", serde_json::json!(1));
        ims.set("request-attributes-user-key", serde_json::json!(1));
        let mut state = make_state(&env, &mut kl, &mut ks, &mut db, &mut iml, &mut ims);

        state.set("cache.user", serde_json::json!({"id": 1}), None).unwrap();
        assert!(state.delete("cache.user").unwrap());
        assert!(!state.exists("cache.user").unwrap());
    });

    // =========================================================================
    // cache.user: DB load
    // =========================================================================
    println!("\n[cache.user DB load]");

    test!("get cache.user loads from DB via _load", {
        // 解決順: sso_user_id → org_id → tenant_id(State) → connection.tenant → cache.user(DB)
        let env = EnvAdapter::new();
        let mut kl = KVSAdapter::new().unwrap();
        let mut ks = KVSAdapter::new().unwrap();
        let mut db = DbAdapter::new();
        let mut iml = InMemoryAdapter::new();
        let mut ims = InMemoryAdapter::new();

        let db_host     = std::env::var("DB_HOST").unwrap_or("localhost".into());
        let db_port     = std::env::var("DB_PORT").unwrap_or("5432".into()).parse::<u64>().unwrap_or(5432);
        let db_database = std::env::var("DB_DATABASE").unwrap_or("state_engine_dev".into());
        let db_username = std::env::var("DB_USERNAME").unwrap_or("state_user".into());
        let db_password = std::env::var("DB_PASSWORD").unwrap_or("state_pass".into());

        // 1. sso_user_id=1 をセット（KVSキー解決に必要）
        iml.set("request-attributes-user-key", serde_json::json!(1));
        ims.set("request-attributes-user-key", serde_json::json!(1));

        let mut state = make_state(&env, &mut kl, &mut ks, &mut db, &mut iml, &mut ims);

        // 2. org_id をセット（tenant_id の State client解決元）
        state.set("cache.user.org_id", serde_json::json!(100), Some(14400)).unwrap();

        // 3. connection.tenant をセット（tenant_id=1 に対応）
        //    tenant_id は org_id(100) から State client経由で解決されるが、
        //    connection.tenant の _store.key = "connection.tenant${cache.user.tenant_id}" なので
        //    tenant_id が確定した上で connection.tenant をセットする必要がある
        state.set("cache.user.tenant_id", serde_json::json!(1), Some(14400)).unwrap();
        let tenant_conn = serde_json::json!({
            "tag": "tenant",
            "id": 1,
            "host": db_host,
            "port": db_port,
            "database": db_database,
            "username": db_username,
            "password": db_password,
        });
        state.set("connection.tenant", tenant_conn, None).unwrap();

        // 4. cache.user を DB から取得（sso_user_id=1 → users テーブル）
        // leaf key set で KVS に断片が入っている可能性があるので先に削除
        state.delete("cache.user").ok();
        let result = state.get("cache.user").unwrap();
        assert!(result.is_some(), "cache.user should be loaded from DB");
        let obj = result.unwrap();
        assert!(obj.get("id").is_some(), "id should be present");
        assert!(obj.get("org_id").is_some(), "org_id should be present");
    });

    // =========================================================================
    // placeholder resolution error cases
    // =========================================================================
    println!("\n[placeholder]");

    test!("set cache.user without session.sso_user_id returns Err", {
        // cache.user._store.key = "user:${session.sso_user_id}"
        // session.sso_user_id が未セットなので placeholder が解決できず Err になるはず
        let env = EnvAdapter::new();
        let mut kl = KVSAdapter::new().unwrap();
        let mut ks = KVSAdapter::new().unwrap();
        let mut db = DbAdapter::new();
        let mut iml = InMemoryAdapter::new();
        let mut ims = InMemoryAdapter::new();
        let mut state = make_state(&env, &mut kl, &mut ks, &mut db, &mut iml, &mut ims);

        let result = state.set("cache.user", serde_json::json!({"id": 1}), None);
        assert!(result.is_err(), "should fail when placeholder cannot be resolved");
    });

    test!("get cache.user without session.sso_user_id returns Err", {
        // store read と load の両方で ${session.sso_user_id} が未解決になる
        let env = EnvAdapter::new();
        let mut kl = KVSAdapter::new().unwrap();
        let mut ks = KVSAdapter::new().unwrap();
        let mut db = DbAdapter::new();
        let mut iml = InMemoryAdapter::new();
        let mut ims = InMemoryAdapter::new();
        let mut state = make_state(&env, &mut kl, &mut ks, &mut db, &mut iml, &mut ims);

        let result = state.get("cache.user");
        assert!(result.is_err(), "should fail when placeholder cannot be resolved");
    });

    test!("get nonexistent key returns KeyNotFound", {
        let env = EnvAdapter::new();
        let mut kl = KVSAdapter::new().unwrap();
        let mut ks = KVSAdapter::new().unwrap();
        let mut db = DbAdapter::new();
        let mut iml = InMemoryAdapter::new();
        let mut ims = InMemoryAdapter::new();
        let mut state = make_state(&env, &mut kl, &mut ks, &mut db, &mut iml, &mut ims);

        let result = state.get("session.nonexistent");
        assert!(matches!(result, Err(state_engine::StateError::KeyNotFound(_))));
    });

    (passed, failed)
}

fn main() {
    env_logger::init();
    println!("=== state-engine Integration Tests ===");
    let (passed, failed) = run_tests();
    println!("\n{} passed, {} failed", passed, failed);
    if failed > 0 {
        std::process::exit(1);
    }
}
