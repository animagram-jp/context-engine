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
    /// 1レコード取得
    fn fetch_one(
        &self,
        connection: &Value,
        table: &str,
        where_clause: Option<&str>,
    ) -> Option<HashMap<String, Value>>;

    /// 複数レコード取得
    fn fetch_all(
        &self,
        connection: &Value,
        table: &str,
        where_clause: Option<&str>,
    ) -> Option<Vec<HashMap<String, Value>>>;

    /// クエリ実行
    fn execute(
        &self,
        connection: &Value,
        query: &str,
        params: &[Value],
    ) -> Result<u64, String>;
}

/// KVSクライアント
/// Redis等のKVS操作
pub trait KVSClient: Send + Sync {
    /// キーから値を取得
    fn get(&self, key: &str) -> Option<Value>;

    /// キーに値を設定
    fn set(&mut self, key: &str, value: Value, ttl: Option<u64>) -> bool;

    /// キーを削除
    fn delete(&mut self, key: &str) -> bool;
}

/// ENVクライアント
/// 環境変数取得
pub trait ENVClient: Send + Sync {
    /// 環境変数を取得
    fn get(&self, key: &str) -> Option<String>;
}

/// APIクライアント
/// 外部API呼び出し
pub trait APIClient: Send + Sync {
    /// GETリクエスト
    fn get(&self, url: &str, headers: Option<&HashMap<String, String>>) -> Result<Value, String>;

    /// POSTリクエスト
    fn post(
        &self,
        url: &str,
        body: &Value,
        headers: Option<&HashMap<String, String>>,
    ) -> Result<Value, String>;
}
