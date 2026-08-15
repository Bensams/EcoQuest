-- Full demo dataset. Run after migrations with: cargo run -p ecoquest-cli -- seed
-- (the CLI runs this file inside a single transaction). Accounts are created by
-- the seeder first; this file only addresses them by email, so fixed hashes are
-- never needed here. Idempotent: fixed identifiers with ON CONFLICT DO NOTHING.

-- ---------------------------------------------------------------------------
-- Cast:
--   alex  → pending org application ("Alex Greenworks")
--   ben   → approved org owner ("Ben Environmental Org") with 3 events
--   erwin → participant on ben's events (verified + pending)
-- ---------------------------------------------------------------------------

-- ---------------------------------------------------------------------------
-- Organizations.
--   org10: alex's PENDING application
--   org20: ben's APPROVED org (reviewed by eco_admin)
-- ---------------------------------------------------------------------------
INSERT INTO organizations (id, name, organization_type, location, description, verification_status, reviewed_by, reviewed_at, owner_id)
SELECT '00000000-0000-0000-0000-000000000010',
       'Alex Greenworks',
       'NON_PROFIT',
       'Quezon City',
       'Community reforestation initiative awaiting admin review',
       'PENDING',
       NULL,
       NULL,
       (SELECT id FROM users WHERE lower(email) = 'alex@ecoquest.test')
FROM users u WHERE lower(u.email) = 'alex@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

INSERT INTO organizations (id, name, organization_type, location, description, verification_status, reviewed_by, reviewed_at, owner_id)
SELECT '00000000-0000-0000-0000-000000000020',
       'Ben Environmental Org',
       'NON_PROFIT',
       'Makati City',
       'Urban sustainability collective organizing cleanups and recycling drives',
       'APPROVED',
       u.id,
       now() - interval '30 days',
       (SELECT id FROM users WHERE lower(email) = 'ben@ecoquest.test')
FROM users u WHERE lower(u.email) = 'admin@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Events (all owned by ben's org).
--   e21 COMPLETED  beach cleanup (past, erwin checked-in + verified)
--   e22 PUBLISHED  recycling drive (future, erwin registered, pending verification)
--   e23 DRAFT      tree planting (future, not yet published)
-- ---------------------------------------------------------------------------
INSERT INTO events (id, organization_id, created_by, name, description, activity_type, location, starts_at, ends_at, capacity, eco_points, status, published_at, cancelled_at, created_at)
SELECT '00000000-0000-0000-0000-000000000021',
       '00000000-0000-0000-0000-000000000020',
       u.id,
       'Manila Bay Coastal Cleanup',
       'Large-scale coastal cleanup along Manila Bay. Verified waste collection counts toward community impact goals.',
       'BEACH_CLEANUP',
       'Manila Bay, Baseco Beach',
       now() - interval '45 days',
       now() - interval '45 days' + interval '4 hours',
       80, 1000, 'COMPLETED',
       now() - interval '45 days',
       NULL,
       now() - interval '46 days'
FROM users u WHERE lower(u.email) = 'ben@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

INSERT INTO events (id, organization_id, created_by, name, description, activity_type, location, starts_at, ends_at, capacity, eco_points, status, published_at, created_at)
SELECT '00000000-0000-0000-0000-000000000022',
       '00000000-0000-0000-0000-000000000020',
       u.id,
       'Barangay Recycling Drive',
       'Community recycling drive open for registration. Bring plastics, paper, and e-waste.',
       'RECYCLING',
       'Makati City, Barangay Hall',
       now() + interval '14 days',
       now() + interval '14 days' + interval '5 hours',
       100, 500, 'PUBLISHED',
       now() - interval '1 day',
       now() - interval '2 days'
FROM users u WHERE lower(u.email) = 'ben@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

