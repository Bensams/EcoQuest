-- Platform achievements.
--
-- `achievement_progress` counters have been written on every verification since
-- phase 4, but nothing ever read them: /api/achievements/me returned only the
-- blockchain achievements, which exist solely for verified BEACH_CLEANUP
-- participation. Verifying any other activity therefore left the achievements
-- page empty. This catalogue turns those counters into visible achievements.

CREATE TABLE achievement_catalog (
    achievement_key TEXT PRIMARY KEY,
    title           TEXT    NOT NULL,
    description     TEXT    NOT NULL,
    -- The achievement_progress.achievement_key counter this tier reads.
    metric          TEXT    NOT NULL,
    threshold       INTEGER NOT NULL CHECK (threshold > 0),
    sort_order      INTEGER NOT NULL DEFAULT 0
);

INSERT INTO achievement_catalog (achievement_key, title, description, metric, threshold, sort_order)
VALUES
    ('FIRST_STEPS',         'First Steps',         'Complete your first verified mission.',        'verified_participations',     1, 10),
    ('COMMITTED_VOLUNTEER', 'Committed Volunteer', 'Complete 5 verified missions.',                'verified_participations',     5, 20),
    ('COMMUNITY_PILLAR',    'Community Pillar',    'Complete 25 verified missions.',               'verified_participations',    25, 30),
    ('POINT_COLLECTOR',     'Point Collector',     'Earn 500 Eco Points from verified missions.',  'eco_points',                500, 40),
    ('ECO_CHAMPION',        'Eco Champion',        'Earn 2,500 Eco Points from verified missions.','eco_points',               2500, 50),
    ('PLANET_GUARDIAN',     'Planet Guardian',     'Earn 10,000 Eco Points from verified missions.','eco_points',             10000, 60);

-- Blockchain achievements had no display copy, so the UI could only show the
-- raw key.
ALTER TABLE achievement_definitions
    ADD COLUMN title       TEXT NOT NULL DEFAULT '',
    ADD COLUMN description TEXT NOT NULL DEFAULT '';
UPDATE achievement_definitions
   SET title = 'Ocean Guardian',
       description = 'Verified participation in a beach cleanup, recorded on chain.'
 WHERE achievement_key = 'OCEAN_GUARDIAN';

-- Backfill the counters from the ledger so accounts verified before this
-- migration show the achievements they already earned. Both counters are
-- derivable, so this is a repair rather than an invention.
INSERT INTO achievement_progress (user_id, achievement_key, progress)
SELECT user_id, 'verified_participations', count(*)::integer
  FROM participations WHERE status = 'VERIFIED' GROUP BY user_id
ON CONFLICT (user_id, achievement_key) DO UPDATE SET progress = EXCLUDED.progress;

-- Positive entries only: an admin correction must not erase an earned tier.
INSERT INTO achievement_progress (user_id, achievement_key, progress)
SELECT user_id, 'eco_points', sum(amount)::integer
  FROM point_transactions WHERE amount > 0 GROUP BY user_id
ON CONFLICT (user_id, achievement_key) DO UPDATE SET progress = EXCLUDED.progress;
