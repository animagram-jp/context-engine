<?php

namespace Adapters;

use Animagram\StateEngine\InMemoryClient;
use Psr\Http\Message\ServerRequestInterface;

class InMemoryAdapter implements InMemoryClient
{
    /** @var array<string, mixed> */
    private array $store = [];

    /** @var array<string, mixed> */
    private array $connectionStore = [];

    public function __construct(private ServerRequestInterface $request) {}

    public function get(string $key): mixed
    {
        if ($key === 'request-attributes-user-key') {
            $attrs = $this->request->getAttributes();
            if (isset($attrs[$key])) {
                return $attrs[$key];
            }

            $headerValue = $this->request->getHeaderLine('request-header-user-key');
            if ($headerValue !== '' && is_numeric($headerValue)) {
                return (int)$headerValue;
            }

            $cookies = $this->request->getCookieParams();
            $cookieValue = $cookies['request-header-user-key'] ?? null;
            if ($cookieValue !== null && is_numeric($cookieValue)) {
                return (int)$cookieValue;
            }

            return null;
        }

        if (str_starts_with($key, 'connection.')) {
            return $this->connectionStore[$key] ?? null;
        }

        return $this->store[$key] ?? null;
    }

    public function set(string $key, mixed $value): void
    {
        if (str_starts_with($key, 'connection.')) {
            $this->connectionStore[$key] = $value;
            return;
        }

        $this->store[$key] = $value;
    }

    public function delete(string $key): bool
    {
        if (str_starts_with($key, 'connection.')) {
            if (array_key_exists($key, $this->connectionStore)) {
                unset($this->connectionStore[$key]);
                return true;
            }
            return false;
        }

        if (array_key_exists($key, $this->store)) {
            unset($this->store[$key]);
            return true;
        }
        return false;
    }
}
