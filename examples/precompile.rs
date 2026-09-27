use context_engine::{Tree, dsl};

fn collect_store_ids(tree: &Tree, out: &mut std::vec::Vec<std::string::String>) {
    if let Tree::Mapping(pairs) = tree {
        for (k, v) in pairs {
            if k.as_slice() == dsl::PROP_STORE {
                if let Tree::Scalar(name) = v {
                    if let Ok(name) = std::str::from_utf8(name) {
                        if !out.iter().any(|s| s == name) {
                            out.push(name.to_string());
                        }
                    }
                }
                continue;
            }
            collect_store_ids(v, out);
        }
    }
}

fn main() {
    let args: std::vec::Vec<std::string::String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: precompile <input.yml> [output.rs]");
        std::process::exit(1);
    }
    let out = if args.len() >= 3 { args[2].clone() } else { "src/dsl_compiled.rs".to_string() };
    let src = std::fs::read(&args[1]).unwrap_or_else(|e| {
        eprintln!("read error: {e}");
        std::process::exit(1);
    });
    let tree = dsl::parse_yaml(&src).unwrap_or_else(|e| {
        eprintln!("parse error: {e}");
        std::process::exit(1);
    });
    let mut store_ids = std::vec::Vec::new();
    collect_store_ids(&tree, &mut store_ids);
    let store_ids: std::vec::Vec<&str> =
        store_ids.iter().map(std::string::String::as_str).collect();
    println!("store_ids: {store_ids:?}");
    context_engine::dsl::Dsl::write(&src, &store_ids, &out).unwrap_or_else(|e| {
        eprintln!("compile error: {e}");
        std::process::exit(1);
    });
    println!("written: {}", out);
}
