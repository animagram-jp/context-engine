#!/usr/bin/env bash
set -e

docker compose up -d --build
EXIT_CODE=$(docker wait example-app)
docker logs example-app
docker compose down
exit $EXIT_CODE
