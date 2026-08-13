-- Full demo dataset. Run after migrations with: cargo run -p ecoquest-cli -- seed
-- (the CLI runs this file inside a single transaction). Accounts are created by
-- the seeder first; this file only addresses them by email, so fixed hashes are
-- never needed here. Idempotent: fixed identifiers with ON CONFLICT DO NOTHING.

-- ---------------------------------------------------------------------------
-- Organizations. 10 is already approved (competition flow); 11 is pending so
-- the admin review UI has something to do.
-- ---------------------------------------------------------------------------
INSERT INTO organizations (id, name, organization_type, location, description, verification_status, reviewed_by, reviewed_at)
SELECT '00000000-0000-0000-0000-000000000010',
       'Butuan Environmental Organization',
       'NON_PROFIT',
       'Butuan City',
       'Competition demonstration organization',
       'APPROVED',
       u.id,
       now() - interval '30 days'
FROM users u WHERE lower(u.email) = 'admin@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

INSERT INTO organizations (id, name, organization_type, location, description)
VALUES ('00000000-0000-0000-0000-000000000011',
        'Green Valley Collective',
        'COMMUNITY',
        'Los Baños',
        'New group awaiting admin review')
ON CONFLICT (id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Memberships. Organizer owns the approved org; Diego owns the pending one.
-- ---------------------------------------------------------------------------
INSERT INTO organization_members (organization_id, user_id, member_role)
SELECT '00000000-0000-0000-0000-000000000010', u.id, 'OWNER'
FROM users u WHERE lower(u.email) = 'organizer@ecoquest.test'
ON CONFLICT DO NOTHING;
INSERT INTO organization_members (organization_id, user_id, member_role)
SELECT '00000000-0000-0000-0000-000000000010', u.id, 'STAFF'
FROM users u WHERE lower(u.email) = 'alex@ecoquest.test'
ON CONFLICT DO NOTHING;
INSERT INTO organization_members (organization_id, user_id, member_role)
SELECT '00000000-0000-0000-0000-000000000010', u.id, 'STAFF'
FROM users u WHERE lower(u.email) = 'maria@ecoquest.test'
ON CONFLICT DO NOTHING;
INSERT INTO organization_members (organization_id, user_id, member_role)
SELECT '00000000-0000-0000-0000-000000000011', u.id, 'OWNER'
FROM users u WHERE lower(u.email) = 'diego@ecoquest.test'
ON CONFLICT DO NOTHING;

-- ---------------------------------------------------------------------------
-- Events, every lifecycle stage. All authored by the approved org's owner.
--   e1 COMPLETED   beach cleanup (past, verified impact)  +1000 points
--   e2 DRAFT       tree planting (future)
--   e3 PUBLISHED   recycling drive (future, open for sign-up)
--   e4 ACTIVE      waste collection (live window, QR open)
--   e5 CANCELLED   education seminar (past, cancelled)
-- ---------------------------------------------------------------------------
INSERT INTO events (id, organization_id, created_by, name, description, activity_type, location, starts_at, ends_at, capacity, eco_points, status, published_at, cancelled_at, created_at)
SELECT '00000000-0000-0000-0000-0000000000e1',
       '00000000-0000-0000-0000-000000000010',
       u.id,
       'Butuan Coastal Cleanup',
       'Competition demonstration cleanup along the Butuan shoreline. Verified waste is counted toward the community goal.',
       'BEACH_CLEANUP',
       'Butuan City, shoreline',
       now() - interval '60 days',
       now() - interval '60 days' + interval '4 hours',
       100, 1000, 'COMPLETED',
       now() - interval '60 days',
       NULL,
       now() - interval '61 days'
FROM users u WHERE lower(u.email) = 'organizer@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

INSERT INTO events (id, organization_id, created_by, name, description, activity_type, location, starts_at, ends_at, capacity, eco_points, status, published_at, created_at)
SELECT '00000000-0000-0000-0000-0000000000e2',
       '00000000-0000-0000-0000-000000000010',
       u.id,
       'Riverside Tree Planting',
       'Draft plan for a reforestation day. Not yet published.',
       'TREE_PLANTING',
       'Butuan City, river park',
       now() + interval '21 days',
       now() + interval '21 days' + interval '3 hours',
       60, 600, 'DRAFT',
       NULL,
       now() - interval '2 days'
FROM users u WHERE lower(u.email) = 'organizer@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

INSERT INTO events (id, organization_id, created_by, name, description, activity_type, location, starts_at, ends_at, capacity, eco_points, status, published_at, created_at)
SELECT '00000000-0000-0000-0000-0000000000e3',
       '00000000-0000-0000-0000-000000000010',
       u.id,
       'Kneel Down Recycling Drive',
       'Community recycling drive, open for registration.',
       'RECYCLING',
       'Butuan City, barangay hall',
       now() + interval '7 days',
       now() + interval '7 days' + interval '5 hours',
       80, 500, 'PUBLISHED',
       now(),
       now() - interval '1 day'
FROM users u WHERE lower(u.email) = 'organizer@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

INSERT INTO events (id, organization_id, created_by, name, description, activity_type, location, starts_at, ends_at, capacity, eco_points, status, published_at, created_at)
SELECT '00000000-0000-0000-0000-0000000000e4',
       '00000000-0000-0000-0000-000000000010',
       u.id,
       'Waste Collection Drive',
       'Live event: volunteers collect litter in Butuan City center.',
       'WASTE_COLLECTION',
       'Butuan City, downtown',
       now() - interval '2 hours',
       now() + interval '4 hours',
       120, 800, 'ACTIVE',
       now() - interval '3 hours',
       now() - interval '3 hours'
FROM users u WHERE lower(u.email) = 'organizer@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

INSERT INTO events (id, organization_id, created_by, name, description, activity_type, location, starts_at, ends_at, capacity, eco_points, status, published_at, cancelled_at, created_at)
SELECT '00000000-0000-0000-0000-0000000000e5',
       '00000000-0000-0000-0000-000000000010',
       u.id,
       'Eco Education Seminar',
       'Planned seminar, cancelled due to weather.',
       'EDUCATION',
       'Butuan City, community center',
       now() - interval '15 days',
       now() - interval '15 days' + interval '2 hours',
       40, 300, 'CANCELLED',
       now() - interval '20 days',
       now() - interval '16 days',
       now() - interval '21 days'
FROM users u WHERE lower(u.email) = 'organizer@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Expected impact definitions. Per verified participant, never event totals.
-- ---------------------------------------------------------------------------
INSERT INTO event_impact_definitions (event_id, metric, unit, expected_value)
VALUES
    ('00000000-0000-0000-0000-0000000000e1', 'waste_collected', 'kg', 12)
ON CONFLICT (event_id, metric) DO NOTHING;
INSERT INTO event_impact_definitions (event_id, metric, unit, expected_value)
VALUES
    ('00000000-0000-0000-0000-0000000000e2', 'trees_planted', 'trees', 25)
ON CONFLICT (event_id, metric) DO NOTHING;
INSERT INTO event_impact_definitions (event_id, metric, unit, expected_value)
VALUES
    ('00000000-0000-0000-0000-0000000000e3', 'waste_collected', 'kg', 5)
ON CONFLICT (event_id, metric) DO NOTHING;
INSERT INTO event_impact_definitions (event_id, metric, unit, expected_value)
VALUES
    ('00000000-0000-0000-0000-0000000000e4', 'waste_collected', 'kg', 8)
ON CONFLICT (event_id, metric) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Live QR token for the ACTIVE event. Hash is deterministic so re-seeding keeps
-- the same 32-byte token_hash; the real code was only shown when generated.
-- ---------------------------------------------------------------------------
INSERT INTO event_qr_tokens (id, event_id, token_hash, created_by, activates_at, expires_at)
SELECT '00000000-0000-0000-0000-0000000000f1',
       '00000000-0000-0000-0000-0000000000e4',
       decode(md5('ecoquest-seed-e4-a') || md5('ecoquest-seed-e4-b'), 'hex'),
       u.id,
       now() - interval '3 hours',
       now() + interval '24 hours'
FROM users u WHERE lower(u.email) = 'organizer@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Participations.
--   e1 COMPLETED: alex VERIFIED, maria VERIFIED, player REJECTED (sample)
--   e3 PUBLISHED: diego + player REGISTERED (not yet checked in)
--   e4 ACTIVE:    alex PENDING_VERIFICATION (checked in via QR), maria REGISTERED
-- ---------------------------------------------------------------------------
INSERT INTO participations (id, event_id, user_id, status, registered_at, checked_in_at, qr_token_id, verified_at, verified_by, points_awarded)
SELECT '00000000-0000-0000-0000-0000000000a1',
       '00000000-0000-0000-0000-0000000000e1',
       u.id, 'VERIFIED',
       now() - interval '60 days',
       now() - interval '60 days' + interval '1 hour',
       NULL,
       now() - interval '59 days',
       (SELECT id FROM users WHERE lower(email) = 'organizer@ecoquest.test'),
       1000
FROM users u WHERE lower(u.email) = 'alex@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

INSERT INTO participations (id, event_id, user_id, status, registered_at, checked_in_at, qr_token_id, verified_at, verified_by, points_awarded)
SELECT '00000000-0000-0000-0000-0000000000a2',
       '00000000-0000-0000-0000-0000000000e1',
       u.id, 'VERIFIED',
       now() - interval '60 days',
       now() - interval '60 days' + interval '1 hour',
       NULL,
       now() - interval '59 days',
       (SELECT id FROM users WHERE lower(email) = 'organizer@ecoquest.test'),
       1000
FROM users u WHERE lower(u.email) = 'maria@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

INSERT INTO participations (id, event_id, user_id, status, registered_at, checked_in_at, qr_token_id, verified_at, verified_by, points_awarded)
SELECT '00000000-0000-0000-0000-0000000000a3',
       '00000000-0000-0000-0000-0000000000e1',
       u.id, 'REJECTED',
       now() - interval '60 days',
       now() - interval '60 days' + interval '1 hour',
       NULL,
       now() - interval '59 days',
       (SELECT id FROM users WHERE lower(email) = 'organizer@ecoquest.test'),
       0
FROM users u WHERE lower(u.email) = 'player@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

INSERT INTO participations (id, event_id, user_id, status, registered_at, checked_in_at, qr_token_id)
SELECT '00000000-0000-0000-0000-0000000000a4',
       '00000000-0000-0000-0000-0000000000e3',
       u.id, 'REGISTERED',
       now() - interval '1 day',
       NULL,
       NULL
FROM users u WHERE lower(u.email) = 'diego@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

INSERT INTO participations (id, event_id, user_id, status, registered_at, checked_in_at, qr_token_id)
SELECT '00000000-0000-0000-0000-0000000000a5',
       '00000000-0000-0000-0000-0000000000e3',
       u.id, 'REGISTERED',
       now() - interval '2 hours',
       NULL,
       NULL
FROM users u WHERE lower(u.email) = 'player@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

INSERT INTO participations (id, event_id, user_id, status, registered_at, checked_in_at, qr_token_id)
SELECT '00000000-0000-0000-0000-0000000000a6',
       '00000000-0000-0000-0000-0000000000e4',
       u.id, 'PENDING_VERIFICATION',
       now() - interval '4 hours',
       now() - interval '2 hours',
       '00000000-0000-0000-0000-0000000000f1'
FROM users u WHERE lower(u.email) = 'alex@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

INSERT INTO participations (id, event_id, user_id, status, registered_at, checked_in_at, qr_token_id)
SELECT '00000000-0000-0000-0000-0000000000a7',
       '00000000-0000-0000-0000-0000000000e4',
       u.id, 'REGISTERED',
       now() - interval '1 hour',
       NULL,
       NULL
FROM users u WHERE lower(u.email) = 'maria@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Verification ledger for the completed event (mirrors PgEventStore writes).
-- ---------------------------------------------------------------------------
INSERT INTO participation_verifications (id, participation_id, verifier_id, decision, reason)
SELECT '00000000-0000-0000-0000-0000000000b1', '00000000-0000-0000-0000-0000000000a1',
       u.id, 'VERIFIED', 'Collected litter along shoreline section A'
FROM users u WHERE lower(u.email) = 'organizer@ecoquest.test'
ON CONFLICT (id) DO NOTHING;
INSERT INTO participation_verifications (id, participation_id, verifier_id, decision, reason)
SELECT '00000000-0000-0000-0000-0000000000b2', '00000000-0000-0000-0000-0000000000a2',
       u.id, 'VERIFIED', 'Collected litter along shoreline section B'
FROM users u WHERE lower(u.email) = 'organizer@ecoquest.test'
ON CONFLICT (id) DO NOTHING;
INSERT INTO participation_verifications (id, participation_id, verifier_id, decision, reason)
SELECT '00000000-0000-0000-0000-0000000000b3', '00000000-0000-0000-0000-0000000000a3',
       u.id, 'REJECTED', 'No measurable contribution logged'
FROM users u WHERE lower(u.email) = 'organizer@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Point ledger. Append-only; amounts must be non-zero.
-- ---------------------------------------------------------------------------
INSERT INTO point_transactions (id, user_id, participation_id, amount, reason, created_by)
SELECT '00000000-0000-0000-0000-0000000000c1',
       u.id, '00000000-0000-0000-0000-0000000000a1', 1000, 'verified event participation',
       (SELECT id FROM users WHERE lower(email) = 'organizer@ecoquest.test')
FROM users u WHERE lower(u.email) = 'alex@ecoquest.test'
ON CONFLICT (id) DO NOTHING;
INSERT INTO point_transactions (id, user_id, participation_id, amount, reason, created_by)
SELECT '00000000-0000-0000-0000-0000000000c2',
       u.id, '00000000-0000-0000-0000-0000000000a2', 1000, 'verified event participation',
       (SELECT id FROM users WHERE lower(email) = 'organizer@ecoquest.test')
FROM users u WHERE lower(u.email) = 'maria@ecoquest.test'
ON CONFLICT (id) DO NOTHING;

-- Recompute eco_points and level exactly as PgEventStore does.
UPDATE users
SET eco_points = COALESCE((SELECT sum(amount) FROM point_transactions WHERE user_id = users.id), 0)::integer,
    level = CASE
        WHEN COALESCE((SELECT sum(amount) FROM point_transactions WHERE user_id = users.id), 0) >= 3000 THEN 6
        WHEN COALESCE((SELECT sum(amount) FROM point_transactions WHERE user_id = users.id), 0) >= 1500 THEN 5
        WHEN COALESCE((SELECT sum(amount) FROM point_transactions WHERE user_id = users.id), 0) >= 700 THEN 4
        WHEN COALESCE((SELECT sum(amount) FROM point_transactions WHERE user_id = users.id), 0) >= 300 THEN 3
        WHEN COALESCE((SELECT sum(amount) FROM point_transactions WHERE user_id = users.id), 0) >= 100 THEN 2
        ELSE 1
    END
WHERE id IN (SELECT user_id FROM point_transactions);

-- ---------------------------------------------------------------------------
-- Starting progress toward platform achievements.
-- ---------------------------------------------------------------------------
INSERT INTO achievement_progress (user_id, achievement_key, progress)
SELECT u.id, 'verified_participations', 1 FROM users u WHERE lower(u.email) = 'alex@ecoquest.test'
ON CONFLICT (user_id, achievement_key) DO NOTHING;
INSERT INTO achievement_progress (user_id, achievement_key, progress)
SELECT u.id, 'eco_points', 1000 FROM users u WHERE lower(u.email) = 'alex@ecoquest.test'
ON CONFLICT (user_id, achievement_key) DO NOTHING;
INSERT INTO achievement_progress (user_id, achievement_key, progress)
SELECT u.id, 'verified_participations', 1 FROM users u WHERE lower(u.email) = 'maria@ecoquest.test'
ON CONFLICT (user_id, achievement_key) DO NOTHING;
INSERT INTO achievement_progress (user_id, achievement_key, progress)
SELECT u.id, 'eco_points', 1000 FROM users u WHERE lower(u.email) = 'maria@ecoquest.test'
ON CONFLICT (user_id, achievement_key) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Impact contributions from the completed cleanup (per verified participant).
-- ---------------------------------------------------------------------------
INSERT INTO impact_contributions (id, participation_id, event_id, metric, unit, value, source, verifier_id, attribution_method)
SELECT '00000000-0000-0000-0000-0000000000d1',
       '00000000-0000-0000-0000-0000000000a1',
       '00000000-0000-0000-0000-0000000000e1',
       'waste_collected', 'kg', 12,
       'EVENT_IMPACT_DEFINITION',
       (SELECT id FROM users WHERE lower(email) = 'organizer@ecoquest.test'),
       'INDIVIDUAL_CONTRIBUTION'
ON CONFLICT (id) DO NOTHING;
INSERT INTO impact_contributions (id, participation_id, event_id, metric, unit, value, source, verifier_id, attribution_method)
SELECT '00000000-0000-0000-0000-0000000000d2',
       '00000000-0000-0000-0000-0000000000a2',
       '00000000-0000-0000-0000-0000000000e1',
       'waste_collected', 'kg', 12,
       'EVENT_IMPACT_DEFINITION',
       (SELECT id FROM users WHERE lower(email) = 'organizer@ecoquest.test'),
       'INDIVIDUAL_CONTRIBUTION'
ON CONFLICT (id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Certificates for the two verified participants. verification_hash is
-- deterministic (64 hex chars from md5 pairs = 32 bytes); the certificate
-- number comes from the sequence default. Matching outbox row is inserted
-- ALREADY processed so the worker will not re-issue.
-- ---------------------------------------------------------------------------
INSERT INTO certificates (certificate_number, participation_id, user_id, event_id, organization_id, issued_at, volunteer_duration_minutes, verification_hash, storage_key)
SELECT 'ECO-' || to_char(current_date, 'YYYY') || '-' || lpad(nextval('certificate_number_seq')::text, 6, '0'),
       '00000000-0000-0000-0000-0000000000a1',
       u.id,
       '00000000-0000-0000-0000-0000000000e1',
       '00000000-0000-0000-0000-000000000010',
       now() - interval '59 days', 240,
       decode(md5('ecoquest-alex-cleanup') || md5('ecoquest-alex-cleanup'), 'hex'),
       'certificates/ECO-alex-cleanup.json'
FROM users u WHERE lower(u.email) = 'alex@ecoquest.test'
ON CONFLICT (participation_id) DO NOTHING;

INSERT INTO certificates (certificate_number, participation_id, user_id, event_id, organization_id, issued_at, volunteer_duration_minutes, verification_hash, storage_key)
SELECT 'ECO-' || to_char(current_date, 'YYYY') || '-' || lpad(nextval('certificate_number_seq')::text, 6, '0'),
       '00000000-0000-0000-0000-0000000000a2',
       u.id,
       '00000000-0000-0000-0000-0000000000e1',
       '00000000-0000-0000-0000-000000000010',
       now() - interval '59 days', 240,
       decode(md5('ecoquest-maria-cleanup') || md5('ecoquest-maria-cleanup'), 'hex'),
       'certificates/ECO-maria-cleanup.json'
FROM users u WHERE lower(u.email) = 'maria@ecoquest.test'
ON CONFLICT (participation_id) DO NOTHING;

-- Certificate outbox rows, marked processed so the worker skips them.
INSERT INTO outbox_events (aggregate_type, aggregate_id, event_type, payload, processed_at)
VALUES ('participation', '00000000-0000-0000-0000-0000000000a1', 'certificate.issue',
        jsonb_build_object('participation_id', '00000000-0000-0000-0000-0000000000a1'),
        now())
ON CONFLICT (aggregate_id, event_type) DO NOTHING;
INSERT INTO outbox_events (aggregate_type, aggregate_id, event_type, payload, processed_at)
VALUES ('participation', '00000000-0000-0000-0000-0000000000a2', 'certificate.issue',
        jsonb_build_object('participation_id', '00000000-0000-0000-0000-0000000000a2'),
        now())
ON CONFLICT (aggregate_id, event_type) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Solana achievement: OCEAN_GUARDIAN is eligible for anyone with one verified
-- beach cleanup. Seeded directly as ELIGIBLE; minting starts on wallet link.
-- verification_reference is an opaque 64-char hex string.
-- ---------------------------------------------------------------------------
INSERT INTO blockchain_achievements (user_id, achievement_key, status, verification_reference)
SELECT u.id, 'OCEAN_GUARDIAN', 'ELIGIBLE',
       'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
FROM users u WHERE lower(u.email) = 'alex@ecoquest.test'
ON CONFLICT (user_id, achievement_key) DO NOTHING;
INSERT INTO blockchain_achievements (user_id, achievement_key, status, verification_reference)
SELECT u.id, 'OCEAN_GUARDIAN', 'ELIGIBLE',
       '5df2ae3e1b3f6a1c4f6f76b2b8a13d44f0f35d4f8f2f6dd33aa2f91e2c6c4f47'
FROM users u WHERE lower(u.email) = 'maria@ecoquest.test'
ON CONFLICT (user_id, achievement_key) DO NOTHING;