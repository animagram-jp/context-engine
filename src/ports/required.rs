// Required Ports - アプリケーションが実装すべきインターフェース
use serde_json::Value;
use std::collections::HashMap;

/// プロセスメモリクライアント
/// 論理キー→物理キーの変換。request/config/headerに分散したデータアクセス
pub trait ProcessMemoryClient: Send + Sync {
    /// プロセスメモリから値を取得
    fn get(&self, key: &str) -> Option<Value>;

    /// プロセスメモリに値を設定
    fn set(&mut self, key: &str, value: Value);

    /// プロセスメモリから値を削除
    fn delete(&mut self, key: &str) -> bool;
}

/// DB接続設定
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
}

/// DB接続設定コンバーター
/// フレームワーク形式 ⇔ ConnectionConfig（標準形式）の相互変換
pub trait DBConnectionConfigConverter: Send + Sync {
    /// フレームワーク形式をConnectionConfigに変換
    fn to_config(&self, framework_config: &HashMap<String, Value>) -> Option<ConnectionConfig>;

    /// ConnectionConfigをフレームワーク形式に変換
    fn from_config(&self, config: &ConnectionConfig) -> HashMap<String, Value>;
}

/// DBクライアント
/// DB接続・クエリ実行（PDO相当）
pub trait DBClient: Send + Sync {
    /// 1レコード取得
    fn fetch_one(
        &self,
        config: &ConnectionConfig,
        table: &str,
        where_clause: Option<&str>,
    ) -> Option<HashMap<String, Value>>;

    /// 複数レコード取得
    fn fetch_all(
        &self,
        config: &ConnectionConfig,
        table: &str,
        where_clause: Option<&str>,
    ) -> Option<Vec<HashMap<String, Value>>>;

    /// クエリ実行
    fn execute(
        &self,
        config: &ConnectionConfig,
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

    /// キーの存在確認
    fn exists(&self, key: &str) -> bool;
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
