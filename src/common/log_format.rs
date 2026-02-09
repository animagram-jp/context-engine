// LogFormat - ログメッセージの統一フォーマット
//
// declare-engine の LogMethodCall trait と同等の機能を提供

use serde_json::Value;

/// ログメッセージのフォーマットユーティリティ
pub struct LogFormat;

impl LogFormat {
    /// メソッド呼び出しログを生成
    ///
    /// 例: `State::get('cache.user')`
    ///
    /// # Arguments
    /// * `class` - クラス名
    /// * `method` - メソッド名
    /// * `args` - 引数のスライス
    pub fn method_call(class: &str, method: &str, args: &[String]) -> String {
        let args_str = args.join(", ");
        format!("{}::{}({})", class, method, args_str)
    }

    /// エラーログを生成
    ///
    /// 例: `State::get: metadata not found`
    ///
    /// # Arguments
    /// * `class` - クラス名
    /// * `method` - メソッド名
    /// * `message` - エラーメッセージ
    pub fn error(class: &str, method: &str, message: &str) -> String {
        format!("{}::{}: {}", class, method, message)
    }

    /// 引数を読みやすくフォーマット（declare-engine の formatArgs 相当）
    ///
    /// - 文字列: 50文字で省略
    /// - 配列: 要素数を表示
    /// - オブジェクト: フィールド数を表示
    /// - null/bool/数値: そのまま
    pub fn format_arg(value: &Value) -> String {
        match value {
            Value::String(s) if s.len() > 50 => {
                format!("'{}'...", &s[..47])
            }
            Value::String(s) => {
                format!("'{}'", s)
            }
            Value::Array(arr) => {
                if arr.is_empty() {
                    "[]".to_string()
                } else {
                    format!("[{} items]", arr.len())
                }
            }
            Value::Object(obj) => {
                if obj.is_empty() {
                    "{}".to_string()
                } else {
                    format!("{{{} fields}}", obj.len())
                }
            }
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
        }
    }

    /// 文字列引数をフォーマット
    pub fn format_str_arg(s: &str) -> String {
        if s.len() > 50 {
            format!("'{}'...", &s[..47])
        } else {
            format!("'{}'", s)
        }
    }
}

/// ログマクロ: メソッド呼び出し
///
/// # Examples
/// ```ignore
/// use state_engine::log_method;
///
/// log_method!("State", "get", "cache.user");
/// // Logs: State::get('cache.user')
/// ```
#[macro_export]
macro_rules! log_method {
    ($class:expr, $method:expr $(, $arg:expr)*) => {{
        #[cfg(feature = "logging")]
        {
            let args: Vec<String> = vec![
                $(
                    $crate::common::log_format::LogFormat::format_str_arg($arg),
                )*
            ];
            log::debug!("{}", $crate::common::log_format::LogFormat::method_call($class, $method, &args));
        }
    }};
}

/// ログマクロ: エラー
///
/// # Examples
/// ```ignore
/// use state_engine::log_err;
///
/// log_err!("State", "get", "metadata not found");
/// // Logs: State::get: metadata not found
/// ```
#[macro_export]
macro_rules! log_err {
    ($class:expr, $method:expr, $msg:expr) => {{
        #[cfg(feature = "logging")]
        {
            log::error!("{}", $crate::common::log_format::LogFormat::error($class, $method, $msg));
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_method_call() {
        let result = LogFormat::method_call("State", "get", &["'cache.user'".to_string()]);
        assert_eq!(result, "State::get('cache.user')");

        let result = LogFormat::method_call("State", "get", &[
            "'cache.user'".to_string(),
            "null".to_string(),
        ]);
        assert_eq!(result, "State::get('cache.user', null)");
    }

    #[test]
    fn test_error() {
        let result = LogFormat::error("State", "get", "metadata not found");
        assert_eq!(result, "State::get: metadata not found");
    }

    #[test]
    fn test_format_arg_string() {
        assert_eq!(LogFormat::format_arg(&json!("hello")), "'hello'");

        let long_str = "a".repeat(60);
        let result = LogFormat::format_arg(&json!(long_str));
        assert!(result.starts_with("'aaa"));
        assert!(result.ends_with("'..."));
        assert_eq!(result.len(), 52); // ' + 47 chars + '...
    }

    #[test]
    fn test_format_arg_array() {
        assert_eq!(LogFormat::format_arg(&json!([])), "[]");
        assert_eq!(LogFormat::format_arg(&json!([1, 2, 3])), "[3 items]");
    }

    #[test]
    fn test_format_arg_object() {
        assert_eq!(LogFormat::format_arg(&json!({})), "{}");
        assert_eq!(LogFormat::format_arg(&json!({"a": 1, "b": 2})), "{2 fields}");
    }

    #[test]
    fn test_format_arg_primitives() {
        assert_eq!(LogFormat::format_arg(&json!(null)), "null");
        assert_eq!(LogFormat::format_arg(&json!(true)), "true");
        assert_eq!(LogFormat::format_arg(&json!(false)), "false");
        assert_eq!(LogFormat::format_arg(&json!(42)), "42");
        assert_eq!(LogFormat::format_arg(&json!(3.14)), "3.14");
    }

    #[test]
    fn test_format_str_arg() {
        assert_eq!(LogFormat::format_str_arg("hello"), "'hello'");

        let long_str = "a".repeat(60);
        let result = LogFormat::format_str_arg(&long_str);
        assert!(result.starts_with("'aaa"));
        assert!(result.ends_with("'..."));
    }
}
