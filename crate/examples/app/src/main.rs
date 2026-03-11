mod adapters;

use adapters::{InMemoryAdapter, EnvAdapter, KVSAdapter, DbAdapter};
use state_engine::{State, InMemoryClient};

fn make_state<'a>(
    env: &'a EnvAdapter,
    kvs: &'a KVSAdapter,
    db: &'a DbAdapter,
    in_memory: &'a InMemoryAdapter,
) -> State<'a> {
    State::new("./manifest")
        .with_env(env)
        .with_kvs(kvs)
        .with_db(db)
        .with_in_memory(in_memory)
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
        let kvs = KVSAdapter::new().unwrap();
        let db = DbAdapter::new();
        let im = InMemoryAdapter::new();
        let mut state = make_state(&env, &kvs, &db, &im);

        let result = state.get("connection.common").unwrap();
        assert!(result.is_some(), "connection.common should be loaded from Env");
        let obj = result.unwrap();
        assert!(obj.get("host").is_some());
        assert!(obj.get("database").is_some());
    });

    test!("exists connection.common after get", {
        let env = EnvAdapter::new();
        let kvs = KVSAdapter::new().unwrap();
        let db = DbAdapter::new();
        let im = InMemoryAdapter::new();
        let mut state = make_state(&env, &kvs, &db, &im);

        state.get("connection.common").unwrap();
        assert!(state.exists("connection.common").unwrap());
    });

    // =========================================================================
    // session: InMemory set/get
    // =========================================================================
    println!("\n[session]");

    test!("set and get session.sso_user_id via InMemory store", {
        let env = EnvAdapter::new();
        let kvs = KVSAdapter::new().unwrap();
        let db = DbAdapter::new();
        let im = InMemoryAdapter::new();
        let mut state = make_state(&env, &kvs, &db, &im);

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
        let kvs = KVSAdapter::new().unwrap();
        let db = DbAdapter::new();
        let im = InMemoryAdapter::new();
        im.set("request-attributes-user-key", serde_json::json!(1));
        let mut state = make_state(&env, &kvs, &db, &im);

        let user = serde_json::json!({"id": 1, "org_id": 100, "tenant_id": 10});
        assert!(state.set("cache.user", user.clone(), Some(3600)).unwrap());
        let got = state.get("cache.user").unwrap();
        assert_eq!(got, Some(user));
    });

    test!("set and get leaf key cache.user.org_id", {
        let env = EnvAdapter::new();
        let kvs = KVSAdapter::new().unwrap();
        let db = DbAdapter::new();
        let im = InMemoryAdapter::new();
        im.set("request-attributes-user-key", serde_json::json!(1));
        let mut state = make_state(&env, &kvs, &db, &im);

        assert!(state.set("cache.user.org_id", serde_json::json!(100), None).unwrap());
        let got = state.get("cache.user.org_id").unwrap();
        assert_eq!(got, Some(serde_json::json!(100)));
    });

    test!("set and get leaf key cache.user.id", {
        let env = EnvAdapter::new();
        let kvs = KVSAdapter::new().unwrap();
        let db = DbAdapter::new();
        let im = InMemoryAdapter::new();
        im.set("request-attributes-user-key", serde_json::json!(1));
        let mut state = make_state(&env, &kvs, &db, &im);

        assert!(state.set("cache.user.id", serde_json::json!(1), None).unwrap());
        let got = state.get("cache.user.id").unwrap();
        assert_eq!(got, Some(serde_json::json!(1)));
    });

    test!("cache.user.tenant_id resolved via State client from org_id", {
        let env = EnvAdapter::new();
        let kvs = KVSAdapter::new().unwrap();
        let db = DbAdapter::new();
        let im = InMemoryAdapter::new();
        im.set("request-attributes-user-key", serde_json::json!(1));
        let mut state = make_state(&env, &kvs, &db, &im);

        assert!(state.set("cache.user.org_id", serde_json::json!(100), Some(14400)).unwrap());
        let got = state.get("cache.user.tenant_id").unwrap();
        assert_eq!(got, Some(serde_json::json!(100)));
    });

    test!("delete cache.user from KVS", {
        let env = EnvAdapter::new();
        let kvs = KVSAdapter::new().unwrap();
        let db = DbAdapter::new();
        let im = InMemoryAdapter::new();
        im.set("request-attributes-user-key", serde_json::json!(1));
        let mut state = make_state(&env, &kvs, &db, &im);

        state.set("cache.user", serde_json::json!({"id": 1}), None).unwrap();
        assert!(state.delete("cache.user").unwrap());
        assert!(!state.exists("cache.user").unwrap());
    });

    // =========================================================================
    // cache.user: DB load
    // =========================================================================
    println!("\n[cache.user DB load]");

    test!("get cache.user loads from DB via _load", {
        let env = EnvAdapter::new();
        let kvs = KVSAdapter::new().unwrap();
        let db = DbAdapter::new();
        let im = InMemoryAdapter::new();

        let db_host     = std::env::var("DB_HOST").unwrap_or("localhost".into());
        let db_port     = std::env::var("DB_PORT").unwrap_or("5432".into()).parse::<u64>().unwrap_or(5432);
        let db_database = std::env::var("DB_DATABASE").unwrap_or("state_engine_dev".into());
        let db_username = std::env::var("DB_USERNAME").unwrap_or("state_user".into());
        let db_password = std::env::var("DB_PASSWORD").unwrap_or("state_pass".into());

        im.set("request-attributes-user-key", serde_json::json!(1));
        let mut state = make_state(&env, &kvs, &db, &im);

        state.set("cache.user.org_id", serde_json::json!(100), Some(14400)).unwrap();
        state.set("cache.user.tenant_id", serde_json::json!(1), Some(14400)).unwrap();

        let tenant_conn = serde_json::json!({
            "tag": "tenant", "id": 1,
            "host": db_host, "port": db_port,
            "database": db_database, "username": db_username, "password": db_password,
        });
        state.set("connection.tenant", tenant_conn, None).unwrap();
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
        let env = EnvAdapter::new();
        let kvs = KVSAdapter::new().unwrap();
        let db = DbAdapter::new();
        let im = InMemoryAdapter::new();
        let mut state = make_state(&env, &kvs, &db, &im);

        let result = state.set("cache.user", serde_json::json!({"id": 1}), None);
        assert!(result.is_err(), "should fail when placeholder cannot be resolved");
    });

    test!("get cache.user without session.sso_user_id returns Err", {
        let env = EnvAdapter::new();
        let kvs = KVSAdapter::new().unwrap();
        let db = DbAdapter::new();
        let im = InMemoryAdapter::new();
        let mut state = make_state(&env, &kvs, &db, &im);

        let result = state.get("cache.user");
        assert!(result.is_err(), "should fail when placeholder cannot be resolved");
    });

    test!("get nonexistent key returns KeyNotFound", {
        let env = EnvAdapter::new();
        let kvs = KVSAdapter::new().unwrap();
        let db = DbAdapter::new();
        let im = InMemoryAdapter::new();
        let mut state = make_state(&env, &kvs, &db, &im);

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
