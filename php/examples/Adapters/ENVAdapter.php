<?php

namespace Adapters;

use Animagram\StateEngine\ENVClient;

class ENVAdapter implements ENVClient
{
    public function get(string $key): ?string
    {
        $value = getenv($key);

        return ($value === false || $value === '') ? null : $value;
    }
}
