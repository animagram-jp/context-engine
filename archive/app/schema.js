const { execSync } = require('child_process');

/**
 * engineコマンドを実行してYML解析結果を取得
 * @param {string} command - コマンド名 (manifest, connection, cache, database)
 * @param {string} keyPath - ドット区切りのキーパス (例: "common", "user")
 * @returns {any} - 取得した値
 */
function engine(command, keyPath = '') {
  try {
    const result = execSync(`/compiled/state-engine ${command} "${keyPath}"`, {
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'pipe']
    });

    return JSON.parse(result.trim());
  } catch (error) {
    throw new Error(`Engine error [${command}]: ${error.message}`);
  }
}

/**
 * manifest YMLファイルから値を取得
 */
function manifest(keyPath = '') {
  return engine('manifest', keyPath);
}

/**
 * connection YMLファイルから値を取得
 */
function connection(keyPath = '') {
  return engine('connection', keyPath);
}

/**
 * cache YMLファイルから値を取得
 */
function cache(keyPath = '') {
  return engine('cache', keyPath);
}

/**
 * database YMLファイルから値を取得
 */
function database(keyPath = '') {
  return engine('database', keyPath);
}

module.exports = { engine, manifest, connection, cache, database };