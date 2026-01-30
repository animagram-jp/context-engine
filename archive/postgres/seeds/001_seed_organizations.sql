-- organizations シードデータ
INSERT INTO organizations (org_id, org_name) VALUES
(1, 'Organization Alpha'),
(2, 'Organization Beta'),
(999, 'Development Org')
ON CONFLICT (org_id) DO NOTHING;
