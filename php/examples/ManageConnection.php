<?php

namespace App\Traits;

use Animagram\StateEngine\State;
use Animagram\StateEngine\Manifest;

trait ManageConnection
{
  public function getConnectionName()
  {
    if (isset($this->connection)) {
      return $this->connection;
    }

    return $this->resolveConnectionFromTable();
  }

  protected function resolveConnectionFromTable(): string
  {
    $table = $this->getTable();
    $manifest = app(Manifest::class);
    $tableConfig = $manifest->get("tables.{$table}");

    if (empty($tableConfig) || !isset($tableConfig['connection'])) {
      throw new \RuntimeException("ManageConnection: No connection config for table '{$table}' in tables.yml");
    }

    $baseConnectionName = $tableConfig['connection']; // 'tenant' or 'common'

    $connectionConfig = app(State::class)->get("connection.{$baseConnectionName}");

    if (!is_array($connectionConfig)) {
      throw new \RuntimeException("ManageConnection: Failed to get connection config for '{$baseConnectionName}'. Result: " . var_export($connectionConfig, true));
    }

    $tag = $connectionConfig['tag'] ?? throw new \RuntimeException("ManageConnection: Missing 'tag' in connection config for '{$baseConnectionName}'");

    if ($tag === 'tenant') {
      $tenantId = $connectionConfig['id'] ?? throw new \RuntimeException("ManageConnection: Missing 'id' for tenant connection");
      $connectionName = 'tenant' . $tenantId;
    } else {
      $connectionName = $tag;
    }

    return $connectionName;
  }
}
