-- Initial database setup for state-engine sample

-- Tenants table
CREATE TABLE IF NOT EXISTS tenants (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    display_name VARCHAR(255),
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
    sso_user_id VARCHAR(255) UNIQUE NOT NULL,
    sso_org_id INTEGER,
    tenant_id INTEGER REFERENCES tenants(id),
    name VARCHAR(255),
    email VARCHAR(255),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Sample data
INSERT INTO tenants (id, name, display_name, db_host, db_port, db_database, db_username, db_password) VALUES
(1, 'tenant_1', 'Tenant One', 'localhost', 5432, 'tenant_1_db', 'tenant_user', 'tenant_pass'),
(2, 'tenant_2', 'Tenant Two', 'localhost', 5432, 'tenant_2_db', 'tenant_user', 'tenant_pass')
ON CONFLICT DO NOTHING;

INSERT INTO users (sso_user_id, sso_org_id, tenant_id, name, email) VALUES
('user001', 100, 1, 'John Doe', 'john@example.com'),
('user002', 100, 1, 'Jane Smith', 'jane@example.com'),
('user003', 200, 2, 'Bob Johnson', 'bob@example.com')
ON CONFLICT DO NOTHING;

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_users_sso_user_id ON users(sso_user_id);
CREATE INDEX IF NOT EXISTS idx_users_tenant_id ON users(tenant_id);
