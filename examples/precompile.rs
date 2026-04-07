fn main() {
    let args: std::vec::Vec<std::string::String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: precompile <input.yml> [output.rs]");
        std::process::exit(1);
    }
    let out = if args.len() >= 3 { args[2].clone() } else { "src/dsl_compiled.rs".to_string() };
    let src = std::fs::read(&args[1])
        .unwrap_or_else(|e| { eprintln!("read error: {e}"); std::process::exit(1); });
    context_engine::dsl::Dsl::write(&src, &out)
        .unwrap_or_else(|e| { eprintln!("compile error: {e}"); std::process::exit(1); });
    println!("written: {}", out);
}
