-- tenants テーブル作成
CREATE TABLE IF NOT EXISTS tenants (
    id SERIAL PRIMARY KEY,
    org_id INTEGER NOT NULL,
    db_host VARCHAR(255) NOT NULL,
    db_port INTEGER DEFAULT 5432,
    db_username VARCHAR(255) NOT NULL,
    db_password VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (org_id) REFERENCES organizations(org_id) ON DELETE CASCADE
);

CREATE INDEX idx_tenants_org_id ON tenants(org_id);
CREATE INDEX idx_tenants_id ON tenants(id);
