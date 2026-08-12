-- Phase 1: identity, sessions, organizations and audit trail.

CREATE TYPE user_role AS ENUM ('PLAYER', 'ORGANIZATION_MEMBER', 'ADMIN');
CREATE TYPE user_status AS ENUM ('ACTIVE', 'SUSPENDED', 'DELETED');
CREATE TYPE verification_status AS ENUM ('PENDING', 'APPROVED', 'REJECTED', 'SUSPENDED');

CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username        TEXT        NOT NULL,
    email           TEXT        NOT NULL,
    password_hash   TEXT        NOT NULL,
    role            user_role   NOT NULL DEFAULT 'PLAYER',
    wallet_address  TEXT,
    status          user_status NOT NULL DEFAULT 'ACTIVE',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Case-insensitive uniqueness: users must not register the same identity twice
-- with different casing.
CREATE UNIQUE INDEX users_email_lower_key ON users (lower(email));
CREATE UNIQUE INDEX users_username_lower_key ON users (lower(username));
-- Wallet addresses are optional but must not be shared between accounts.
CREATE UNIQUE INDEX users_wallet_address_key ON users (wallet_address)
    WHERE wallet_address IS NOT NULL;

-- Only the SHA-256 hash of a refresh token is stored, so a database leak cannot
-- be replayed against the API.
CREATE TABLE refresh_tokens (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash   BYTEA       NOT NULL UNIQUE,
    issued_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL,
    revoked_at   TIMESTAMPTZ,
    -- Set when this token is rotated, pointing at its replacement. Lets us
    -- detect reuse of an already-rotated token.
    replaced_by  UUID REFERENCES refresh_tokens (id) ON DELETE SET NULL,
    user_agent   TEXT,
    ip_address   INET
);

CREATE INDEX refresh_tokens_user_id_idx ON refresh_tokens (user_id);
CREATE INDEX refresh_tokens_expires_at_idx ON refresh_tokens (expires_at);

CREATE TABLE organizations (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                TEXT                NOT NULL,
    organization_type   TEXT                NOT NULL,
    location            TEXT                NOT NULL,
    description         TEXT                NOT NULL DEFAULT '',
    verification_status verification_status NOT NULL DEFAULT 'PENDING',
    reviewed_by         UUID REFERENCES users (id) ON DELETE SET NULL,
    reviewed_at         TIMESTAMPTZ,
    created_at          TIMESTAMPTZ         NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ         NOT NULL DEFAULT now(),
    -- A review is only meaningful when both reviewer and timestamp are present.
    CONSTRAINT organizations_review_complete
        CHECK ((reviewed_by IS NULL) = (reviewed_at IS NULL))
);

CREATE UNIQUE INDEX organizations_name_lower_key ON organizations (lower(name));

CREATE TABLE organization_members (
    organization_id UUID        NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    user_id         UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- Membership role inside the organization (OWNER, STAFF, ...); distinct from
    -- the platform-wide users.role.
    member_role     TEXT        NOT NULL DEFAULT 'STAFF',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, user_id)
);

CREATE INDEX organization_members_user_id_idx ON organization_members (user_id);

-- Append-only trail of security relevant account actions.
CREATE TABLE audit_logs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_id    UUID REFERENCES users (id) ON DELETE SET NULL,
    action      TEXT        NOT NULL,
    entity_type TEXT        NOT NULL,
    entity_id   UUID,
    metadata    JSONB       NOT NULL DEFAULT '{}'::jsonb,
    ip_address  INET,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX audit_logs_actor_id_idx ON audit_logs (actor_id);
CREATE INDEX audit_logs_created_at_idx ON audit_logs (created_at DESC);

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER users_set_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER organizations_set_updated_at
    BEFORE UPDATE ON organizations
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
