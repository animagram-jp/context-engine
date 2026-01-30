-- tenant DB users シードデータ
-- このSQLはtenant DBで実行される想定
INSERT INTO users (sso_user_id, org_id, email, password) VALUES
(1001, 1, 'user1@org1.example.com', '$2a$10$dummyhash'),
(1002, 1, 'user2@org1.example.com', '$2a$10$dummyhash'),
(2001, 2, 'user1@org2.example.com', '$2a$10$dummyhash')
ON CONFLICT (sso_user_id) DO NOTHING;
