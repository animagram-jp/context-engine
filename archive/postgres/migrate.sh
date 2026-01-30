#!/bin/sh
# PostgreSQL migration script

set -e

# 環境変数の読み込み
COMMON_DB=${POSTGRES_DB:-common_db}
COMMON_HOST="localhost"
COMMON_PORT="5432"
COMMON_USER=${POSTGRES_USER:-postgres}
COMMON_PASS=${POSTGRES_PASSWORD:-root}

TENANT1_DB="db_tenant1"
TENANT1_HOST="postgres-tenant-1"
TENANT1_PORT="5432"
TENANT1_USER="postgres"
TENANT1_PASS="root"

echo "=== PostgreSQL Migration Script ==="
echo ""

# Common DBのマイグレーション
echo ">>> Migrating common_db..."
export PGPASSWORD=$COMMON_PASS

for file in /migrations/*.sql; do
  echo "Running $(basename $file)..."
  psql -h $COMMON_HOST -p $COMMON_PORT -U $COMMON_USER -d $COMMON_DB -f $file
done

echo ""
echo ">>> Seeding common_db..."

for file in /seeds/00[1-3]_*.sql; do
  echo "Running $(basename $file)..."
  psql -h $COMMON_HOST -p $COMMON_PORT -U $COMMON_USER -d $COMMON_DB -f $file
done

echo ""
echo ">>> Migrating tenant DBs..."
export PGPASSWORD=$TENANT1_PASS

# Tenant1 DBのマイグレーション
echo "Running 004_create_tenant_users.sql on $TENANT1_DB..."
psql -h $TENANT1_HOST -p $TENANT1_PORT -U $TENANT1_USER -d $TENANT1_DB -f /migrations/004_create_tenant_users.sql

echo ""
echo ">>> Seeding tenant DBs..."
echo "Running 004_seed_tenant_users.sql on $TENANT1_DB..."
psql -h $TENANT1_HOST -p $TENANT1_PORT -U $TENANT1_USER -d $TENANT1_DB -f /seeds/004_seed_tenant_users.sql

echo ""
echo "=== Migration completed ==="
