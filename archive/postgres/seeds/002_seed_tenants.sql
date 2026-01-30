-- tenants シードデータ
INSERT INTO tenants (id, org_id, db_host, db_port, db_username, db_password) VALUES
(1, 1, 'postgres-tenant-1', 5432, 'postgres', 'root')
ON CONFLICT (id) DO NOTHING;
