-- Initial database setup for state-engine sample

-- Tenants table
CREATE TABLE IF NOT EXISTS tenants (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    db_host VARCHAR(255),
    db_port INTEGER DEFAULT 5432,
    db_database VARCHAR(255),
    db_username VARCHAR(255),
    db_password VARCHAR(255),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    sso_user_id INTEGER UNIQUE NOT NULL,
    sso_org_id INTEGER,
    tenant_id INTEGER REFERENCES tenants(id),
    name VARCHAR(255),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Sample data
INSERT INTO tenants (id, name, db_host, db_port, db_database, db_username, db_password) VALUES
(1, 'Tenant One', 'postgres', 5432, 'state_engine_dev', 'state_user', 'state_pass'),
(2, 'Tenant Two', 'postgres', 5432, 'state_engine_dev', 'state_user', 'state_pass')
ON CONFLICT DO NOTHING;

-- sso_user_id=1, org_id=100, tenant_id=1
-- sso_user_id=2, org_id=200, tenant_id=2
INSERT INTO users (sso_user_id, sso_org_id, tenant_id, name) VALUES
(1, 100, 1, 'John Doe'),
(2, 200, 2, 'Jane Smith')
ON CONFLICT DO NOTHING;

CREATE INDEX IF NOT EXISTS idx_users_sso_user_id ON users(sso_user_id);
