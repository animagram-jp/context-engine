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

/// State - 統一CRUD実装
///
/// manifest の _state/_store/_load に従って状態を管理する。
/// state-engineの唯一の外部向けインターフェース。
pub trait State {
    /// 状態を取得
    ///
    /// 1. _store から値を取得
    /// 2. miss時は _load に従い自動ロード
    /// 3. ロード成功時は _store に保存して返却
    ///
    /// # Arguments
    /// * `key` - manifest key ("filename.node.field")
    ///
    /// # Returns
    /// * `Some(Value)` - 値が存在する場合
    /// * `None` - 値が存在せず、ロードも失敗した場合
    fn get(&mut self, key: &str) -> Option<Value>;

    /// 状態を設定
    ///
    /// _store に従って値を保存する。
    ///
    /// # Arguments
    /// * `key` - manifest key ("filename.node.field")
    /// * `value` - 保存する値
    /// * `ttl` - TTL（秒）。KVS使用時のみ有効。Noneの場合はYAML定義に従う
    ///
    /// # Returns
    /// * `true` - 保存成功
    /// * `false` - 保存失敗
    fn set(&mut self, key: &str, value: Value, ttl: Option<u64>) -> bool;

    /// 状態を削除
    ///
    /// _store から該当の {key:value} レコードを削除する。
    ///
    /// # Arguments
    /// * `key` - manifest key ("filename.node.field")
    ///
    /// # Returns
    /// * `true` - 削除成功
    /// * `false` - 削除失敗またはキーが存在しない
    fn delete(&mut self, key: &str) -> bool;
}
