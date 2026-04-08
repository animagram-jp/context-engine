extern crate std;

#[path = "implements.rs"]
mod implements;

use implements::MyRegistry;
use context_engine::context::Context;
use context_engine::ports::provided::{Context as ContextTrait, ContextError};
use context_engine::{Index, Tree};
use std::sync::Arc;

// ── fixture ───────────────────────────────────────────────────────────────────

fn make_context<'r>(registry: &'r MyRegistry) -> Context<'r> {
    let src = include_bytes!("tenant.yml");
    let tree = context_engine::dsl::parse_yaml(src).expect("parse failed");
    let (paths, children, leaves, interning, interning_idx) = context_engine::dsl::Dsl::compile(&tree);
    let index = Arc::new(Index::new(paths, children, leaves, interning, interning_idx));
    Context::new(index, registry)
}

fn scalar(s: &str) -> Tree {
    Tree::Scalar(s.as_bytes().to_vec())
}

// ── test runner ───────────────────────────────────────────────────────────────

fn main() {
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
    // _load: Memory(key="request.authorization.user"), _store: Kvs(inherited)
    // =========================================================================
    std::println!("\n[session.user.id]");

    test!("get loads from Memory when key is preset", {
        let registry = MyRegistry::new();
        registry.memory_set("request.authorization.user", scalar("42"));
        let mut ctx = make_context(&registry);
        let got = ctx.get("session.user.id").unwrap();
        assert_eq!(got, Some(scalar("42")));
    });

    test!("get returns None when Memory has no key", {
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        // _load returns None → LoadFailed(NotFound)
        let result = ctx.get("session.user.id");
        assert!(matches!(result, Err(ContextError::LoadFailed(_))));
    });

    test!("get cache hit on second call", {
        let registry = MyRegistry::new();
        registry.memory_set("request.authorization.user", scalar("42"));
        let mut ctx = make_context(&registry);
        ctx.get("session.user.id").unwrap();
        // Memory cleared — second get must come from cache
        registry.memory_clear();
        let got = ctx.get("session.user.id").unwrap();
        assert_eq!(got, Some(scalar("42")));
    });

    test!("set writes to Kvs and cache", {
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        assert!(ctx.set("session.user.id", scalar("99")).unwrap());
        let got = ctx.get("session.user.id").unwrap();
        assert_eq!(got, Some(scalar("99")));
    });

    test!("exists true after set", {
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        ctx.set("session.user.id", scalar("1")).unwrap();
        assert!(ctx.exists("session.user.id").unwrap());
    });

    test!("exists false before set", {
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        assert!(!ctx.exists("session.user.id").unwrap());
    });

    test!("delete removes from Kvs and cache", {
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        ctx.set("session.user.id", scalar("1")).unwrap();
        assert!(ctx.delete("session.user.id").unwrap());
        assert!(!ctx.exists("session.user.id").unwrap());
    });

    // =========================================================================
    // session.user.name
    // _load: TenantDb(key="users.id.${session.user.id}") — placeholder依存
    // _store: Kvs(inherited)
    // =========================================================================
    std::println!("\n[session.user.name — placeholder in key]");

    test!("set and get without placeholder resolution", {
        // key contains ${session.user.id} but set bypasses _load
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        assert!(ctx.set("session.user.name", scalar("alice")).unwrap());
        let got = ctx.get("session.user.name").unwrap();
        assert_eq!(got, Some(scalar("alice")));
    });

    // =========================================================================
    // session.user.name_copy
    // value: ${session.user.name} — single placeholder, type-preserving copy
    // =========================================================================
    std::println!("\n[session.user.name_copy — placeholder value]");

    test!("get resolves placeholder to session.user.name value", {
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        ctx.set("session.user.name", scalar("alice")).unwrap();
        let got = ctx.get("session.user.name_copy").unwrap();
        assert_eq!(got, Some(scalar("alice")));
    });

    test!("get returns LoadFailed when referenced path has no value", {
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        // session.user.name not set, _load will fail
        let result = ctx.get("session.user.name_copy");
        assert!(matches!(result, Err(ContextError::LoadFailed(_))));
    });

    // =========================================================================
    // session.user.tenant.id
    // _load: Memory(key="request.authorization.tenant"), _store: Kvs(inherited from session.user)
    // =========================================================================
    std::println!("\n[session.user.tenant.id]");

    test!("get loads from Memory", {
        let registry = MyRegistry::new();
        registry.memory_set("request.authorization.tenant", scalar("10"));
        let mut ctx = make_context(&registry);
        let got = ctx.get("session.user.tenant.id").unwrap();
        assert_eq!(got, Some(scalar("10")));
    });

    test!("set and get", {
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        assert!(ctx.set("session.user.tenant.id", scalar("10")).unwrap());
        let got = ctx.get("session.user.tenant.id").unwrap();
        assert_eq!(got, Some(scalar("10")));
    });

    // =========================================================================
    // connection.common_db — static leaf values
    // _load: Env, static values: driver="postgres", charset="UTF8"
    // =========================================================================
    std::println!("\n[connection.common_db — static values]");

    test!("get driver returns static value postgres", {
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        let got = ctx.get("connection.common_db.driver").unwrap();
        assert_eq!(got, Some(scalar("postgres")));
    });

    test!("get charset returns static value UTF8", {
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        let got = ctx.get("connection.common_db.charset").unwrap();
        assert_eq!(got, Some(scalar("UTF8")));
    });

    test!("get host returns None when Env not set", {
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        // Env has no _load client registered for connection.common_db.host
        let result = ctx.get("connection.common_db.host");
        assert!(matches!(result, Err(ContextError::LoadFailed(_))));
    });

    // =========================================================================
    // connection.tenant_db — static leaf values
    // =========================================================================
    std::println!("\n[connection.tenant_db — static values]");

    test!("get driver returns static value postgres", {
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        let got = ctx.get("connection.tenant_db.driver").unwrap();
        assert_eq!(got, Some(scalar("postgres")));
    });

    // =========================================================================
    // recursion guard
    // =========================================================================
    std::println!("\n[recursion]");

    test!("get same path twice in sequence does not recurse", {
        let registry = MyRegistry::new();
        registry.memory_set("request.authorization.user", scalar("1"));
        let mut ctx = make_context(&registry);
        ctx.get("session.user.id").unwrap();
        // second independent call — called_paths should be cleared between calls
        let got = ctx.get("session.user.id").unwrap();
        assert_eq!(got, Some(scalar("1")));
    });

    // =========================================================================
    // KeyNotFound
    // =========================================================================
    std::println!("\n[KeyNotFound]");

    test!("get nonexistent path", {
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        let result = ctx.get("session.user.nonexistent");
        assert!(matches!(result, Err(ContextError::KeyNotFound(_))));
    });

    test!("set nonexistent path", {
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        let result = ctx.set("session.user.nonexistent", scalar("x"));
        assert!(matches!(result, Err(ContextError::KeyNotFound(_))));
    });

    test!("delete nonexistent path", {
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        let result = ctx.delete("session.nonexistent");
        assert!(matches!(result, Err(ContextError::KeyNotFound(_))));
    });

    test!("exists nonexistent path", {
        let registry = MyRegistry::new();
        let mut ctx = make_context(&registry);
        let result = ctx.exists("session.nonexistent");
        assert!(matches!(result, Err(ContextError::KeyNotFound(_))));
    });

    // =========================================================================
    // results
    // =========================================================================
    std::println!("\n{} passed, {} failed", passed, failed);
    if failed > 0 {
        std::process::exit(1);
    }
}
