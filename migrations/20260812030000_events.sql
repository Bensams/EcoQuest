-- Phase 3: environmental missions, QR check-in and participation tracking.

CREATE TYPE event_status AS ENUM ('DRAFT', 'PUBLISHED', 'ACTIVE', 'COMPLETED', 'CANCELLED');

CREATE TYPE activity_type AS ENUM (
    'TREE_PLANTING',
    'WASTE_COLLECTION',
    'RECYCLING',
    'BEACH_CLEANUP',
    'ENERGY_SAVING',
    'EDUCATION',
    'OTHER'
);

CREATE TYPE participation_status AS ENUM (
    'REGISTERED',
    'PENDING_VERIFICATION',
    'VERIFIED',
    'REJECTED',
    'CANCELLED'
);

CREATE TABLE events (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID          NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    created_by      UUID          REFERENCES users (id) ON DELETE SET NULL,
    name            TEXT          NOT NULL,
    description     TEXT          NOT NULL DEFAULT '',
    activity_type   activity_type NOT NULL,
    location        TEXT          NOT NULL,
    starts_at       TIMESTAMPTZ   NOT NULL,
    ends_at         TIMESTAMPTZ   NOT NULL,
    -- Registration capacity. Bounded so a typo cannot create an unbounded event.
    capacity        INTEGER       NOT NULL CHECK (capacity > 0 AND capacity <= 100000),
    -- Reward granted on verification, never at check-in time.
    eco_points      INTEGER       NOT NULL CHECK (eco_points >= 0 AND eco_points <= 100000),
    status          event_status  NOT NULL DEFAULT 'DRAFT',
    published_at    TIMESTAMPTZ,
    cancelled_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ   NOT NULL DEFAULT now(),
    CONSTRAINT events_window_ordered CHECK (ends_at > starts_at)
);

CREATE INDEX events_organization_id_idx ON events (organization_id);
-- Browsing published missions is the hottest query; order matches the API.
CREATE INDEX events_status_starts_at_idx ON events (status, starts_at);

-- Expected impact, one row per metric (trees, waste_kg, volunteer_hours, ...).
-- A table instead of columns so new metrics need no migration.
CREATE TABLE event_impact_definitions (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id       UUID             NOT NULL REFERENCES events (id) ON DELETE CASCADE,
    metric         TEXT             NOT NULL,
    unit           TEXT             NOT NULL,
    expected_value DOUBLE PRECISION NOT NULL CHECK (expected_value >= 0),
    created_at     TIMESTAMPTZ      NOT NULL DEFAULT now(),
    UNIQUE (event_id, metric)
);

-- Only the SHA-256 hash of the check-in code is stored, so a database leak
-- cannot be replayed at a mission site. The code itself is shown once, when it
-- is generated or rotated.
CREATE TABLE event_qr_tokens (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id     UUID        NOT NULL REFERENCES events (id) ON DELETE CASCADE,
    token_hash   BYTEA       NOT NULL UNIQUE,
    created_by   UUID        REFERENCES users (id) ON DELETE SET NULL,
    activates_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL,
    revoked_at   TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT event_qr_tokens_window_ordered CHECK (expires_at > activates_at)
);

-- Rotation is enforced by the database: an event can never have two live codes.
CREATE UNIQUE INDEX event_qr_tokens_one_active_idx ON event_qr_tokens (event_id)
    WHERE revoked_at IS NULL;

CREATE TABLE participations (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id       UUID                 NOT NULL REFERENCES events (id) ON DELETE CASCADE,
    user_id        UUID                 NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    status         participation_status NOT NULL DEFAULT 'REGISTERED',
    registered_at  TIMESTAMPTZ          NOT NULL DEFAULT now(),
    checked_in_at  TIMESTAMPTZ,
    -- Which code was scanned; keeps an audit link after rotation.
    qr_token_id    UUID REFERENCES event_qr_tokens (id) ON DELETE SET NULL,
    verified_at    TIMESTAMPTZ,
    verified_by    UUID REFERENCES users (id) ON DELETE SET NULL,
    points_awarded INTEGER              NOT NULL DEFAULT 0 CHECK (points_awarded >= 0),
    created_at     TIMESTAMPTZ          NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ          NOT NULL DEFAULT now(),
    -- One participation per user per event.
    UNIQUE (event_id, user_id),
    -- Points exist only after verification: check-in can never award them.
    CONSTRAINT participations_points_require_verification
        CHECK (points_awarded = 0 OR status = 'VERIFIED'),
    -- A checked-in participation must record when it happened.
    CONSTRAINT participations_checkin_timestamped
        CHECK (status = 'REGISTERED' OR status = 'CANCELLED' OR checked_in_at IS NOT NULL)
);

CREATE INDEX participations_user_id_idx ON participations (user_id);
CREATE INDEX participations_event_status_idx ON participations (event_id, status);

CREATE TRIGGER events_set_updated_at
    BEFORE UPDATE ON events
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER participations_set_updated_at
    BEFORE UPDATE ON participations
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
