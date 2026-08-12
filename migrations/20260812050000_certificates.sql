-- Phase 5: immutable certificate identity and revocation audit.
CREATE TYPE certificate_status AS ENUM ('VALID', 'REVOKED');
CREATE SEQUENCE certificate_number_seq START WITH 1;

CREATE TABLE certificates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    certificate_number TEXT NOT NULL UNIQUE DEFAULT ('ECO-' || to_char(current_date, 'YYYY') || '-' || lpad(nextval('certificate_number_seq')::text, 6, '0')),
    participation_id UUID NOT NULL UNIQUE REFERENCES participations(id) ON DELETE RESTRICT,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE RESTRICT,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    volunteer_duration_minutes INTEGER NOT NULL CHECK (volunteer_duration_minutes >= 0),
    verification_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(verification_hash) = 32),
    storage_key TEXT NOT NULL UNIQUE,
    status certificate_status NOT NULL DEFAULT 'VALID',
    revoked_at TIMESTAMPTZ,
    revoked_by UUID REFERENCES users(id) ON DELETE SET NULL,
    revocation_reason TEXT,
    CONSTRAINT certificates_revocation_complete CHECK (
      (status = 'VALID' AND revoked_at IS NULL AND revoked_by IS NULL AND revocation_reason IS NULL)
      OR (status = 'REVOKED' AND revoked_at IS NOT NULL AND revoked_by IS NOT NULL AND length(trim(revocation_reason)) > 0)
    )
);
CREATE INDEX certificates_user_id_idx ON certificates(user_id);
CREATE INDEX certificates_event_id_idx ON certificates(event_id);
