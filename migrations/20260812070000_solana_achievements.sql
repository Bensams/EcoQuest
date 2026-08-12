-- Phase 8: server-authoritative, privacy-preserving Solana achievements.
CREATE TYPE achievement_mint_status AS ENUM
    ('NOT_ELIGIBLE', 'ELIGIBLE', 'QUEUED', 'MINTING', 'MINTED', 'FAILED');

CREATE TABLE wallet_challenges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    wallet_address TEXT NOT NULL,
    nonce_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(nonce_hash) = 32),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (expires_at > created_at)
);
CREATE INDEX wallet_challenges_open_idx ON wallet_challenges(user_id, expires_at)
    WHERE consumed_at IS NULL;

CREATE TABLE achievement_definitions (
    achievement_key TEXT PRIMARY KEY,
    required_verified_beach_cleanups INTEGER NOT NULL CHECK (required_verified_beach_cleanups > 0)
);
INSERT INTO achievement_definitions (achievement_key, required_verified_beach_cleanups)
VALUES ('OCEAN_GUARDIAN', 1);

CREATE TABLE blockchain_achievements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    achievement_key TEXT NOT NULL REFERENCES achievement_definitions(achievement_key) ON DELETE RESTRICT,
    status achievement_mint_status NOT NULL DEFAULT 'NOT_ELIGIBLE',
    -- Opaque server-generated reference; no name, email, location, or user UUID on chain.
    verification_reference TEXT NOT NULL UNIQUE,
    wallet_address TEXT,
    mint_identifier TEXT,
    transaction_signature TEXT,
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    next_attempt_at TIMESTAMPTZ,
    last_error TEXT,
    minted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, achievement_key),
    CHECK ((status <> 'MINTED') OR (mint_identifier IS NOT NULL AND transaction_signature IS NOT NULL))
);
CREATE INDEX blockchain_achievements_due_idx ON blockchain_achievements(next_attempt_at)
    WHERE status IN ('QUEUED', 'FAILED');
CREATE TRIGGER blockchain_achievements_set_updated_at BEFORE UPDATE ON blockchain_achievements
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

ALTER TABLE outbox_events ADD COLUMN available_at TIMESTAMPTZ NOT NULL DEFAULT now();
CREATE INDEX outbox_events_available_idx ON outbox_events(available_at, created_at)
    WHERE processed_at IS NULL;

-- DB transaction that verifies a participation creates eligibility only. Minting is separate.
-- Requirement is configurable here; UI has no achievement-award endpoint.
CREATE OR REPLACE FUNCTION queue_ocean_guardian_achievement() RETURNS trigger AS $$
DECLARE
    required_count INTEGER;
    verified_count INTEGER;
    achievement_id UUID;
BEGIN
    IF NEW.status <> 'VERIFIED' OR OLD.status = 'VERIFIED' THEN RETURN NEW; END IF;
    IF NOT EXISTS (SELECT 1 FROM events WHERE id = NEW.event_id AND activity_type = 'BEACH_CLEANUP') THEN RETURN NEW; END IF;
    SELECT required_verified_beach_cleanups INTO required_count
      FROM achievement_definitions WHERE achievement_key = 'OCEAN_GUARDIAN';
    SELECT count(*) INTO verified_count FROM participations p JOIN events e ON e.id=p.event_id
      WHERE p.user_id=NEW.user_id AND p.status='VERIFIED' AND e.activity_type='BEACH_CLEANUP';
    IF verified_count < required_count THEN RETURN NEW; END IF;
    INSERT INTO blockchain_achievements (user_id, achievement_key, status, verification_reference)
      VALUES (NEW.user_id, 'OCEAN_GUARDIAN', 'ELIGIBLE', encode(gen_random_bytes(32), 'hex'))
      ON CONFLICT (user_id, achievement_key) DO NOTHING RETURNING id INTO achievement_id;
    IF achievement_id IS NOT NULL THEN
      INSERT INTO outbox_events (aggregate_type, aggregate_id, event_type, payload)
      VALUES ('blockchain_achievement', achievement_id, 'achievement.mint', jsonb_build_object('achievement_id', achievement_id));
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER participations_queue_ocean_guardian AFTER UPDATE OF status ON participations
FOR EACH ROW EXECUTE FUNCTION queue_ocean_guardian_achievement();
