/// DbClient implementation using PostgreSQL
///
/// Implements the DbClient Required Port.

use state_engine::ports::required::DbClient;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct DbAdapter {
    pool: Mutex<HashMap<String, tokio_postgres::Client>>,
}

impl DbAdapter {
    pub fn new() -> Self {
        Self {
            pool: Mutex::new(HashMap::new()),
        }
    }

    fn get_connection_name(config: &Value) -> Result<String, String> {
        let tag = config.get("tag")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'tag' field in connection config".to_string())?;

        if tag == "common" {
            Ok(format!("connection.{}", tag))
        } else if tag == "tenant" {
            let id = config.get("id")
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    _ => None,
                })
                .ok_or_else(|| "Missing 'id' field for tenant connection".to_string())?;
            Ok(format!("connection.{}{}", tag, id))
        } else {
            Err(format!("Unsupported tag: {}", tag))
        }
    }

    async fn connect_from_config(config: &Value) -> Result<tokio_postgres::Client, String> {
        let config_obj = config.as_object()
            .ok_or("connection must be an object")?;

        let host = config_obj.get("host").and_then(|v| v.as_str()).ok_or("Missing host")?;
        let port = config_obj.get("port").and_then(|v| v.as_u64()).unwrap_or(5432) as u16;
        let database = config_obj.get("database").and_then(|v| v.as_str()).ok_or("Missing database")?;
        let username = config_obj.get("username").and_then(|v| v.as_str()).ok_or("Missing username")?;
        let password = config_obj.get("password").and_then(|v| v.as_str()).ok_or("Missing password")?;

        let conn_str = format!(
            "host={} port={} dbname={} user={} password={}",
            host, port, database, username, password
        );

        let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls)
            .await
            .map_err(|e| format!("Failed to connect: {}", e))?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        Ok(client)
    }
}

impl DbClient for DbAdapter {
    fn get(
        &self,
        connection: &Value,
        table: &str,
        columns: &[&str],
        where_clause: Option<&str>,
    ) -> Option<Vec<HashMap<String, Value>>> {
        let runtime = tokio::runtime::Runtime::new().ok()?;

        runtime.block_on(async {
            let conn_name = Self::get_connection_name(connection).ok()?;
            let mut pool_lock = self.pool.lock().unwrap();

            if !pool_lock.contains_key(&conn_name) {
                let client = Self::connect_from_config(connection).await.ok()?;
                pool_lock.insert(conn_name.clone(), client);
            }

            let client = pool_lock.get(&conn_name)?;
            let column_list = if columns.is_empty() { "*".to_string() } else { columns.join(", ") };

            let query = if let Some(wc) = where_clause {
                format!("SELECT {} FROM {} WHERE {}", column_list, table, wc)
            } else {
                format!("SELECT {} FROM {}", column_list, table)
            };

            let rows = client.query(&query, &[]).await.ok()?;

            let mut results = Vec::new();
            for row in rows {
                let mut map = HashMap::new();
                for (idx, column) in row.columns().iter().enumerate() {
                    let value: Value = match column.type_() {
                        &tokio_postgres::types::Type::INT4 => {
                            row.try_get::<_, i32>(idx).map(|v| serde_json::json!(v)).unwrap_or(Value::Null)
                        }
                        &tokio_postgres::types::Type::INT8 => {
                            row.try_get::<_, i64>(idx).map(|v| serde_json::json!(v)).unwrap_or(Value::Null)
                        }
                        &tokio_postgres::types::Type::TEXT | &tokio_postgres::types::Type::VARCHAR => {
                            row.try_get::<_, String>(idx).map(|v| serde_json::json!(v)).unwrap_or(Value::Null)
                        }
                        &tokio_postgres::types::Type::BOOL => {
                            row.try_get::<_, bool>(idx).map(|v| serde_json::json!(v)).unwrap_or(Value::Null)
                        }
                        _ => Value::Null,
                    };
                    map.insert(column.name().to_string(), value);
                }
                results.push(map);
            }

            Some(results)
        })
    }

    fn set(
        &self,
        _connection: &Value,
        _table: &str,
        _values: &HashMap<String, Value>,
        _where_clause: Option<&str>,
    ) -> bool { false }

    fn delete(
        &self,
        _connection: &Value,
        _table: &str,
        _where_clause: Option<&str>,
    ) -> bool { false }
}
