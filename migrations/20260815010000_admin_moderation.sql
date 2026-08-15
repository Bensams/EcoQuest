-- Administrator management: moderation reasons, extra event states, flags,
-- password-reset tokens, and audit lookup indexes. Soft states only.

ALTER TYPE verification_status ADD VALUE IF NOT EXISTS 'INACTIVE';
ALTER TYPE event_status ADD VALUE IF NOT EXISTS 'SUSPENDED';
ALTER TYPE event_status ADD VALUE IF NOT EXISTS 'ARCHIVED';

ALTER TABLE organizations
    ADD COLUMN IF NOT EXISTS review_reason TEXT,
    ADD COLUMN IF NOT EXISTS supporting_documents JSONB NOT NULL DEFAULT '[]'::jsonb;

ALTER TABLE events
    ADD COLUMN IF NOT EXISTS previous_status event_status,
    ADD COLUMN IF NOT EXISTS moderation_reason TEXT,
    ADD COLUMN IF NOT EXISTS moderated_by UUID REFERENCES users (id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS moderated_at TIMESTAMPTZ;

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS status_reason TEXT;

ALTER TABLE participations
    ADD COLUMN IF NOT EXISTS flagged BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS flag_reason TEXT,
    ADD COLUMN IF NOT EXISTS flagged_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS flagged_by UUID REFERENCES users (id) ON DELETE SET NULL;

CREATE TABLE IF NOT EXISTS password_reset_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash  BYTEA       NOT NULL UNIQUE,
    created_by  UUID        REFERENCES users (id) ON DELETE SET NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    used_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS password_reset_tokens_user_id_idx
    ON password_reset_tokens (user_id);
CREATE INDEX IF NOT EXISTS audit_logs_entity_idx
    ON audit_logs (entity_type, entity_id, created_at DESC);
CREATE INDEX IF NOT EXISTS participations_flagged_idx
    ON participations (event_id)
    WHERE flagged;
