-- Phase 10 fix: one definition of player level.
--
-- users.level was maintained by a CASE ladder in the verification transaction
-- (six tiers at 100/300/700/1500/3000 points) while crates/game-wasm and
-- docs/demo-fallback.md use a thirty-level ladder of one level per 100 points.
-- The same 1,000-point player was therefore "level 4" in the verification API
-- response and "level 11" in the client. Deriving the column from eco_points
-- removes the second definition so the two can no longer drift apart.
ALTER TABLE users DROP COLUMN level;
ALTER TABLE users
    ADD COLUMN level INTEGER NOT NULL
    GENERATED ALWAYS AS (LEAST(30, eco_points / 100 + 1)) STORED;

COMMENT ON COLUMN users.level IS
    'Derived display level. Mirrors LEVEL_THRESHOLDS in crates/game-wasm: one level per 100 Eco Points, capped at 30.';
