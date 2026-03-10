<?php

namespace App\Database;

use Illuminate\Database\DatabaseManager;
use Animagram\StateEngine\State;
use Animagram\StateEngine\Manifest;

class CustomDatabaseManager extends DatabaseManager
{
  /**
   *
   * @var bool
   */
  protected $inTransactionMode = false;

  /**
   *
   * @var array<int, array<string>>
   */
  protected $transactionConnections = [];

  /**
   *
   * @var int
   */
  protected $transactionLevel = 0;

  /**
   *
   * @param  string  $table
   * @param  string|null  $as
   * @return \Illuminate\Database\Query\Builder
   */
  public function table($table, $as = null)
  {
    $connectionName = $this->resolveConnectionFromTable($table);
    return $this->connection($connectionName)->table($table, $as);
  }

  /**
   *
   * @param  string  $table
   * @return string|null
   */
  protected function resolveConnectionFromTable(string $table): string
  {
    $manifest = app(Manifest::class);
    $tableConfig = $manifest->get("tables.{$table}");

    if (empty($tableConfig) || !isset($tableConfig['connection'])) {
      throw new \RuntimeException("CustomDatabaseManager: No connection config for table '{$table}' in tables.yml");
    }

    $baseConnectionName = $tableConfig['connection'];

    $connectionConfig = app(State::class)->get("connection.{$baseConnectionName}");

    if (!is_array($connectionConfig)) {
      throw new \RuntimeException("CustomDatabaseManager: Failed to get connection config for '{$baseConnectionName}'. Result: " . var_export($connectionConfig, true));
    }

    $tag = $connectionConfig['tag'] ?? throw new \RuntimeException("CustomDatabaseManager: Missing 'tag' in connection config for '{$baseConnectionName}'");

    if ($tag === 'tenant') {
      $tenantId = $connectionConfig['id'] ?? throw new \RuntimeException("CustomDatabaseManager: Missing 'id' for tenant connection");
      $connectionName = 'tenant' . $tenantId;
    } else {
      $connectionName = $tag;
    }

    return $connectionName;
  }

  /**
   *
   * @param  string|null  $name
   * @return \Illuminate\Database\Connection
   */
  public function connection($name = null)
  {
    if ($name !== null) {
      $connection = parent::connection($name);
      $this->trackTransactionConnection($name, $connection);
      return $connection;
    }

    $connection = parent::connection($name);
    $this->trackTransactionConnection($this->getDefaultConnection(), $connection);
    return $connection;
  }

  /**
   *
   * @param string $connectionName
   * @param \Illuminate\Database\Connection $connection
   * @return void
   */
  protected function trackTransactionConnection($connectionName, $connection)
  {
    if ($this->inTransactionMode && $this->transactionLevel > 0) {
      $currentLevelConnections = $this->transactionConnections[$this->transactionLevel] ?? [];

      if (!in_array($connectionName, $currentLevelConnections)) {
        $connection->beginTransaction();
        $this->transactionConnections[$this->transactionLevel][] = $connectionName;
      }
    }
  }

  /**
   *
   * @param  string|null  $name
   * @return void
   */
  public function beginTransaction($name = null)
  {
    if ($name !== null) {
      parent::connection($name)->beginTransaction();
    } else {
      $this->inTransactionMode = true;
      $this->transactionLevel++;

      if (!isset($this->transactionConnections[$this->transactionLevel])) {
        $this->transactionConnections[$this->transactionLevel] = [];
      }
    }
  }

  /**
   *
   * @param  string|null  $name
   * @return void
   */
  public function commit($name = null)
  {
    if ($name !== null) {
      parent::connection($name)->commit();
    } else {
      \Log::info('=== CustomDatabaseManager::commit() ===', [
        'transactionLevel' => $this->transactionLevel,
        'transactionConnections' => $this->transactionConnections,
        'inTransactionMode' => $this->inTransactionMode,
      ]);

      if (isset($this->transactionConnections[$this->transactionLevel])) {
        foreach ($this->transactionConnections[$this->transactionLevel] as $connectionName) {
          \Log::info('=== Committing connection ===', [
            'level' => $this->transactionLevel,
            'connection' => $connectionName,
            'transaction_level_before' => parent::connection($connectionName)->transactionLevel(),
          ]);
          parent::connection($connectionName)->commit();
          \Log::info('=== Committed connection ===', [
            'connection' => $connectionName,
            'transaction_level_after' => parent::connection($connectionName)->transactionLevel(),
          ]);
        }

        unset($this->transactionConnections[$this->transactionLevel]);
      }

      $this->transactionLevel--;

      if ($this->transactionLevel === 0) {
        $this->inTransactionMode = false;
        $this->transactionConnections = [];
      }
    }
  }

  /**
   *
   * @param  string|null  $name
   * @param  int|null  $toLevel
   * @return void
   */
  public function rollBack($name = null, $toLevel = null)
  {
    if ($name !== null) {
      parent::connection($name)->rollBack($toLevel);
    } else {
      if (isset($this->transactionConnections[$this->transactionLevel])) {
        foreach ($this->transactionConnections[$this->transactionLevel] as $connectionName) {
          parent::connection($connectionName)->rollBack($toLevel);
        }

        unset($this->transactionConnections[$this->transactionLevel]);
      }

      $this->transactionLevel--;

      if ($this->transactionLevel === 0) {
        $this->inTransactionMode = false;
        $this->transactionConnections = [];
      }
    }
  }
}
