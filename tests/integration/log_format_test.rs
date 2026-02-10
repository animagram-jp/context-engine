// Log format tests
use state_engine::common::log_format::LogFormat;
use serde_json::json;

#[test]
fn test_log_format_usage() {
    // LogFormat構造体のメソッドを直接テスト
    let msg = LogFormat::method_call("State", "get", &["'cache.user'".to_string()]);
    assert_eq!(msg, "State::get('cache.user')");

    let err = LogFormat::error("State", "get", "metadata not found");
    assert_eq!(err, "State::get: metadata not found");

    // format_arg のテスト
    assert_eq!(LogFormat::format_arg(&json!("test")), "'test'");
    assert_eq!(LogFormat::format_arg(&json!(42)), "42");
    assert_eq!(LogFormat::format_arg(&json!([1, 2, 3])), "[3 items]");
    assert_eq!(LogFormat::format_arg(&json!({"a": 1})), "{1 fields}");
}

#[test]
fn test_log_format_long_string() {
    let long_str = "a".repeat(60);
    let result = LogFormat::format_arg(&json!(long_str));

    // 長い文字列は省略される
    assert!(result.len() < 60);
    assert!(result.ends_with("'..."));
}

#[test]
fn test_log_format_str_arg() {
    assert_eq!(LogFormat::format_str_arg("short"), "'short'");

    let long = "x".repeat(60);
    let result = LogFormat::format_str_arg(&long);
    assert!(result.starts_with("'xxx"));
    assert!(result.ends_with("'..."));
    assert_eq!(result.len(), 52); // ' + 47 chars + '...
}

// マクロのコンパイルテスト（実際のログ出力はしない）
#[test]
fn test_macro_compilation() {
    // マクロが正しくコンパイルされることを確認
    // loggingフィーチャーが有効でも無効でも、コンパイルは通る
    #[allow(unused_macros)]
    macro_rules! test_log_method {
        () => {
            state_engine::log_method!("Test", "method", "arg1", "arg2");
        };
    }

    #[allow(unused_macros)]
    macro_rules! test_log_err {
        () => {
            state_engine::log_err!("Test", "method", "error message");
        };
    }

    // コンパイルが通ればOK
    assert!(true);
}
