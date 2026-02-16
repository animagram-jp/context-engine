use serde_json::Value;

/// # Examples
/// ```
/// use state_engine::common::log_format::LogFormat;
///
/// let method_message = LogFormat::method("State", "get", &["'key'".to_string()]);
/// assert_eq!(method_message, "State::get('key')");
/// let error_message = LogFormat::error("State", "get", "not found");
/// assert_eq!(error_message, "State::get: not found");
/// ```
pub struct LogFormat;

impl LogFormat {

    pub fn method(class: &str, method: &str, args: &[String]) -> String {
        let args_str = args.join(", ");
        format!("{}::{}({})", class, method, args_str)
    }

    pub fn error(class: &str, method: &str, message: &str) -> String {
        format!("{}::{}: {}", class, method, message)
    }

    /// Format JSON value for log output
    ///
    /// # Examples
    /// ```
    /// use state_engine::common::log_format::LogFormat;
    /// use serde_json::json;
    ///
    /// assert_eq!(LogFormat::format_arg(&json!("text")), "'text'");
    /// assert_eq!(LogFormat::format_arg(&json!(42)), "42");
    /// assert_eq!(LogFormat::format_arg(&json!(true)), "true");
    /// assert_eq!(LogFormat::format_arg(&json!(null)), "null");
    /// assert_eq!(LogFormat::format_arg(&json!([])), "[]");
    /// assert_eq!(LogFormat::format_arg(&json!({})), "{}");
    /// assert_eq!(LogFormat::format_arg(&json!([1, 2, 3])), "[3 items]");
    /// assert_eq!(LogFormat::format_arg(&json!({"a": 1})), "{1 fields}");
    /// ```
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

    /// Format string argument for log output
    ///
    /// # Examples
    /// ```
    /// use state_engine::common::log_format::LogFormat;
    ///
    /// assert_eq!(LogFormat::format_str_arg("key"), "'key'");
    /// ```
    pub fn format_str_arg(s: &str) -> String {
        if s.len() > 50 {
            format!("'{}'...", &s[..47])
        } else {
            format!("'{}'", s)
        }
    }
}

/// Log macro: method call
///
/// # Examples
/// ```ignore
/// use crate::method_log;
///
/// method_log!("State", "get", "cache.user");
/// // Logs: State::get('cache.user')
/// ```
#[macro_export]
macro_rules! method_log {
    ($class:expr, $method:expr $(, $arg:expr)*) => {{
        #[cfg(feature = "logging")]
        {
            let args: Vec<String> = vec![
                $(
                    $crate::common::log_format::LogFormat::format_str_arg($arg),
                )*
            ];
            log::debug!("{}", $crate::common::log_format::LogFormat::method($class, $method, &args));
        }
    }};
}

/// Log macro: error
///
/// # Examples
/// ```ignore
/// use state_engine::error_log;
///
/// error_log!("State", "get", "metadata not found");
/// // Logs: State::get: metadata not found
/// ```
#[macro_export]
macro_rules! error_log {
    ($class:expr, $method:expr, $msg:expr) => {{
        #[cfg(feature = "logging")]
        {
            log::error!("{}", $crate::common::log_format::LogFormat::error($class, $method, $msg));
        }
    }};
}

/// Log macro: warning
///
/// # Examples
/// ```ignore
/// use crate::warn_log;
///
/// warn_log!("State", "resolve_config_placeholders", "unresolved placeholders: session.id");
/// // Logs: State::resolve_config_placeholders: unresolved placeholders: session.id
/// ```
#[macro_export]
macro_rules! warn_log {
    ($class:expr, $method:expr, $msg:expr) => {{
        #[cfg(feature = "logging")]
        {
            log::warn!("{}", $crate::common::log_format::LogFormat::error($class, $method, $msg));
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_method_multiple_args() {
        let result = LogFormat::method("State", "get", &[
            "'cache.user'".to_string(),
            "null".to_string(),
        ]);
        assert_eq!(result, "State::get('cache.user', null)");
    }

    #[test]
    fn test_format_arg_long_string() {
        let long_str = "a".repeat(60);
        let result = LogFormat::format_arg(&json!(long_str));
        assert!(result.starts_with("'aaa"));
        assert!(result.ends_with("'..."));
        assert_eq!(result.len(), 52); // ' + 47 chars + '...
    }

    #[test]
    fn test_format_str_arg_long_string() {
        let long_str = "a".repeat(60);
        let result = LogFormat::format_str_arg(&long_str);
        assert!(result.starts_with("'aaa"));
        assert!(result.ends_with("'..."));
    }
}
