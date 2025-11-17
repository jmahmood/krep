//! Progression logic for increasing workout intensity.
//!
//! This module implements the progression rules for different movement types:
//! - Burpees: Reps increase to ceiling, then style upgrades
//! - KB swings: Linear rep progression with configurable max
//! - Pullups: Rep progression (band selection is manual)

use crate::{BurpeeStyle, Config, MovementStyle, ProgressionState, UserMicrodoseState};
use chrono::Utc;

/// Upgrade burpee intensity based on current state
///
/// Progression rules:
/// 1. Increase reps until ceiling (default 10)
/// 2. Then upgrade style and reset reps
/// 3. Style progression: 4-count → 6-count → 6-count-2-pump → seal
pub fn upgrade_burpee(state: &mut ProgressionState, rep_ceiling: i32) {
    // If we haven't hit the ceiling, just increment reps
    if state.reps < rep_ceiling {
        state.reps += 1;
        state.level += 1;
        state.last_upgraded = Some(Utc::now());
        tracing::debug!("Burpee progression: increased reps to {}", state.reps);
        return;
    }

    // At ceiling - upgrade style and reset reps
    let (new_style, new_reps) = match &state.style {
        MovementStyle::Burpee(BurpeeStyle::FourCount) => {
            (MovementStyle::Burpee(BurpeeStyle::SixCount), 6)
        }
        MovementStyle::Burpee(BurpeeStyle::SixCount) => {
            (MovementStyle::Burpee(BurpeeStyle::SixCountTwoPump), 5)
        }
        MovementStyle::Burpee(BurpeeStyle::SixCountTwoPump) => {
            (MovementStyle::Burpee(BurpeeStyle::Seal), 4)
        }
        MovementStyle::Burpee(BurpeeStyle::Seal) => {
            // Max level - just increase reps to ceiling
            state.reps = rep_ceiling;
            state.level += 1;
            state.last_upgraded = Some(Utc::now());
            tracing::debug!("Burpee progression: at max level (Seal @ {})", rep_ceiling);
            return;
        }
        _ => {
            // Shouldn't happen, but default to 4-count
            (MovementStyle::Burpee(BurpeeStyle::FourCount), 3)
        }
    };

    state.style = new_style.clone();
    state.reps = new_reps;
    state.level += 1;
    state.last_upgraded = Some(Utc::now());

    tracing::debug!(
        "Burpee progression: upgraded style to {:?}, reset reps to {}",
        new_style,
        new_reps
    );
}

/// Upgrade KB swing intensity (simple linear progression)
///
/// Progression: base_reps + level, capped at max_reps
pub fn upgrade_kb_swing(state: &mut ProgressionState, base_reps: i32, max_reps: i32) {
    if state.reps < max_reps {
        state.reps = (base_reps + state.level as i32 + 1).min(max_reps);
        state.level += 1;
        state.last_upgraded = Some(Utc::now());
        tracing::debug!("KB swing progression: increased to {} reps", state.reps);
    } else {
        tracing::debug!("KB swing progression: already at max ({} reps)", max_reps);
    }
}

/// Upgrade pullup GTG intensity (simple rep progression)
///
/// Progression: Increase reps up to a ceiling
/// Band selection is manual (user decides when to reduce assistance)
pub fn upgrade_pullup(state: &mut ProgressionState, max_reps: i32) {
    if state.reps < max_reps {
        state.reps += 1;
        state.level += 1;
        state.last_upgraded = Some(Utc::now());
        tracing::debug!("Pullup progression: increased to {} reps", state.reps);
    } else {
        tracing::debug!("Pullup progression: already at max ({} reps)", max_reps);
    }
}

