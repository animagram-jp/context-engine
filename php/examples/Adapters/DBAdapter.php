<?php

namespace Adapters;

use Animagram\StateEngine\DBClient;
use PDO;
use PDOException;

class DBAdapter implements DBClient
{
    /** @var array<string, PDO> */
    private array $connections = [];

    public function fetch(mixed $connection, string $table, array $columns, ?string $where): array
    {
        $pdo = $this->getConnection($connection);

        $columnList = empty($columns) ? '*' : implode(', ', array_map(fn($col) => "`{$col}`", $columns));

        $sql = "SELECT {$columnList} FROM `{$table}`";
        if ($where !== null) {
            $sql .= " WHERE {$where}";
        }

        $stmt = $pdo->query($sql);
        return $stmt->fetchAll(PDO::FETCH_ASSOC) ?: [];
    }

    private function getConnection(mixed $connection): PDO
    {
        if (is_array($connection)) {
            $tag = $connection['tag'] ?? throw new \RuntimeException("Missing 'tag' in connection config");

            $connectionName = $tag === 'tenant'
                ? 'tenant' . ($connection['id'] ?? throw new \RuntimeException("Missing 'id' for tenant connection"))
                : $tag;

            if (!isset($this->connections[$connectionName])) {
                $this->connections[$connectionName] = $this->createPDO($connection);
            }

            return $this->connections[$connectionName];
        }

        throw new \InvalidArgumentException("Invalid connection type: expected array");
    }

    private function createPDO(array $config): PDO
    {
        $driver   = $config['driver']   ?? throw new \RuntimeException("Missing 'driver' in connection config");
        $host     = $config['host']     ?? throw new \RuntimeException("Missing 'host' in connection config");
        $port     = $config['port']     ?? throw new \RuntimeException("Missing 'port' in connection config");
        $database = $config['database'] ?? throw new \RuntimeException("Missing 'database' in connection config");
        $username = $config['username'] ?? throw new \RuntimeException("Missing 'username' in connection config");
        $password = $config['password'] ?? throw new \RuntimeException("Missing 'password' in connection config");

        $dsn = "{$driver}:host={$host};port={$port};dbname={$database};charset=utf8mb4";

        try {
            return new PDO($dsn, $username, $password, [
                PDO::ATTR_ERRMODE            => PDO::ERRMODE_EXCEPTION,
                PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC,
                PDO::ATTR_EMULATE_PREPARES   => false,
            ]);
        } catch (PDOException $e) {
            throw new \RuntimeException("Failed to connect to database: {$e->getMessage()}", 0, $e);
        }
    }
}
