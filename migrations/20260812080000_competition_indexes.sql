-- Phase 10 query review: existing UNIQUE / lifecycle indexes cover event and certificate lookups.
-- Personal-impact aggregation joins verified participations to contributions by participation id.
CREATE INDEX impact_contributions_participation_id_idx ON impact_contributions(participation_id);
