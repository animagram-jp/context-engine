<?php

namespace App;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Psr\Http\Server\MiddlewareInterface;
use Psr\Http\Server\RequestHandlerInterface;
use Slim\Psr7\Response as SlimResponse;
use Animagram\StateEngine\State;

class AuthMiddleware implements MiddlewareInterface
{
    public function __construct(private State $state) {}

    public function process(Request $request, RequestHandlerInterface $handler): Response
    {
        $userKey = $this->state->get('request-attributes-user-key');

        if ($userKey === null || $this->state->get('cache.user.id') === null) {
            $response = new SlimResponse();
            $response->getBody()->write(json_encode(['message' => 'Unauthorized']));
            return $response->withStatus(401)->withHeader('Content-Type', 'application/json');
        }

        return $handler->handle($request);
    }
}
