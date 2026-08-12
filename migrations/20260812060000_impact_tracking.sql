-- Phase 7: Impact definitions are per verified participant, never event totals.
-- This prevents multiplying a whole-event total by participant count.
ALTER TABLE impact_contributions
    ADD COLUMN source TEXT NOT NULL DEFAULT 'EVENT_IMPACT_DEFINITION',
    ADD COLUMN verifier_id UUID REFERENCES users(id) ON DELETE SET NULL,
    ADD COLUMN attribution_method TEXT NOT NULL DEFAULT 'INDIVIDUAL_CONTRIBUTION',
    ADD CONSTRAINT impact_contributions_attribution_method_check
        CHECK (attribution_method = 'INDIVIDUAL_CONTRIBUTION');

COMMENT ON COLUMN event_impact_definitions.expected_value IS
    'Per verified participant contribution, not a whole-event total.';
CREATE INDEX impact_contributions_event_metric_idx ON impact_contributions(event_id, metric);
CREATE INDEX participations_user_verified_idx ON participations(user_id) WHERE status = 'VERIFIED';

-- One public MVP goal. Progress derives live from verified contributions.
CREATE TABLE community_goals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    metric TEXT NOT NULL,
    unit TEXT NOT NULL,
    target_value DOUBLE PRECISION NOT NULL CHECK (target_value > 0),
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO community_goals (name, metric, unit, target_value)
VALUES ('Collect 10,000 kg of waste', 'waste_collected', 'kg', 10000);
CREATE UNIQUE INDEX community_goals_one_active_idx ON community_goals(active) WHERE active;
