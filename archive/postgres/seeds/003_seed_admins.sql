-- admins シードデータ
INSERT INTO admins (username, email, password) VALUES
('admin', 'admin@example.com', '$2a$10$dummyhash'),
('devadmin', 'dev@example.com', '$2a$10$dummyhash')
ON CONFLICT (username) DO NOTHING;
