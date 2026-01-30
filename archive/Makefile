.PHONY: help up down stop restart migrate repl test logs-rust logs-postgres clean rebuild db-schema db-common db-tenant build-rust

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

up: ## Start all containers
	docker compose up -d

down: ## Stop all containers
	docker compose down

stop: ## Stop all containers (alias)
	docker compose stop

restart: ## Restart all containers
	docker compose restart

migrate: ## Run database migrations
	docker compose exec postgres-common sh /migrate.sh

repl: ## Start Node.js REPL
	docker compose exec node node /app/repl.js

test: ## Run manifest test
	docker compose exec node node /app/test.js

build-rust: ## Build Rust binary and copy to /compiled
	docker compose exec rust sh -c "cargo build --release && mkdir -p /compiled && cp -f target/release/state-engine /compiled/state-engine"

logs-rust: ## Show Rust container logs
	docker compose logs -f rust

logs-postgres: ## Show PostgreSQL logs
	docker compose logs -f postgres-common postgres-tenant-1

clean: ## Remove all volumes and containers
	docker compose down -v

rebuild: ## Rebuild and restart all containers
	docker compose down
	docker compose build --no-cache
	docker compose up -d

db-schema: ## Show all databases and tables
	@echo "=== Common DB (common_db) ==="
	@docker compose exec postgres-common psql -U postgres -d common_db -c "\dt"
	@echo ""
	@echo "=== Tenant DB (db_tenant1) ==="
	@docker compose exec postgres-tenant-1 psql -U postgres -d db_tenant1 -c "\dt"

db-common: ## Show common_db tables schema
	@echo "=== organizations table ==="
	@docker compose exec postgres-common psql -U postgres -d common_db -c "\d organizations"
	@echo ""
	@echo "=== tenants table ==="
	@docker compose exec postgres-common psql -U postgres -d common_db -c "\d tenants"
	@echo ""
	@echo "=== admins table ==="
	@docker compose exec postgres-common psql -U postgres -d common_db -c "\d admins"
	@echo ""
	@echo "=== Sample data: organizations ==="
	@docker compose exec postgres-common psql -U postgres -d common_db -c "SELECT * FROM organizations;"
	@echo ""
	@echo "=== Sample data: tenants ==="
	@docker compose exec postgres-common psql -U postgres -d common_db -c "SELECT * FROM tenants;"

db-tenant: ## Show tenant db tables schema
	@echo "=== users table (tenant db) ==="
	@docker compose exec postgres-tenant-1 psql -U postgres -d db_tenant1 -c "\d users"
	@echo ""
	@echo "=== Sample data: users ==="
	@docker compose exec postgres-tenant-1 psql -U postgres -d db_tenant1 -c "SELECT * FROM users;"
