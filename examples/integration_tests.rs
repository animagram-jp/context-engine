extern crate std;

#[path = "implements.rs"]
mod implements;

use implements::MyStores;
use context_engine::context::Context;
use context_engine::provided::{Context as ContextTrait, ContextError};
use context_engine::{Index, Tree, debug_log};
use context_engine::list::{List, VariableList};
use std::sync::Arc;
use env_logger;

// ── fixture ───────────────────────────────────────────────────────────────────

include!("../src/dsl_compiled.rs");

fn make_context<'r>(stores: &'r MyStores) -> Context<'r> {
    let index = Arc::new(Index::new(
        List  { data: PATHS.to_vec() },
        VariableList { identity: CHILDREN_IDENTITY.iter().map(|&x| x as usize).collect(), data: CHILDREN_DATA.to_vec() },
        VariableList { identity: LEAVES_IDENTITY.iter().map(|&x| x as usize).collect(),   data: LEAVES_DATA.to_vec() },
        VariableList { identity: VALUES_IDENTITY.iter().map(|&x| x as usize).collect(),   data: VALUES_DATA.to_vec() },
        VariableList { identity: WORDS_IDENTITY.iter().map(|&x| x as usize).collect(),    data: WORDS_DATA.to_vec() },
        VariableList { identity: MAP_KEYS_IDENTITY.iter().map(|&x| x as usize).collect(), data: MAP_KEYS_DATA.to_vec() },
        VariableList { identity: MAP_VALS_IDENTITY.iter().map(|&x| x as usize).collect(), data: MAP_VALS_DATA.to_vec() },
        VariableList { identity: ARGS_KEYS_IDENTITY.iter().map(|&x| x as usize).collect(),data: ARGS_KEYS_DATA.to_vec() },
        VariableList { identity: ARGS_VALS_IDENTITY.iter().map(|&x| x as usize).collect(),data: ARGS_VALS_DATA.to_vec() },
    ));
    Context::new(index, stores)
}

fn scalar(s: &str) -> Tree {
    Tree::Scalar(s.as_bytes().to_vec())
}

// ── test runner ───────────────────────────────────────────────────────────────

