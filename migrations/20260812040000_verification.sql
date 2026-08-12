-- Phase 4: verified participation ledger and asynchronous side effects.
ALTER TABLE users ADD COLUMN eco_points INTEGER NOT NULL DEFAULT 0 CHECK (eco_points >= 0);
ALTER TABLE users ADD COLUMN level INTEGER NOT NULL DEFAULT 1 CHECK (level >= 1);

CREATE TABLE participation_verifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    participation_id UUID NOT NULL UNIQUE REFERENCES participations(id) ON DELETE RESTRICT,
    verifier_id UUID REFERENCES users(id) ON DELETE SET NULL,
    decision participation_status NOT NULL CHECK (decision IN ('VERIFIED', 'REJECTED')),
    reason TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Append-only ledger. Corrections add a compensating entry; never delete ledger history.
CREATE TABLE point_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    participation_id UUID UNIQUE REFERENCES participations(id) ON DELETE RESTRICT,
    amount INTEGER NOT NULL,
    reason TEXT NOT NULL,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (amount <> 0),
    CHECK (length(trim(reason)) > 0)
);
CREATE INDEX point_transactions_user_created_idx ON point_transactions(user_id, created_at DESC);

CREATE TABLE achievement_progress (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    achievement_key TEXT NOT NULL,
    progress INTEGER NOT NULL DEFAULT 0 CHECK (progress >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, achievement_key)
);

CREATE TABLE impact_contributions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    participation_id UUID NOT NULL REFERENCES participations(id) ON DELETE RESTRICT,
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE RESTRICT,
    metric TEXT NOT NULL,
    unit TEXT NOT NULL,
    value DOUBLE PRECISION NOT NULL CHECK (value >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(participation_id, metric)
);

CREATE TABLE outbox_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    aggregate_type TEXT NOT NULL,
    aggregate_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    processed_at TIMESTAMPTZ,
    UNIQUE(aggregate_id, event_type)
);
CREATE INDEX outbox_events_unprocessed_idx ON outbox_events(created_at) WHERE processed_at IS NULL;

CREATE TRIGGER achievement_progress_set_updated_at BEFORE UPDATE ON achievement_progress
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- A rejection is terminal and cannot carry rewards.
ALTER TABLE participations ADD CONSTRAINT participations_rejected_no_points
CHECK (status <> 'REJECTED' OR points_awarded = 0);
