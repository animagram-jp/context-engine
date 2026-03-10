<?php

namespace App;

use Animagram\StateEngine\State;
use Animagram\StateEngine\Manifest;

class Auth
{
    private const RESERVED_ORG_ID = 999999;

    private ?int $userId = null;

    public function __construct(
        private State $state,
        private Manifest $manifest,
    ) {}

    public function attempt(array $credentials, bool $remember = false): bool
    {
        $user_key = $credentials['user_key'] ?? null;
        $orgId    = $credentials['org_id']   ?? null;

        $manifestTtl = (int)$this->manifest->getMeta('cache.user')['_store']['ttl'];

        $this->state->set('request-attributes-user-key', $user_key);
        $this->state->set('cache.user.org_id', $orgId, $manifestTtl);

        $userId = $this->state->get('cache.user.id');

        if ($userId === null) {
            $this->state->delete('cache.user');
            return false;
        }

        $ttl = $remember ? null : $manifestTtl;
        $this->state->set('cache.user.id', $userId, $ttl);
        $this->userId = (int)$userId;

        return true;
    }

    public function isReservedOrg(): bool
    {
        return $this->state->get('cache.user.org_id') === self::RESERVED_ORG_ID;
    }

    public function id(): ?int
    {
        return $this->userId ?? ($this->state->get('cache.user.id') !== null
            ? (int)$this->state->get('cache.user.id')
            : null);
    }

    public function check(): bool
    {
        return $this->state->get('cache.user.id') !== null;
    }

    public function isMaintainer(): bool
    {
        if (!$this->check()) {
            return false;
        }
        return $this->isReservedOrg() || (bool)$this->state->get('cache.user.is_admin');
    }
}