fn main() {
    env_logger::init();
    let mut passed = 0usize;
    let mut failed = 0usize;

    macro_rules! test {
        ($name:expr, $body:block) => {{
            std::print!("  {} ... ", $name);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body));
            match result {
                Ok(()) => { std::println!("ok"); passed += 1; }
                Err(_) => { std::println!("FAILED"); failed += 1; }
            }
        }};
    }

    // =========================================================================
    // session.user.id
    // _get: Memory(key="request.authorization.user"), _set: Kvs(inherited)
    // =========================================================================
    std::println!("\n[session.user.id]");

    test!("get gets from Memory when key is preset", {
        let stores = MyStores::new();
        stores.memory_set("request.authorization.user", scalar("42"));
        let mut context = make_context(&stores);
        let got = context.get("session.user.id").unwrap();
        assert_eq!(got, Some(scalar("42")));
    });

    test!("get returns None when Memory has no key", {
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        // _get returns None → LoadFailed(NotFound)
        let result = context.get("session.user.id");
        assert!(matches!(result, Err(ContextError::LoadFailed(_))));
    });

    test!("get cache hit on second call", {
        let stores = MyStores::new();
        stores.memory_set("request.authorization.user", scalar("42"));
        let mut context = make_context(&stores);
        context.get("session.user.id").unwrap();
        // Memory cleared — second get must come from cache
        stores.memory_clear();
        let got = context.get("session.user.id").unwrap();
        assert_eq!(got, Some(scalar("42")));
    });

    test!("set writes to Kvs and cache", {
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        assert!(context.set("session.user.id", scalar("99")).unwrap());
        let got = context.get("session.user.id").unwrap();
        assert_eq!(got, Some(scalar("99")));
    });

    test!("exists true after set", {
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        context.set("session.user.id", scalar("1")).unwrap();
        assert!(context.exists("session.user.id").unwrap());
    });

    test!("exists false before set", {
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        assert!(!context.exists("session.user.id").unwrap());
    });

    test!("delete removes from Kvs and cache", {
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        context.set("session.user.id", scalar("1")).unwrap();
        assert!(context.delete("session.user.id").unwrap());
        assert!(!context.exists("session.user.id").unwrap());
    });

    // =========================================================================
    // session.user.name
    // _get: TenantDb(key="users.id.${session.user.id}") — placeholder依存
    // _set: Kvs(inherited)
    // =========================================================================
    std::println!("\n[session.user.name — placeholder in key]");

    test!("set and get without placeholder resolution", {
        // key contains ${session.user.id} but set bypasses _get
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        assert!(context.set("session.user.name", scalar("alice")).unwrap());
        let got = context.get("session.user.name").unwrap();
        assert_eq!(got, Some(scalar("alice")));
    });

    test!("get via _get resolves key placeholder and expands map", {
        let stores = MyStores::new();
        stores.memory_set("request.authorization.user", scalar("1"));
        stores.tenant_db_set("users.id.1", Tree::Mapping(std::vec![
            (b"name".to_vec(),          Tree::Scalar(b"alice".to_vec())),
            (b"email".to_vec(),         Tree::Scalar(b"alice@example.com".to_vec())),
            (b"password_hash".to_vec(), Tree::Scalar(b"hash".to_vec())),
            (b"is_manager".to_vec(),    Tree::Scalar(b"false".to_vec())),
            (b"color_mode".to_vec(),    Tree::Scalar(b"dark".to_vec())),
        ]));
        let mut context = make_context(&stores);
        let got = context.get("session.user.name").unwrap();
        assert_eq!(got, Some(scalar("alice")));
        let got_email = context.get("session.user.email").unwrap();
        assert_eq!(got_email, Some(scalar("alice@example.com")));
    });

    // =========================================================================
    // session.user.name_copy
    // value: ${session.user.name} — single placeholder, type-preserving copy
    // =========================================================================
    std::println!("\n[session.user.name_copy — placeholder value]");

    test!("get resolves placeholder to session.user.name value", {
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        context.set("session.user.name", scalar("alice")).unwrap();
        let got = context.get("session.user.name_copy").unwrap();
        assert_eq!(got, Some(scalar("alice")));
    });

    test!("get returns LoadFailed when referenced path has no value", {
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        // session.user.name not set, _get will fail
        let result = context.get("session.user.name_copy");
        debug_log!("integration_test", "get returns LoadFailed", &format!("{:?}", result));
        assert!(matches!(result, Err(ContextError::LoadFailed(_))));
    });

    // =========================================================================
    // session.user.tenant.id
    // _get: Memory(key="request.authorization.tenant"), _set: Kvs(inherited from session.user)
    // =========================================================================
    std::println!("\n[session.user.tenant.id]");

    test!("get gets from Memory", {
        let stores = MyStores::new();
        stores.memory_set("request.authorization.tenant", scalar("10"));
        let mut context = make_context(&stores);
        let got = context.get("session.user.tenant.id").unwrap();
        assert_eq!(got, Some(scalar("10")));
    });

    test!("set and get", {
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        assert!(context.set("session.user.tenant.id", scalar("10")).unwrap());
        let got = context.get("session.user.tenant.id").unwrap();
        assert_eq!(got, Some(scalar("10")));
    });

    // =========================================================================
    // connection.common_db — static leaf values
    // _get: Env, static values: driver="postgres", charset="UTF8"
    // =========================================================================
    std::println!("\n[connection.common_db — static values]");

    test!("get driver returns static value postgres", {
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        let got = context.get("connection.common_db.driver").unwrap();
        assert_eq!(got, Some(scalar("postgres")));
    });

    test!("get charset returns static value UTF8", {
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        let got = context.get("connection.common_db.charset").unwrap();
        assert_eq!(got, Some(scalar("UTF8")));
    });

    test!("get host returns None when Env not set", {
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        let result = context.get("connection.common_db.host");
        assert!(matches!(result, Err(ContextError::LoadFailed(_))));
    });

    // =========================================================================
    // connection.tenant_db — static leaf values
    // =========================================================================
    std::println!("\n[connection.tenant_db — static values]");

    test!("get driver returns static value postgres", {
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        let got = context.get("connection.tenant_db.driver").unwrap();
        assert_eq!(got, Some(scalar("postgres")));
    });

    // =========================================================================
    // recursion guard
    // =========================================================================
    std::println!("\n[recursion]");

    test!("get same path twice in sequence does not recurse", {
        let stores = MyStores::new();
        stores.memory_set("request.authorization.user", scalar("1"));
        let mut context = make_context(&stores);
        context.get("session.user.id").unwrap();
        // second independent call — called_paths should be cleared between calls
        let got = context.get("session.user.id").unwrap();
        assert_eq!(got, Some(scalar("1")));
    });

    // =========================================================================
    // KeyNotFound
    // =========================================================================
    std::println!("\n[KeyNotFound]");

    test!("get nonexistent path", {
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        let result = context.get("session.user.nonexistent");
        assert!(matches!(result, Err(ContextError::KeyNotFound(_))));
    });

    test!("set nonexistent path", {
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        let result = context.set("session.user.nonexistent", scalar("x"));
        assert!(matches!(result, Err(ContextError::KeyNotFound(_))));
    });

    test!("delete nonexistent path", {
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        let result = context.delete("session.nonexistent");
        assert!(matches!(result, Err(ContextError::KeyNotFound(_))));
    });

    test!("exists nonexistent path", {
        let stores = MyStores::new();
        let mut context = make_context(&stores);
        let result = context.exists("session.nonexistent");
        assert!(matches!(result, Err(ContextError::KeyNotFound(_))));
    });

    // =========================================================================
    // recursion limit — circular dependency detection via called_paths
    // =========================================================================
    std::println!("\n[RecursionLimitExceeded]");

    test!("get with circular dependency returns RecursionLimitExceeded", {
        // a._get.key = "${b}", b._get.key = "${a}" — circular
        let tree = Tree::Mapping(std::vec![
            (b"a".to_vec(), Tree::Mapping(std::vec![
                (b"_get".to_vec(), Tree::Mapping(std::vec![
                    (b"store".to_vec(), Tree::Scalar(b"Memory".to_vec())),
                    (b"key".to_vec(),   Tree::Scalar(b"${b}".to_vec())),
                ])),
            ])),
            (b"b".to_vec(), Tree::Mapping(std::vec![
                (b"_get".to_vec(), Tree::Mapping(std::vec![
                    (b"store".to_vec(), Tree::Scalar(b"Memory".to_vec())),
                    (b"key".to_vec(),   Tree::Scalar(b"${a}".to_vec())),
                ])),
            ])),
        ]);
        let (paths, ch, leaves, values, words, map_keys, map_vals, args_keys, args_vals)
            = context_engine::dsl::Dsl::compile(&tree, &["Memory"]).unwrap();
        let index = Arc::new(Index::new(paths, ch, leaves, values, words, map_keys, map_vals, args_keys, args_vals));
        let stores = MyStores::new();
        let mut context = Context::new(index, &stores);
        let result = context.get("a");
        assert!(matches!(result, Err(ContextError::RecursionLimitExceeded)));
    });

    // =========================================================================
    // results
    // =========================================================================
    std::println!("\n{} passed, {} failed", passed, failed);
    if failed > 0 {
        std::process::exit(1);
    }
}
