<?php

namespace App;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Animagram\StateEngine\State;
use Animagram\StateEngine\Manifest;

class CallbackHandler
{
    public function __construct(
        private State $state,
        private Manifest $manifest,
    ) {}

    public function __invoke(Request $request, Response $response): Response
    {
        $cookies = $request->getCookieParams();
        $accessToken = $cookies['access_token'] ?? null;

        if (!$accessToken) {
            return $response->withStatus(401);
        }

        $refreshToken = $cookies['refresh_token'] ?? null;
        $remember = $refreshToken !== null;

        $userInfo = $this->extractUserInfo($accessToken);
        $user_key = $userInfo['user_key'];
        $orgId    = $userInfo['org_id'];

        $auth = new Auth($this->state, $this->manifest);
        $result = $auth->attempt(['user_key' => $user_key, 'org_id' => $orgId], $remember);

        if (!$result) {
            $payload = json_encode(['message' => '該当のsso user idが見つかりません']);
            $response->getBody()->write($payload);
            return $response->withStatus(401)->withHeader('Content-Type', 'application/json');
        }

        $location = $auth->isReservedOrg() ? '/maintainer/dashboard' : '/dashboard';
        return $response->withStatus(302)->withHeader('Location', $location);
    }

    private function extractUserInfo(string $accessToken): array
    {
        // JWT decode is app-specific; replace with your SSO JWT library
        throw new \RuntimeException('extractUserInfo() not implemented — wire your SSO JWT library here');
    }
}
