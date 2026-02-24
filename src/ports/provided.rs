// Provided Ports - ライブラリが提供するインターフェース
use serde_json::Value;
use std::collections::HashMap;

/// Manifest 操作のエラー型
#[derive(Debug, PartialEq)]
pub enum ManifestError {
    /// ファイルが見つからない
    FileNotFound(String),
    /// .yml と .yaml の両方が存在する（曖昧）
    AmbiguousFile(String),
    /// ファイルの読み込みに失敗
    ReadError(String),
    /// YAML のパースに失敗
    ParseError(String),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::FileNotFound(msg)  => write!(f, "FileNotFound: {}", msg),
            ManifestError::AmbiguousFile(msg) => write!(f, "AmbiguousFile: {}", msg),
            ManifestError::ReadError(msg)     => write!(f, "ReadError: {}", msg),
            ManifestError::ParseError(msg)    => write!(f, "ParseError: {}", msg),
        }
    }
}

/// State 操作のエラー型
#[derive(Debug, PartialEq)]
pub enum StateError {
    /// マニフェストのロードに失敗
    ManifestLoadFailed(String),
    /// 指定キーがマニフェストに存在しない
    KeyNotFound(String),
    /// 再帰呼び出しの上限に達した
    RecursionLimitExceeded,
    /// ストアへの書き込み／削除に失敗
    StoreFailed(String),
    /// ロードに失敗
    LoadFailed(String),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateError::ManifestLoadFailed(msg)  => write!(f, "ManifestLoadFailed: {}", msg),
            StateError::KeyNotFound(msg)          => write!(f, "KeyNotFound: {}", msg),
            StateError::RecursionLimitExceeded    => write!(f, "RecursionLimitExceeded"),
            StateError::StoreFailed(msg)          => write!(f, "StoreFailed: {}", msg),
            StateError::LoadFailed(msg)           => write!(f, "LoadFailed: {}", msg),
        }
    }
}

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

    /// キーから値のみを取得（メタデータと null を除く）
    ///
    /// get() との違い:
    /// - メタデータ（_で始まるキー）を除外
    /// - null 値のフィールドを除外
    ///
    /// 内部利用想定（State から呼ばれる）
    fn get_value(&mut self, key: &str) -> Value;

    /// YAMLファイルをロード（未ロードの場合のみ）
    ///
    /// # Errors
    /// * `ManifestError::FileNotFound` - .yml/.yaml どちらも存在しない
    /// * `ManifestError::AmbiguousFile` - .yml と .yaml の両方が存在する
    /// * `ManifestError::ReadError` - ファイル読み込み失敗
    /// * `ManifestError::ParseError` - YAML パース失敗
    fn load_file(&mut self, file: &str) -> Result<(), ManifestError>;
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
    /// * `Ok(Some(Value))` - 値が存在する場合
    /// * `Ok(None)` - 値が存在せず、ロードも失敗した場合
    /// * `Err(StateError)` - キー不正・再帰超過などエラーの場合
    fn get(&mut self, key: &str) -> Result<Option<Value>, StateError>;

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
    /// * `Ok(true)` - 保存成功
    /// * `Ok(false)` - _store 設定がなく保存できない（エラーではない）
    /// * `Err(StateError)` - キー不正・マニフェスト未ロードなどエラーの場合
    fn set(&mut self, key: &str, value: Value, ttl: Option<u64>) -> Result<bool, StateError>;

    /// 状態を削除
    ///
    /// _store から該当の {key:value} レコードを削除する。
    ///
    /// # Arguments
    /// * `key` - manifest key ("filename.node.field")
    ///
    /// # Returns
    /// * `Ok(true)` - 削除成功
    /// * `Ok(false)` - キーが存在しない
    /// * `Err(StateError)` - キー不正・マニフェスト未ロードなどエラーの場合
    fn delete(&mut self, key: &str) -> Result<bool, StateError>;

    /// キーの存在確認（自動ロードなし）
    ///
    /// get()と異なり、自動ロードをトリガーしない。
    /// キャッシュとストアのみをチェックする。
    ///
    /// # Arguments
    /// * `key` - manifest key ("filename.node.field")
    ///
    /// # Returns
    /// * `Ok(true)` - キーが存在する（キャッシュまたはストアに存在）
    /// * `Ok(false)` - キーが存在しない
    /// * `Err(StateError)` - キー不正・マニフェスト未ロードなどエラーの場合
    fn exists(&mut self, key: &str) -> Result<bool, StateError>;
}