INSERT INTO events (id, organization_id, created_by, name, description, activity_type, location, starts_at, ends_at, capacity, eco_points, status, published_at, created_at)
SELECT '00000000-0000-0000-0000-000000000023',
       '00000000-0000-0000-0000-000000000020',
       u.id,
       'UP Diliman Tree Planting',
       'Draft plan for a reforestation day at UP campus. Not yet published.',
       'TREE_PLANTING',
       'Quezon City, UP Diliman',
       now() + interval '30 days',
       now() + interval '30 days' + interval '3 hours',
       60, 600, 'DRAFT',
       NULL,
       now() - interval '5 days'
FROM users u WHERE lower(u.email) = 'ben@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Expected impact definitions (per verified participant).
-- ---------------------------------------------------------------------------
INSERT INTO event_impact_definitions (event_id, metric, unit, expected_value)
VALUES
    ('00000000-0000-0000-0000-000000000021', 'waste_collected', 'kg', 15)
ON CONFLICT (event_id, metric) DO NOTHING;
INSERT INTO event_impact_definitions (event_id, metric, unit, expected_value)
VALUES
    ('00000000-0000-0000-0000-000000000022', 'waste_collected', 'kg', 8)
ON CONFLICT (event_id, metric) DO NOTHING;
INSERT INTO event_impact_definitions (event_id, metric, unit, expected_value)
VALUES
    ('00000000-0000-0000-0000-000000000023', 'trees_planted', 'trees', 20)
