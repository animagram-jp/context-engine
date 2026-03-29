use crate::ports::provided::Value;

/// # Examples
/// ```
/// use state_engine::LogFormat;
///
/// let fn_message = LogFormat::call("State", "get", &["'key'".to_string()]);
/// assert_eq!(fn_message, "State::get('key')");
/// ```
pub struct LogFormat;

impl LogFormat {
    pub fn call(class: &str, fn_name: &str, args: &[String]) -> String {
        let args_str = args.join(", ");
        format!("{}::{}({})", class, fn_name, args_str)
    }

    /// Format Value for log output
    ///
    /// # Examples
    /// ```
    /// use state_engine::{LogFormat, Value};
    ///
    /// assert_eq!(LogFormat::format_arg(&Value::Scalar(b"text".to_vec())), "'text'");
    /// assert_eq!(LogFormat::format_arg(&Value::Null), "null");
    /// assert_eq!(LogFormat::format_arg(&Value::Sequence(vec![])), "[]");
    /// assert_eq!(LogFormat::format_arg(&Value::Mapping(vec![])), "{}");
    /// assert_eq!(LogFormat::format_arg(&Value::Sequence(vec![Value::Null, Value::Null, Value::Null])), "[3 items]");
    /// assert_eq!(LogFormat::format_arg(&Value::Mapping(vec![(b"a".to_vec(), Value::Null)])), "{1 fields}");
    /// ```
    pub fn format_arg(value: &Value) -> String {
        match value {
            Value::Scalar(b) => {
                let s = String::from_utf8_lossy(b);
                if s.len() > 50 { format!("'{}'...", &s[..47]) } else { format!("'{}'", s) }
            }
            Value::Sequence(arr) if arr.is_empty() => "[]".to_string(),
            Value::Sequence(arr) => format!("[{} items]", arr.len()),
            Value::Mapping(obj) if obj.is_empty() => "{}".to_string(),
            Value::Mapping(obj) => format!("{{{} fields}}", obj.len()),
            Value::Null => "null".to_string(),
        }
    }

    /// Format string argument for log output
    ///
    /// # Examples
    /// ```
    /// use state_engine::LogFormat;
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

/// Log macro: fn call
///
/// # Examples
/// ```ignore
/// use crate::fn_log;
///
/// fn_log!("State", "get", "cache.user");
/// // Logs: State::get('cache.user')
/// ```
#[macro_export]
macro_rules! fn_log {
    ($class:expr, $fun:expr $(, $arg:expr)*) => {{
        #[cfg(feature = "logging")]
        {
            let args: Vec<String> = vec![
                $(
                    $crate::log_format::LogFormat::format_str_arg($arg),
                )*
            ];
            log::debug!("{}", $crate::log_format::LogFormat::call($class, $fun, &args));
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fn_multiple_args() {
        let result = LogFormat::call("State", "get", &[
            "'cache.user'".to_string(),
            "null".to_string(),
        ]);
        assert_eq!(result, "State::get('cache.user', null)");
    }

    #[test]
    fn test_format_arg_long_string() {
        let long_str = "a".repeat(60);
        let result = LogFormat::format_arg(&Value::Scalar(long_str.into_bytes()));
        assert!(result.starts_with("'aaa"));
        assert!(result.ends_with("'..."));
        assert_eq!(result.len(), 52);
    }

    #[test]
    fn test_format_str_arg_long_string() {
        let long_str = "a".repeat(60);
        let result = LogFormat::format_str_arg(&long_str);
        assert!(result.starts_with("'aaa"));
        assert!(result.ends_with("'..."));
    }
}
