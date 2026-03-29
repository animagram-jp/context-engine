/// DbClient implementation using PostgreSQL
///
/// Implements the DbClient Required Port.

use state_engine::Value;
use state_engine::ports::required::DbClient;
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
        let fields = match config {
            Value::Mapping(f) => f,
            _ => return Err("connection must be a mapping".to_string()),
        };

        let tag = fields.iter()
            .find(|(k, _)| k == b"tag")
            .and_then(|(_, v)| match v { Value::Scalar(b) => std::str::from_utf8(b).ok(), _ => None })
            .ok_or_else(|| "Missing 'tag' field in connection config".to_string())?;

        if tag == "common" {
            Ok(format!("connection.{}", tag))
        } else if tag == "tenant" {
            let id = fields.iter()
                .find(|(k, _)| k == b"id")
                .and_then(|(_, v)| match v { Value::Scalar(b) => std::str::from_utf8(b).ok(), _ => None })
                .ok_or_else(|| "Missing 'id' field for tenant connection".to_string())?;
            Ok(format!("connection.{}{}", tag, id))
        } else {
            Err(format!("Unsupported tag: {}", tag))
        }
    }

    async fn connect_from_config(config: &Value) -> Result<tokio_postgres::Client, String> {
        let fields = match config {
            Value::Mapping(f) => f,
            _ => return Err("connection must be a mapping".to_string()),
        };

        let scalar = |key: &[u8]| -> Option<&str> {
            fields.iter()
                .find(|(k, _)| k.as_slice() == key)
                .and_then(|(_, v)| match v { Value::Scalar(b) => std::str::from_utf8(b).ok(), _ => None })
        };

        let host     = scalar(b"host").ok_or("Missing host")?;
        let port_str = scalar(b"port").unwrap_or("5432");
        let port     = port_str.parse::<u16>().unwrap_or(5432);
        let database = scalar(b"database").ok_or("Missing database")?;
        let username = scalar(b"username").ok_or("Missing username")?;
        let password = scalar(b"password").ok_or("Missing password")?;

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
        columns: &[(Vec<u8>, Vec<u8>)],
        where_clause: Option<&[u8]>,
    ) -> Option<Vec<Value>> {
        let runtime = tokio::runtime::Runtime::new().ok()?;

        runtime.block_on(async {
            let conn_name = Self::get_connection_name(connection).ok()?;
            let mut pool_lock = self.pool.lock().unwrap();

            if !pool_lock.contains_key(&conn_name) {
                let client = Self::connect_from_config(connection).await.ok()?;
                pool_lock.insert(conn_name.clone(), client);
            }

            let client = pool_lock.get(&conn_name)?;

            let col_names: Vec<&str> = columns.iter()
                .filter_map(|(k, _)| std::str::from_utf8(k).ok())
                .collect();
            let column_list = if col_names.is_empty() { "*".to_string() } else { col_names.join(", ") };

            let where_str = where_clause.and_then(|b| std::str::from_utf8(b).ok());
            let query = if let Some(wc) = where_str {
                format!("SELECT {} FROM {} WHERE {}", column_list, table, wc)
            } else {
                format!("SELECT {} FROM {}", column_list, table)
            };

            let rows = client.query(&query, &[]).await.ok()?;

            let mut results = Vec::new();
            for row in rows {
                let mut fields = Vec::new();
                for (idx, column) in row.columns().iter().enumerate() {
                    let val: Value = match column.type_() {
                        &tokio_postgres::types::Type::INT4 => {
                            row.try_get::<_, i32>(idx)
                                .map(|v| Value::Scalar(v.to_string().into_bytes()))
                                .unwrap_or(Value::Null)
                        }
                        &tokio_postgres::types::Type::INT8 => {
                            row.try_get::<_, i64>(idx)
                                .map(|v| Value::Scalar(v.to_string().into_bytes()))
                                .unwrap_or(Value::Null)
                        }
                        &tokio_postgres::types::Type::TEXT | &tokio_postgres::types::Type::VARCHAR => {
                            row.try_get::<_, String>(idx)
                                .map(|v| Value::Scalar(v.into_bytes()))
                                .unwrap_or(Value::Null)
                        }
                        &tokio_postgres::types::Type::BOOL => {
                            row.try_get::<_, bool>(idx)
                                .map(|v| Value::Scalar(if v { b"true".to_vec() } else { b"false".to_vec() }))
                                .unwrap_or(Value::Null)
                        }
                        _ => Value::Null,
                    };
                    fields.push((column.name().as_bytes().to_vec(), val));
                }
                results.push(Value::Mapping(fields));
            }

            Some(results)
        })
    }

    fn set(
        &self,
        _connection: &Value,
        _table: &str,
        _columns: &[(Vec<u8>, Vec<u8>)],
        _where_clause: Option<&[u8]>,
    ) -> bool { false }

    fn delete(
        &self,
        _connection: &Value,
        _table: &str,
        _where_clause: Option<&[u8]>,
    ) -> bool { false }
}