/// Upgrade intensity for a specific microdose definition
///
/// This is the main entry point for progression upgrades.
pub fn increase_intensity(def_id: &str, user_state: &mut UserMicrodoseState, config: &Config) {
    // Get or create progression state
    let state = user_state
        .progressions
        .entry(def_id.to_string())
        .or_insert_with(|| {
            // Initialize based on definition type
            let (reps, style) = match def_id {
                "emom_burpee_5m" => (3, MovementStyle::Burpee(BurpeeStyle::FourCount)),
                "emom_kb_swing_5m" => (5, MovementStyle::None),
                "gtg_pullup_band" => (3, MovementStyle::None),
                _ => (3, MovementStyle::None),
            };

            ProgressionState {
                reps,
                style,
                level: 0,
                last_upgraded: None,
            }
        });

    // Apply progression rules based on definition ID
    match def_id {
        "emom_burpee_5m" => {
            upgrade_burpee(state, config.progression.burpee_rep_ceiling);
        }
        "emom_kb_swing_5m" => {
            upgrade_kb_swing(state, 5, config.progression.kb_swing_max_reps);
        }
        "gtg_pullup_band" => {
            upgrade_pullup(state, 8);
        }
        _ => {
            tracing::warn!("Unknown definition ID for progression: {}", def_id);
        }
    }

    tracing::info!(
        "Increased intensity for {}: level {}, {} reps",
        def_id,
        state.level,
        state.reps
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_burpee_reps_progression() {
        let mut state = ProgressionState {
            reps: 3,
            style: MovementStyle::Burpee(BurpeeStyle::FourCount),
            level: 0,
            last_upgraded: None,
        };

        // Should increase reps until ceiling
        for expected_reps in 4..=10 {
            upgrade_burpee(&mut state, 10);
            assert_eq!(state.reps, expected_reps);
        }
    }

    #[test]
    fn test_burpee_style_upgrade() {
        let mut state = ProgressionState {
            reps: 10,
            style: MovementStyle::Burpee(BurpeeStyle::FourCount),
            level: 7,
            last_upgraded: None,
        };

        // At ceiling, should upgrade to 6-count
        upgrade_burpee(&mut state, 10);
        assert!(matches!(
            state.style,
            MovementStyle::Burpee(BurpeeStyle::SixCount)
        ));
        assert_eq!(state.reps, 6); // Reset to lower reps
    }

    #[test]
    fn test_burpee_full_progression() {
        let mut state = ProgressionState {
            reps: 3,
            style: MovementStyle::Burpee(BurpeeStyle::FourCount),
            level: 0,
            last_upgraded: None,
        };

        // Progress through all styles
        for _ in 0..7 {
            upgrade_burpee(&mut state, 10);
        } // 4-count at 10

        upgrade_burpee(&mut state, 10); // Should upgrade to 6-count
        assert!(matches!(
            state.style,
            MovementStyle::Burpee(BurpeeStyle::SixCount)
        ));

        for _ in 0..4 {
            upgrade_burpee(&mut state, 10);
        } // 6-count at 10

        upgrade_burpee(&mut state, 10); // Should upgrade to 6-count-2-pump
        assert!(matches!(
            state.style,
            MovementStyle::Burpee(BurpeeStyle::SixCountTwoPump)
        ));

        for _ in 0..5 {
            upgrade_burpee(&mut state, 10);
        } // 6-count-2-pump at 10

        upgrade_burpee(&mut state, 10); // Should upgrade to Seal
        assert!(matches!(
            state.style,
            MovementStyle::Burpee(BurpeeStyle::Seal)
        ));
    }

    #[test]
    fn test_kb_swing_progression() {
        let mut state = ProgressionState {
            reps: 5,
            style: MovementStyle::None,
            level: 0,
            last_upgraded: None,
        };

        upgrade_kb_swing(&mut state, 5, 15);
        assert_eq!(state.reps, 6);
        assert_eq!(state.level, 1);

        upgrade_kb_swing(&mut state, 5, 15);
        assert_eq!(state.reps, 7);
        assert_eq!(state.level, 2);
    }

    #[test]
    fn test_kb_swing_respects_max() {
        let mut state = ProgressionState {
            reps: 14,
            style: MovementStyle::None,
            level: 9,
            last_upgraded: None,
        };

        upgrade_kb_swing(&mut state, 5, 15);
        assert_eq!(state.reps, 15);

        // Should not exceed max
        upgrade_kb_swing(&mut state, 5, 15);
        assert_eq!(state.reps, 15);
    }

    #[test]
    fn test_pullup_progression() {
        let mut state = ProgressionState {
            reps: 3,
            style: MovementStyle::None,
            level: 0,
            last_upgraded: None,
        };

        for expected_reps in 4..=8 {
            upgrade_pullup(&mut state, 8);
            assert_eq!(state.reps, expected_reps);
        }

        // Should not exceed max
        upgrade_pullup(&mut state, 8);
        assert_eq!(state.reps, 8);
    }

    #[test]
    fn test_increase_intensity_creates_state() {
        let mut user_state = UserMicrodoseState::default();
        let config = Config::default();

        increase_intensity("emom_burpee_5m", &mut user_state, &config);

        assert!(user_state.progressions.contains_key("emom_burpee_5m"));
        let state = &user_state.progressions["emom_burpee_5m"];
        assert_eq!(state.reps, 4); // Started at 3, increased to 4
        assert_eq!(state.level, 1);
    }
}
