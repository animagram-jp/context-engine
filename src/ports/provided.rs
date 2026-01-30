// Provided Ports - ライブラリが提供するインターフェース
use serde_json::Value;
use std::collections::HashMap;

/// YAMLマニフェストファイル読み込み・管理
pub trait Manifest {
    /// キーからデータを取得（メタデータを除く）
    /// 形式: "filename.path.to.key"
    fn get(&mut self, key: &str, default: Option<Value>) -> Value;

    /// メタデータを取得
    /// 指定されたキーのパス上のすべての_始まりキーを収集
    fn get_meta(&mut self, key: &str) -> HashMap<String, Value>;

    /// 存在しないキーのリストを取得
    fn get_missing_keys(&self) -> &[String];

    /// 存在しないキーのリストをクリア
    fn clear_missing_keys(&mut self);
}

/// DB接続管理 - テーブル名から接続を自動解決
pub trait DBConnection {
    /// テーブル名から接続名を取得
    fn get(&mut self, table_name: &str) -> Option<String>;

    /// 接続キーをパラメータで解決し、実際の接続名を返す
    fn resolve(&mut self, connection_key: &str, params: &HashMap<String, Value>) -> Option<String>;
}

/// KVStore - app/tenant/user スコープへのアクセス提供
pub trait KVStore {
    /// app スコープへのアクセス
    fn app(&mut self) -> Box<dyn Scope + '_>;

    /// tenant スコープへのアクセス
    fn tenant(&mut self) -> Box<dyn Scope + '_>;

    /// user スコープへのアクセス
    fn user(&mut self) -> Box<dyn Scope + '_>;
}

/// Scope - スコープ内でのKVS操作（get/set/delete）
pub trait Scope {
    /// キーから値を取得（キャッシュミス時は自動ロード）
    fn get(&mut self, key: &str) -> Option<Value>;

    /// キーに値を設定
    fn set(&mut self, key: &str, value: Value, ttl: Option<u64>) -> bool;

    /// キーを削除
    fn delete(&mut self, key: &str) -> bool;
}

/// UserKey - ユーザーコンテキスト管理
pub trait UserKey {
    /// sso_user_id をプロセスメモリに設定
    fn set(&mut self);

    /// 現在のユーザーIDを取得
    fn get(&self) -> Option<String>;
}