ON CONFLICT (event_id, metric) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Live QR token for the COMPLETED event (used for erwin's check-in).
-- Token hash is deterministic so re-seeding keeps the same token.
-- ---------------------------------------------------------------------------
INSERT INTO event_qr_tokens (id, event_id, token_hash, created_by, activates_at, expires_at)
SELECT '00000000-0000-0000-0000-0000000000f1',
       '00000000-0000-0000-0000-000000000021',
       decode(md5('ecoquest-seed-e21-a') || md5('ecoquest-seed-e21-b'), 'hex'),
       u.id,
       now() - interval '46 days',
       now() + interval '24 hours'
FROM users u WHERE lower(u.email) = 'ben@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Participations.
--   e21 COMPLETED: erwin VERIFIED (checked in, +1000 pts, cert issued)
--   e22 PUBLISHED: erwin REGISTERED (registered, not yet checked in)
-- ---------------------------------------------------------------------------
INSERT INTO participations (id, event_id, user_id, status, registered_at, checked_in_at, qr_token_id, verified_at, verified_by, points_awarded)
SELECT '00000000-0000-0000-0000-0000000000a1',
       '00000000-0000-0000-0000-000000000021',
       u.id, 'VERIFIED',
       now() - interval '45 days',
       now() - interval '45 days' + interval '1 hour',
       '00000000-0000-0000-0000-0000000000f1',
       now() - interval '44 days',
       (SELECT id FROM users WHERE lower(email) = 'ben@ecoquest.test'),
       1000
FROM users u WHERE lower(u.email) = 'erwin@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

INSERT INTO participations (id, event_id, user_id, status, registered_at, checked_in_at, qr_token_id)
SELECT '00000000-0000-0000-0000-0000000000a2',
       '00000000-0000-0000-0000-000000000022',
       u.id, 'REGISTERED',
       now() - interval '2 days',
       NULL,
       NULL
FROM users u WHERE lower(u.email) = 'erwin@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Verification ledger (mirrors PgEventStore writes).
-- ---------------------------------------------------------------------------
INSERT INTO participation_verifications (id, participation_id, verifier_id, decision, reason)
SELECT '00000000-0000-0000-0000-0000000000b1', '00000000-0000-0000-0000-0000000000a1',
       u.id, 'VERIFIED', 'Collected 15kg of waste along Manila Bay shoreline'
FROM users u WHERE lower(u.email) = 'ben@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Point ledger.
-- ---------------------------------------------------------------------------
INSERT INTO point_transactions (id, user_id, participation_id, amount, reason, created_by)
SELECT '00000000-0000-0000-0000-0000000000c1',
       u.id, '00000000-0000-0000-0000-0000000000a1', 1000, 'verified event participation',
       (SELECT id FROM users WHERE lower(email) = 'ben@ecoquest.test')
FROM users u WHERE lower(u.email) = 'erwin@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

-- Recompute eco_points and level exactly as PgEventStore does (30-level curve).
UPDATE users
SET eco_points = COALESCE((SELECT sum(amount) FROM point_transactions WHERE user_id = users.id), 0)::integer,
    level = LEAST((COALESCE((SELECT sum(amount) FROM point_transactions WHERE user_id = users.id), 0) / 100) + 1, 30)
WHERE id IN (SELECT user_id FROM point_transactions);

-- ---------------------------------------------------------------------------
-- Achievement progress for erwin (first verified participation).
-- ---------------------------------------------------------------------------
INSERT INTO achievement_progress (user_id, achievement_key, progress)
SELECT u.id, 'verified_participations', 1 FROM users u WHERE lower(u.email) = 'erwin@ecoquest.test'
ON CONFLICT (user_id, achievement_key) DO NOTHING;
INSERT INTO achievement_progress (user_id, achievement_key, progress)
SELECT u.id, 'eco_points', 1000 FROM users u WHERE lower(u.email) = 'erwin@ecoquest.test'
ON CONFLICT (user_id, achievement_key) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Impact contributions from the completed cleanup (per verified participant).
-- ---------------------------------------------------------------------------
INSERT INTO impact_contributions (id, participation_id, event_id, metric, unit, value, source, verifier_id, attribution_method)
SELECT '00000000-0000-0000-0000-0000000000d1',
       '00000000-0000-0000-0000-0000000000a1',
       '00000000-0000-0000-0000-000000000021',
       'waste_collected', 'kg', 15,
       'EVENT_IMPACT_DEFINITION',
       (SELECT id FROM users WHERE lower(email) = 'ben@ecoquest.test'),
       'INDIVIDUAL_CONTRIBUTION'
ON CONFLICT (id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Certificate for the verified participation.
-- verification_hash is deterministic (64 hex chars from md5 pairs = 32 bytes).
-- storage_key is a placeholder path.
-- ---------------------------------------------------------------------------
INSERT INTO certificates (certificate_number, participation_id, user_id, event_id, organization_id, issued_at, volunteer_duration_minutes, verification_hash, storage_key)
SELECT 'ECO-' || to_char(current_date, 'YYYY') || '-' || lpad(nextval('certificate_number_seq')::text, 6, '0'),
       '00000000-0000-0000-0000-0000000000a1',
       u.id,
       '00000000-0000-0000-0000-000000000021',
       '00000000-0000-0000-0000-000000000020',
       now() - interval '44 days', 240,
       decode(md5('ecoquest-erwin-cleanup') || md5('ecoquest-erwin-cleanup'), 'hex'),
       'certificates/ECO-erwin-cleanup.json'
FROM users u WHERE lower(u.email) = 'erwin@ecoquest.test'
ON CONFLICT (participation_id) DO NOTHING;

-- Certificate outbox row, marked processed so the worker skips it.
INSERT INTO outbox_events (aggregate_type, aggregate_id, event_type, payload, processed_at)
VALUES ('participation', '00000000-0000-0000-0000-0000000000a1', 'certificate.issue',
        jsonb_build_object('participation_id', '00000000-0000-0000-0000-0000000000a1'),
        now())
ON CONFLICT (aggregate_id, event_type) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Solana achievement: OCEAN_GUARDIAN eligible for erwin (one verified beach cleanup).
-- verification_reference is an opaque 64-char hex string.
-- ---------------------------------------------------------------------------
INSERT INTO blockchain_achievements (user_id, achievement_key, status, verification_reference)
SELECT u.id, 'OCEAN_GUARDIAN', 'ELIGIBLE',
       'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
FROM users u WHERE lower(u.email) = 'erwin@ecoquest.test'
ON CONFLICT (user_id, achievement_key) DO NOTHING;
