//! Deterministic presentation-side progression compiled to WebAssembly.
//!
//! API owns Eco Points. This crate has no point mutation or award API; it only
//! derives display state from a server-provided total.

/// Points required to enter each level. Index zero is level one.
const LEVEL_THRESHOLDS: [u32; 30] = [
    0, 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000, 1100, 1200, 1300, 1400, 1500, 1600, 1700,
    1800, 1900, 2000, 2100, 2200, 2300, 2400, 2500, 2600, 2700, 2800, 2900,
];

/// Read-only game state derived from trusted API point total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameState {
    pub level: u32,
    pub progress_points: u32,
    pub points_to_next_level: u32,
    pub character: &'static str,
    pub environment: &'static str,
    /// Zero polluted, one cleanup started, two recovering, three healthy.
    pub restoration_stage: u32,
}

/// Derives display-only state from API-provided Eco Points.
#[must_use]
pub fn game_state_for_points(points: u32) -> GameState {
    let level_index = LEVEL_THRESHOLDS
        .iter()
        .rposition(|threshold| points >= *threshold)
        .unwrap_or(0);
    let level = u32::try_from(level_index).unwrap_or(0) + 1;
    let current_threshold = LEVEL_THRESHOLDS[level_index];
    let next_threshold = LEVEL_THRESHOLDS.get(level_index + 1).copied();
    GameState {
        level,
        progress_points: points.saturating_sub(current_threshold),
        points_to_next_level: next_threshold.map_or(0, |next| next.saturating_sub(points)),
        character: if level >= 20 {
            "Eco Guardian"
        } else if level >= 5 {
            "Turtle"
        } else {
            "Beginner"
        },
        environment: if level >= 30 {
            "Global Explorer"
        } else if level >= 15 {
            "Forest"
        } else if level >= 10 {
            "Ocean"
        } else {
            "Coast"
        },
        restoration_stage: if level >= 20 {
            3
        } else if level >= 10 {
            2
        } else if level >= 5 {
            1
        } else {
            0
        },
    }
}

#[cfg(target_arch = "wasm32")]
mod bindings {
    use super::game_state_for_points;
    use wasm_bindgen::prelude::*;

    /// JSON game state for browser rendering. Input must originate at API.
    #[wasm_bindgen]
    pub fn game_state_json(points: u32) -> String {
        let state = game_state_for_points(points);
        format!(
            r#"{{"level":{},"progress_points":{},"points_to_next_level":{},"character":"{}","environment":"{}","restoration_stage":{}}}"#,
            state.level,
            state.progress_points,
            state.points_to_next_level,
            state.character,
            state.environment,
            state.restoration_stage
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_level_boundary_is_correct() {
        for (index, threshold) in LEVEL_THRESHOLDS.iter().enumerate() {
            let expected = u32::try_from(index).unwrap_or(0) + 1;
            assert_eq!(game_state_for_points(*threshold).level, expected);
            if *threshold > 0 {
                assert_eq!(game_state_for_points(*threshold - 1).level, expected - 1);
            }
        }
        assert_eq!(game_state_for_points(u32::MAX).level, 30);
    }

    #[test]
    fn unlock_boundaries_are_correct() {
        assert_eq!(game_state_for_points(399).character, "Beginner");
        assert_eq!(game_state_for_points(400).character, "Turtle");
        assert_eq!(game_state_for_points(899).environment, "Coast");
        assert_eq!(game_state_for_points(900).environment, "Ocean");
        assert_eq!(game_state_for_points(1400).environment, "Forest");
        assert_eq!(game_state_for_points(1900).character, "Eco Guardian");
        assert_eq!(game_state_for_points(1900).restoration_stage, 3);
        assert_eq!(game_state_for_points(2900).environment, "Global Explorer");
    }

    /// `users.level` is a generated column defined as `LEAST(30, eco_points / 100 + 1)`
    /// in migrations/20260812090000_level_alignment.sql. Both ladders must agree, or
    /// the API and the client will report different levels for the same player.
    #[test]
    fn level_matches_the_generated_database_column() {
        for points in [0_u32, 1, 99, 100, 101, 999, 1000, 2899, 2900, 3000, 10_000] {
            let sql_level = (points / 100 + 1).min(30);
            assert_eq!(
                game_state_for_points(points).level,
                sql_level,
                "ladders disagree at {points} points"
            );
        }
    }

    #[test]
    fn progress_stops_at_max_level() {
        let state = game_state_for_points(3000);
        assert_eq!(state.progress_points, 100);
        assert_eq!(state.points_to_next_level, 0);
    }
}
