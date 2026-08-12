-- Run only after migrations, through scripts/demo.ps1. Fixed UUIDs make screenshots repeatable.
-- Password hashes are intentionally not stored here. Demo accounts are created through API first.
BEGIN;
INSERT INTO organizations (id, name, organization_type, location, description, verification_status, reviewed_by, reviewed_at)
SELECT '00000000-0000-0000-0000-000000000010', 'Butuan Environmental Organization', 'NON_PROFIT', 'Butuan City', 'Competition demonstration organization', 'APPROVED', u.id, now()
FROM users u WHERE lower(u.email) = 'admin@ecoquest.test'
ON CONFLICT (id) DO UPDATE SET verification_status = 'APPROVED', reviewed_by = EXCLUDED.reviewed_by, reviewed_at = EXCLUDED.reviewed_at;
INSERT INTO organization_members (organization_id, user_id, member_role)
SELECT '00000000-0000-0000-0000-000000000010', u.id, 'OWNER' FROM users u WHERE lower(u.email) = 'organizer@ecoquest.test'
ON CONFLICT DO NOTHING;
UPDATE users SET role = 'ADMIN' WHERE lower(email) = 'admin@ecoquest.test';
UPDATE users SET role = 'ORGANIZATION_MEMBER' WHERE lower(email) = 'organizer@ecoquest.test';
COMMIT;
