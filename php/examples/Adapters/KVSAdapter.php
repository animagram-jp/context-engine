<?php

namespace Adapters;

use Animagram\StateEngine\KVSClient;
use Redis;

class KVSAdapter implements KVSClient
{
    private Redis $redis;

    public function __construct()
    {
        $this->redis = new Redis();
        $this->redis->connect(
            getenv('REDIS_HOST') ?: '127.0.0.1',
            (int)(getenv('REDIS_PORT') ?: 6379)
        );

        $db = getenv('REDIS_DATABASE');
        if ($db !== false && $db !== '') {
            $this->redis->select((int)$db);
        }

        $password = getenv('REDIS_PASSWORD');
        if ($password !== false && $password !== '') {
            $this->redis->auth($password);
        }
    }

    public function get(string $key): ?string
    {
        $value = $this->redis->get($key);
        return $value === false ? null : $value;
    }

    public function set(string $key, string $value, ?int $ttl = null): bool
    {
        if ($ttl !== null) {
            return $this->redis->setex($key, $ttl, $value);
        }
        return $this->redis->set($key, $value);
    }

    public function delete(string $key): bool
    {
        return $this->redis->del($key) > 0;
    }
}
