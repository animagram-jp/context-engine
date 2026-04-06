fn main() {
    let args: std::vec::Vec<std::string::String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: precompile <input.yml> <output.rs>");
        std::process::exit(1);
    }
    let src = std::fs::read(&args[1])
        .unwrap_or_else(|e| { eprintln!("read error: {e}"); std::process::exit(1); });
    context_engine::dsl::Dsl::write(&src, &args[2])
        .unwrap_or_else(|e| { eprintln!("compile error: {e}"); std::process::exit(1); });
    println!("written: {}", args[2]);
}
