mod adapters;

use adapters::{InMemoryAdapter, EnvAdapter, KVSAdapter, DbAdapter, HttpAdapter};
use state_engine::{State, Value, ports::required::InMemoryClient};
use std::sync::Arc;

fn make_state(
    env: Arc<EnvAdapter>,
    kvs: Arc<KVSAdapter>,
    db: Arc<DbAdapter>,
    in_memory: Arc<InMemoryAdapter>,
    http: Arc<HttpAdapter>,
) -> State {
    State::new("./manifest")
        .with_env(env)
        .with_kvs(kvs)
        .with_db(db)
        .with_in_memory(in_memory)
        .with_http(http)
}

fn scalar(s: &str) -> Value {
    Value::Scalar(s.as_bytes().to_vec())
}

fn mapping_get<'a>(v: &'a Value, key: &[u8]) -> Option<&'a Value> {
    match v {
        Value::Mapping(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
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
        let im = Arc::new(InMemoryAdapter::new());
        let mut state = make_state(
            Arc::new(EnvAdapter::new()),
            Arc::new(KVSAdapter::new().unwrap()),
            Arc::new(DbAdapter::new()),
            im,
            Arc::new(HttpAdapter),
        );

        let result = state.get("connection.common").unwrap();
        assert!(result.is_some(), "connection.common should be loaded from Env");
        let obj = result.unwrap();
        assert!(mapping_get(&obj, b"host").is_some());
        assert!(mapping_get(&obj, b"database").is_some());
    });

    test!("exists connection.common after get", {
        let im = Arc::new(InMemoryAdapter::new());
        let mut state = make_state(
            Arc::new(EnvAdapter::new()),
            Arc::new(KVSAdapter::new().unwrap()),
            Arc::new(DbAdapter::new()),
            im,
            Arc::new(HttpAdapter),
        );

        state.get("connection.common").unwrap();
        assert!(state.exists("connection.common").unwrap());
    });

    // =========================================================================
    // session: InMemory set/get
    // =========================================================================
    println!("\n[session]");

    test!("set and get session.sso_user_id via InMemory store", {
        let im = Arc::new(InMemoryAdapter::new());
        let mut state = make_state(
            Arc::new(EnvAdapter::new()),
            Arc::new(KVSAdapter::new().unwrap()),
            Arc::new(DbAdapter::new()),
            im,
            Arc::new(HttpAdapter),
        );

        assert!(state.set("session.sso_user_id", scalar("42"), None).unwrap());
        let got = state.get("session.sso_user_id").unwrap();
        assert_eq!(got, Some(scalar("42")));
    });

    // =========================================================================
    // cache.user: KVS set/get/delete
    // =========================================================================
    println!("\n[cache.user]");

    test!("set and get cache.user via KVS", {
        let im = Arc::new(InMemoryAdapter::new());
        im.set("request-attributes-user-key", scalar("1"));
        let mut state = make_state(
            Arc::new(EnvAdapter::new()),
            Arc::new(KVSAdapter::new().unwrap()),
            Arc::new(DbAdapter::new()),
            im,
            Arc::new(HttpAdapter),
        );

        let user = Value::Mapping(vec![
            (b"id".to_vec(),        scalar("1")),
            (b"org_id".to_vec(),    scalar("100")),
            (b"tenant_id".to_vec(), scalar("10")),
        ]);
        assert!(state.set("cache.user", user.clone(), Some(3600)).unwrap());
        let got = state.get("cache.user").unwrap();
        assert_eq!(got, Some(user));
    });

    test!("set and get leaf key cache.user.org_id", {
        let im = Arc::new(InMemoryAdapter::new());
        im.set("request-attributes-user-key", scalar("1"));
        let kvs = Arc::new(KVSAdapter::new().unwrap());
        let mut state = make_state(
            Arc::new(EnvAdapter::new()),
            Arc::clone(&kvs),
            Arc::new(DbAdapter::new()),
            im,
            Arc::new(HttpAdapter),
        );

        assert!(state.set("cache.user.org_id", scalar("100"), None).unwrap());
        let got = state.get("cache.user.org_id").unwrap();
        assert_eq!(got, Some(scalar("100")));

        // verify Redis directly: KVS key is "user:1" (session.sso_user_id=1)
        // expected: encoded Mapping with org_id=100, not raw b"100"
        let raw = kvs.raw_get("user:1");
        assert!(raw.is_some(), "user:1 should exist in Redis");
        let decoded = state_engine::codec_value::decode(raw.as_deref().unwrap());
        assert!(decoded.is_some(), "Redis value should decode as Value");
        let decoded = decoded.unwrap();
        assert_eq!(
            mapping_get(&decoded, b"org_id"),
            Some(&scalar("100")),
            "org_id in Redis mapping should be 100"
        );
    });

    test!("set and get leaf key cache.user.id", {
        let im = Arc::new(InMemoryAdapter::new());
        im.set("request-attributes-user-key", scalar("1"));
        let mut state = make_state(
            Arc::new(EnvAdapter::new()),
            Arc::new(KVSAdapter::new().unwrap()),
            Arc::new(DbAdapter::new()),
            im,
            Arc::new(HttpAdapter),
        );

        assert!(state.set("cache.user.id", scalar("1"), None).unwrap());
        let got = state.get("cache.user.id").unwrap();
        assert_eq!(got, Some(scalar("1")));
    });

    test!("cache.user.tenant_id resolved via State client from org_id", {
        let im = Arc::new(InMemoryAdapter::new());
        im.set("request-attributes-user-key", scalar("1"));
        let mut state = make_state(
            Arc::new(EnvAdapter::new()),
            Arc::new(KVSAdapter::new().unwrap()),
            Arc::new(DbAdapter::new()),
            im,
            Arc::new(HttpAdapter),
        );

        assert!(state.set("cache.user.org_id", scalar("100"), Some(14400)).unwrap());
        let got = state.get("cache.user.tenant_id").unwrap();
        assert_eq!(got, Some(scalar("100")));
    });

    test!("delete cache.user from KVS", {
        let im = Arc::new(InMemoryAdapter::new());
        im.set("request-attributes-user-key", scalar("1"));
        let mut state = make_state(
            Arc::new(EnvAdapter::new()),
            Arc::new(KVSAdapter::new().unwrap()),
            Arc::new(DbAdapter::new()),
            im,
            Arc::new(HttpAdapter),
        );

        state.set("cache.user", Value::Mapping(vec![(b"id".to_vec(), scalar("1"))]), None).unwrap();
        assert!(state.delete("cache.user").unwrap());
        assert!(!state.exists("cache.user").unwrap());
    });

    // =========================================================================
    // cache.user: DB load
    // =========================================================================
    println!("\n[cache.user DB load]");

    test!("get cache.user loads from DB via _load", {
        let db_host     = std::env::var("DB_HOST").unwrap_or("localhost".into());
        let db_port     = std::env::var("DB_PORT").unwrap_or("5432".into());
        let db_database = std::env::var("DB_DATABASE").unwrap_or("state_engine_dev".into());
        let db_username = std::env::var("DB_USERNAME").unwrap_or("state_user".into());
        let db_password = std::env::var("DB_PASSWORD").unwrap_or("state_pass".into());

        let im = Arc::new(InMemoryAdapter::new());
        im.set("request-attributes-user-key", scalar("1"));
        let mut state = make_state(
            Arc::new(EnvAdapter::new()),
            Arc::new(KVSAdapter::new().unwrap()),
            Arc::new(DbAdapter::new()),
            im,
            Arc::new(HttpAdapter),
        );

        state.set("cache.user.org_id", scalar("100"), Some(14400)).unwrap();
        state.set("cache.user.tenant_id", scalar("1"), Some(14400)).unwrap();

        let tenant_conn = Value::Mapping(vec![
            (b"tag".to_vec(),      scalar("tenant")),
            (b"id".to_vec(),       scalar("1")),
            (b"host".to_vec(),     scalar(&db_host)),
            (b"port".to_vec(),     scalar(&db_port)),
            (b"database".to_vec(), scalar(&db_database)),
            (b"username".to_vec(), scalar(&db_username)),
            (b"password".to_vec(), scalar(&db_password)),
        ]);
        state.set("connection.tenant", tenant_conn, None).unwrap();
        state.delete("cache.user").ok();

        let result = state.get("cache.user").unwrap();
        assert!(result.is_some(), "cache.user should be loaded from DB");
        let obj = result.unwrap();
        assert!(mapping_get(&obj, b"id").is_some(), "id should be present");
        assert!(mapping_get(&obj, b"org_id").is_some(), "org_id should be present");
    });

    // =========================================================================
    // cache.tenant.health: HTTP load (mock)
    // Prerequisite: cache.user.tenant_id must be set before cache.tenant can resolve
    // =========================================================================
    println!("\n[cache.tenant.health HTTP load]");

    test!("get cache.tenant.health loads from HTTP after cache.user is set", {
        let im = Arc::new(InMemoryAdapter::new());
        im.set("request-attributes-user-key", scalar("1"));
        let mut state = make_state(
            Arc::new(EnvAdapter::new()),
            Arc::new(KVSAdapter::new().unwrap()),
            Arc::new(DbAdapter::new()),
            im,
            Arc::new(HttpAdapter),
        );

        // cache.tenant._store key = "tenant:${cache.user.tenant_id}" — must be resolvable
        // cache.tenant._load (Db) would be tried first, but Db returns None (stub)
        // cache.tenant.health._load (HTTP) overrides at leaf level → mock returns {status: ok}
        state.set("cache.user.tenant_id", scalar("42"), Some(3600)).unwrap();

        let result = state.get("cache.tenant.health").unwrap();
        assert!(result.is_some(), "cache.tenant.health should be loaded from HTTP");
        let obj = result.unwrap();
        assert_eq!(mapping_get(&obj, b"status"), Some(&scalar("ok")));
    });

    // =========================================================================
    // placeholder resolution error cases
    // =========================================================================
    println!("\n[placeholder]");

    test!("set cache.user without session.sso_user_id returns Err", {
        let im = Arc::new(InMemoryAdapter::new());
        let mut state = make_state(
            Arc::new(EnvAdapter::new()),
            Arc::new(KVSAdapter::new().unwrap()),
            Arc::new(DbAdapter::new()),
            im,
            Arc::new(HttpAdapter),
        );

        let result = state.set("cache.user", Value::Mapping(vec![(b"id".to_vec(), scalar("1"))]), None);
        assert!(result.is_err(), "should fail when placeholder cannot be resolved");
    });

    test!("get cache.user without session.sso_user_id returns Err", {
        let im = Arc::new(InMemoryAdapter::new());
        let mut state = make_state(
            Arc::new(EnvAdapter::new()),
            Arc::new(KVSAdapter::new().unwrap()),
            Arc::new(DbAdapter::new()),
            im,
            Arc::new(HttpAdapter),
        );

        let result = state.get("cache.user");
        assert!(result.is_err(), "should fail when placeholder cannot be resolved");
    });

    test!("get nonexistent key returns KeyNotFound", {
        let im = Arc::new(InMemoryAdapter::new());
        let mut state = make_state(
            Arc::new(EnvAdapter::new()),
            Arc::new(KVSAdapter::new().unwrap()),
            Arc::new(DbAdapter::new()),
            im,
            Arc::new(HttpAdapter),
        );

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
