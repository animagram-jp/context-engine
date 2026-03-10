<?php

use DI\ContainerBuilder;
use Psr\Container\ContainerInterface;
use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Slim\Factory\AppFactory;
use Animagram\StateEngine\Manifest;
use Animagram\StateEngine\State;
use Animagram\StateEngine\Load;
use App\Auth;
use App\AuthMiddleware;
use App\CallbackHandler;
use Adapters\DBAdapter;
use Adapters\ENVAdapter;
use Adapters\InMemoryAdapter;
use Adapters\KVSAdapter;

require __DIR__ . '/vendor/autoload.php';

$builder = new ContainerBuilder();

$builder->addDefinitions([
    InMemoryAdapter::class => function (ContainerInterface $c) {
        return new InMemoryAdapter($c->get(Request::class));
    },

    KVSAdapter::class => function () {
        return new KVSAdapter();
    },

    DBAdapter::class => function () {
        return new DBAdapter();
    },

    ENVAdapter::class => function () {
        return new ENVAdapter();
    },

    Manifest::class => function () {
        $manifestDir = realpath(__DIR__ . '/../manifest');
        return new Manifest($manifestDir);
    },

    Load::class => function (ContainerInterface $c) {
        return (new Load())
            ->withDbClient($c->get(DBAdapter::class))
            ->withKvsClient($c->get(KVSAdapter::class))
            ->withInMemory($c->get(InMemoryAdapter::class))
            ->withEnvClient($c->get(ENVAdapter::class));
    },

    State::class => function (ContainerInterface $c) {
        return new State(
            $c->get(Manifest::class),
            $c->get(Load::class)
        );
    },

    Auth::class => function (ContainerInterface $c) {
        return new Auth(
            $c->get(State::class),
            $c->get(Manifest::class),
        );
    },

    CallbackHandler::class => function (ContainerInterface $c) {
        return new CallbackHandler(
            $c->get(State::class),
            $c->get(Manifest::class),
        );
    },

    AuthMiddleware::class => function (ContainerInterface $c) {
        return new AuthMiddleware($c->get(State::class));
    },
]);

$container = $builder->build();

AppFactory::setContainer($container);
$app = AppFactory::create();

$app->get('/auth/callback', CallbackHandler::class);

$app->get('/process/{id}', function (Request $request, Response $response, array $args) use ($container) {
    $user_id = (int)$args['id'];

    $bridge = new YourName\RustBridge();
    $snake_result = $bridge->do_heavy_logic($user_id);

    $payload = json_encode(['result' => $snake_result, 'id' => $user_id]);
    $response->getBody()->write($payload);

    return $response->withHeader('Content-Type', 'application/json');
});

$app->run();
