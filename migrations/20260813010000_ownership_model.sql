-- Phase: organization ownership model.
--
-- Moves EcoQuest from platform-role + membership authorization to single-owner
-- organization authorization:
--   1. user_role: PLAYER -> USER; ORGANIZATION_MEMBER is retired (never assigned).
--   2. organizations.owner_id points at the user who owns the organization.
--   3. organization_members is dropped; ownership carries the authorization.
--   4. events gain cancelled_by and cancellation_reason for admin moderation.
--   5. users.level is recomputed on the same 30-level curve the game uses.

-- --- Roles ------------------------------------------------------------
-- Rename PLAYER to USER. Existing ORGANIZATION_MEMBER accounts become USER;
-- ownership of their organizations is preserved via owner_id (see below).
ALTER TYPE user_role RENAME VALUE 'PLAYER' TO 'USER';
UPDATE users SET role = 'USER' WHERE role = 'ORGANIZATION_MEMBER';
-- The enum value ORGANIZATION_MEMBER remains defined but is never assigned.
ALTER TABLE users ALTER COLUMN role SET DEFAULT 'USER';

-- --- Organization ownership ------------------------------------------
ALTER TABLE organizations ADD COLUMN owner_id UUID REFERENCES users (id) ON DELETE SET NULL;

-- Backfill: existing OWNER memberships become owner_id.
UPDATE organizations o
SET owner_id = m.user_id
FROM organization_members m
WHERE m.organization_id = o.id
  AND m.member_role = 'OWNER'
  AND o.owner_id IS NULL;

CREATE INDEX organizations_owner_id_idx ON organizations (owner_id);

-- Membership is replaced by ownership for the MVP.
DROP TABLE organization_members;

-- --- Admin moderation of events ---------------------------------------
ALTER TABLE events ADD COLUMN cancelled_by UUID REFERENCES users (id) ON DELETE SET NULL;
ALTER TABLE events ADD COLUMN cancellation_reason TEXT;
