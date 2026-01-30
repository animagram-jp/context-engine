-- tenant DB用 users テーブル作成
-- このSQLはtenant DBで実行される想定
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    sso_user_id INTEGER NOT NULL UNIQUE,
    org_id INTEGER NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    password VARCHAR(255),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_sso_user_id ON users(sso_user_id);
CREATE INDEX idx_users_org_id ON users(org_id);
CREATE INDEX idx_users_email ON users(email);
