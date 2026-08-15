-- Controlled vocabulary for impact metrics.
--
-- `metric` and `unit` were free text on every event. Two problems followed:
--
--   * Totals silently split. Impact aggregates GROUP BY (metric, unit), so one
--     organizer typing 'count' and another 'trees' produced two "Trees Planted"
--     rows that never added up. The community goal matches on metric AND unit,
--     so a mission recorded as 'kgs' contributed nothing towards it and said
--     nothing about why.
--   * `expected_value` had a floor of 0 and no ceiling, so an organizer could
--     declare an arbitrary per-volunteer figure straight into the platform-wide
--     total with no review.

CREATE TABLE impact_metrics (
    metric              TEXT PRIMARY KEY,
    label               TEXT             NOT NULL,
    -- The one unit this metric is ever recorded in.
    unit                TEXT             NOT NULL,
    -- Sanity ceiling for a single verified volunteer's contribution.
    max_per_participant DOUBLE PRECISION NOT NULL CHECK (max_per_participant > 0),
    sort_order          INTEGER          NOT NULL DEFAULT 0
);

INSERT INTO impact_metrics (metric, label, unit, max_per_participant, sort_order) VALUES
    ('waste_collected',   'Waste collected',   'kg',    500,   10),
    ('plastic_removed',   'Plastic removed',   'kg',    500,   20),
    ('recyclables_sorted','Recyclables sorted','kg',    500,   30),
    ('trees_planted',     'Trees planted',     'trees', 500,   40),
    ('area_restored',     'Area restored',     'm2',    10000, 50),
    ('volunteer_hours',   'Volunteer hours',   'hours', 24,    60);

-- Adopt any metric already in use that the seed list does not cover, so this
-- migration never destroys data it did not anticipate. Such a row keeps its own
-- unit and gets a permissive ceiling; curate it by hand afterwards.
INSERT INTO impact_metrics (metric, label, unit, max_per_participant, sort_order)
SELECT DISTINCT ON (d.metric)
       d.metric, initcap(replace(d.metric, '_', ' ')), d.unit, 100000, 900
  FROM event_impact_definitions d
 WHERE NOT EXISTS (SELECT 1 FROM impact_metrics m WHERE m.metric = d.metric)
 ORDER BY d.metric, d.unit
ON CONFLICT (metric) DO NOTHING;

-- Fold every recorded unit onto the catalogue's canonical one. Both tables are
-- unique on (owner, metric) rather than on unit, so no row can collide.
UPDATE event_impact_definitions d SET unit = m.unit
  FROM impact_metrics m WHERE d.metric = m.metric AND d.unit <> m.unit;
UPDATE impact_contributions c SET unit = m.unit
  FROM impact_metrics m WHERE c.metric = m.metric AND c.unit <> m.unit;
UPDATE community_goals g SET unit = m.unit
  FROM impact_metrics m WHERE g.metric = m.metric AND g.unit <> m.unit;

-- Enforced in the database as well as the service: a metric outside the
-- catalogue can no longer be written by any path.
ALTER TABLE event_impact_definitions
    ADD CONSTRAINT event_impact_definitions_metric_fkey
    FOREIGN KEY (metric) REFERENCES impact_metrics(metric) ON UPDATE CASCADE;
ALTER TABLE impact_contributions
    ADD CONSTRAINT impact_contributions_metric_fkey
    FOREIGN KEY (metric) REFERENCES impact_metrics(metric) ON UPDATE CASCADE;
