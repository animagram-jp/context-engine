/// DBClient implementation using PostgreSQL
///
/// Implements the DBClient Required Port.

use state_engine::ports::required::DBClient;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct DBAdapter {
    // Use Mutex for interior mutability with Send + Sync
    pool: Mutex<Option<tokio_postgres::Client>>,
}

impl DBAdapter {
    pub fn new() -> Self {
        Self {
            pool: Mutex::new(None),
        }
    }

    /// Create database connection from config
    async fn connect_from_config(config: &Value) -> Result<tokio_postgres::Client, String> {
        let config_obj = config.as_object()
            .ok_or("connection must be an object")?;

        let host = config_obj.get("host")
            .and_then(|v| v.as_str())
            .ok_or("Missing host in config")?;
        let port = config_obj.get("port")
            .and_then(|v| v.as_u64())
            .unwrap_or(5432) as u16;
        let database = config_obj.get("database")
            .and_then(|v| v.as_str())
            .ok_or("Missing database in config")?;
        let username = config_obj.get("username")
            .and_then(|v| v.as_str())
            .ok_or("Missing username in config")?;
        let password = config_obj.get("password")
            .and_then(|v| v.as_str())
            .ok_or("Missing password in config")?;

        let conn_str = format!(
            "host={} port={} dbname={} user={} password={}",
            host, port, database, username, password
        );

        let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls)
            .await
            .map_err(|e| format!("Failed to connect to database: {}", e))?;

        // Spawn connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        Ok(client)
    }
}

impl DBClient for DBAdapter {
    fn fetch(
        &self,
        connection: &Value,
        table: &str,
        columns: &[&str],
        where_clause: Option<&str>,
    ) -> Option<Vec<HashMap<String, Value>>> {
        // Create runtime for async operation
        let runtime = tokio::runtime::Runtime::new().ok()?;

        runtime.block_on(async {
            // Ensure we have a connection
            let mut pool_lock = self.pool.lock().unwrap();
            if pool_lock.is_none() {
                let client = Self::connect_from_config(connection).await.ok()?;
                *pool_lock = Some(client);
            }

            let client = pool_lock.as_ref()?;

            // Build SELECT clause
            let column_list = if columns.is_empty() {
                "*".to_string()
            } else {
                columns.join(", ")
            };

            let query = if let Some(wc) = where_clause {
                format!("SELECT {} FROM {} WHERE {}", column_list, table, wc)
            } else {
                format!("SELECT {} FROM {}", column_list, table)
            };

            let rows = client.query(&query, &[]).await.ok()?;

            // Convert rows to Vec<HashMap<String, Value>>
            let mut results = Vec::new();
            for row in rows {
                let mut map = HashMap::new();
                for (idx, column) in row.columns().iter().enumerate() {
                    let value: Value = match column.type_() {
                        &tokio_postgres::types::Type::INT4 => {
                            row.try_get::<_, i32>(idx)
                                .map(|v| serde_json::json!(v))
                                .unwrap_or(Value::Null)
                        }
                        &tokio_postgres::types::Type::INT8 => {
                            row.try_get::<_, i64>(idx)
                                .map(|v| serde_json::json!(v))
                                .unwrap_or(Value::Null)
                        }
                        &tokio_postgres::types::Type::TEXT | &tokio_postgres::types::Type::VARCHAR => {
                            row.try_get::<_, String>(idx)
                                .map(|v| serde_json::json!(v))
                                .unwrap_or(Value::Null)
                        }
                        &tokio_postgres::types::Type::BOOL => {
                            row.try_get::<_, bool>(idx)
                                .map(|v| serde_json::json!(v))
                                .unwrap_or(Value::Null)
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
}
