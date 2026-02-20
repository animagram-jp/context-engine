// Required Ports - アプリケーションが実装すべきインターフェース
use serde_json::Value;
use std::collections::HashMap;

/// プロセスメモリクライアント
/// 論理キー→物理キーの変換。request/config/headerに分散したデータアクセス
pub trait InMemoryClient: Send + Sync {
    /// プロセスメモリから値を取得
    fn get(&self, key: &str) -> Option<Value>;

    /// プロセスメモリに値を設定
    fn set(&mut self, key: &str, value: Value);

    /// プロセスメモリから値を削除
    fn delete(&mut self, key: &str) -> bool;
}

/// DBクライアント
/// DB接続・クエリ実行（PDO相当）
///
/// # connection 引数について
/// - Value::Object: 接続情報が含まれる Object (例: {host: "...", port: 3306, ...})
/// - Value::String: 接続識別子 (例: "common", "tenant")
///
/// **重要:** DBClient 実装内で State を呼び出してはいけません。
/// String 形式の connection を受け取った場合は、実装側で事前に用意した
/// 接続マップから取得するか、エラーを返してください。
pub trait DBClient: Send + Sync {
    /// レコード取得（単数でも複数でも対応）
    ///
    /// # Arguments
    /// * `connection` - 接続情報 (Object or String)
    /// * `table` - テーブル名
    /// * `columns` - SELECT するカラム名の配列（map から自動抽出）
    /// * `where_clause` - WHERE 条件（省略可）
    ///
    /// # Returns
    /// * `Some(Vec<HashMap>)` - 取得成功（0件以上）
    /// * `None` - エラー
    ///
    /// # SQL 生成例
    /// ```sql
    /// SELECT db_host, db_port, db_database FROM tenants WHERE id=1
    /// ```
    fn fetch(
        &self,
        connection: &Value,
        table: &str,
        columns: &[&str],
        where_clause: Option<&str>,
    ) -> Option<Vec<HashMap<String, Value>>>;
}

/// KVSクライアント
/// Redis等のKVS操作
///
/// KVSは文字列のみを扱う（primitive型）。
/// serialize/deserializeはState層で行う。
pub trait KVSClient: Send + Sync {
    /// キーから値を取得
    ///
    /// # Returns
    /// * `Some(String)` - 取得成功
    /// * `None` - キーが存在しない
    fn get(&self, key: &str) -> Option<String>;

    /// キーに値を設定
    ///
    /// # Arguments
    /// * `key` - キー
    /// * `value` - 値（文字列）
    /// * `ttl` - TTL（秒）
    fn set(&mut self, key: &str, value: String, ttl: Option<u64>) -> bool;

    /// キーを削除
    fn delete(&mut self, key: &str) -> bool;
}

/// Envクライアント
/// 環境変数取得
pub trait EnvClient: Send + Sync {
    /// 環境変数を取得
    fn get(&self, key: &str) -> Option<String>;
}

// future function: API Client
// /// 外部API呼び出し
// pub trait APIClient: Send + Sync {
//     /// GETリクエスト
//     fn get(&self, url: &str, headers: Option<&HashMap<String, String>>) -> Result<Value, String>;

//     /// POSTリクエスト
//     fn post(
//         &self,
//         url: &str,
//         body: &Value,
//         headers: Option<&HashMap<String, String>>,
//     ) -> Result<Value, String>;
// }
