//! Prescription engine for selecting microdose workouts.
//!
//! This module implements the v1.1 prescription logic:
//! - Check strength signal for recent lower-body work
//! - Check time since last VO2 session
//! - Round-robin selection for categories and definitions

use crate::{
    Catalog, Error, MicrodoseCategory, MicrodoseDefinition, ProgressionState, Result,
    StrengthSessionType, UserContext,
};
use chrono::Duration;

/// A prescribed microdose with computed intensity parameters
#[derive(Clone, Debug)]
pub struct PrescribedMicrodose {
    pub definition: MicrodoseDefinition,
    pub reps: Option<i32>,
    pub style: Option<crate::MovementStyle>,
}

/// Prescribe the next microdose based on context and rules
///
/// ## V1.1 Prescription Logic
///
/// 1. **Strength-based override** (within 24h):
///    - If lower-body strength session ≤ 24h ago → GTG pullup OR mobility
///
/// 2. **VO2 timing**:
///    - If last VO2 session > 4h ago → VO2 category
///
/// 3. **Default round-robin**:
///    - Cycle through [VO2, GTG, Mobility] categories
///
pub fn prescribe_next(
    catalog: &Catalog,
    ctx: &UserContext,
    target_category: Option<MicrodoseCategory>,
) -> Result<PrescribedMicrodose> {
    // Determine category to prescribe
    let category = if let Some(cat) = target_category {
        cat
    } else {
        determine_category(ctx)?
    };

    tracing::info!("Prescribing microdose from category: {:?}", category);

    // Select definition from category
    let definition = select_definition_from_category(catalog, ctx, &category)?;

    // Compute intensity based on progression state
    let (reps, style) = compute_intensity(definition, ctx);

    Ok(PrescribedMicrodose {
        definition: definition.clone(),
        reps,
        style,
    })
}

/// Determine which category to prescribe from based on context
fn determine_category(ctx: &UserContext) -> Result<MicrodoseCategory> {
    // Rule 1: Recent lower-body strength → prefer GTG or Mobility
    if let Some(ref strength) = ctx.external_strength {
        let time_since_strength = ctx.now - strength.last_session_at;

        if time_since_strength < Duration::hours(24)
            && strength.session_type == StrengthSessionType::Lower
        {
            tracing::info!(
                "Recent lower-body strength detected ({} hours ago), preferring GTG/Mobility",
                time_since_strength.num_hours()
            );
            return Ok(MicrodoseCategory::Gtg);
        }
    }

    // Rule 2: Check time since last VO2 session
    let last_vo2 = crate::history::find_last_session_by_category(&ctx.recent_sessions, "vo2");

    if let Some(last_vo2_session) = last_vo2 {
        let time_since_vo2 = ctx.now - last_vo2_session.performed_at;

        if time_since_vo2 > Duration::hours(4) {
            tracing::info!(
                "Last VO2 session was {} hours ago (> 4h), prescribing VO2",
                time_since_vo2.num_hours()
            );
            return Ok(MicrodoseCategory::Vo2);
        }
    } else {
        // No VO2 sessions in history → prescribe VO2
        tracing::info!("No recent VO2 sessions found, prescribing VO2");
        return Ok(MicrodoseCategory::Vo2);
    }

    // Rule 3: Default round-robin based on last category
    let last_category = ctx
        .recent_sessions
        .first()
        .and_then(|s| {
            // Infer category from definition ID
            if s.definition_id.contains("vo2") || s.definition_id.contains("emom") {
                Some(MicrodoseCategory::Vo2)
            } else if s.definition_id.contains("gtg") {
                Some(MicrodoseCategory::Gtg)
            } else if s.definition_id.contains("mobility") {
                Some(MicrodoseCategory::Mobility)
            } else {
                None
            }
        });

    let next_category = match last_category {
        Some(MicrodoseCategory::Vo2) => MicrodoseCategory::Gtg,
        Some(MicrodoseCategory::Gtg) => MicrodoseCategory::Mobility,
        Some(MicrodoseCategory::Mobility) => MicrodoseCategory::Vo2,
        None => MicrodoseCategory::Vo2, // Default to VO2 if unknown
    };

    tracing::info!("Round-robin selection: {:?}", next_category);
    Ok(next_category)
}

/// Select a specific definition from a category
fn select_definition_from_category<'a>(
    catalog: &'a Catalog,
    ctx: &UserContext,
    category: &MicrodoseCategory,
) -> Result<&'a MicrodoseDefinition> {
    // Get all definitions in the category
    let mut candidates: Vec<_> = catalog
        .microdoses
        .values()
        .filter(|d| &d.category == category)
        .collect();

    if candidates.is_empty() {
        return Err(Error::Prescription(format!(
            "No microdoses found in category {:?}",
            category
        )));
    }

    // Sort for deterministic selection
    candidates.sort_by_key(|d| &d.id);

    // Handle category-specific selection logic
    match category {
        MicrodoseCategory::Vo2 => {
            // Round-robin between VO2 definitions
            let last_vo2_def = ctx
                .recent_sessions
                .iter()
                .find(|s| s.definition_id.contains("vo2") || s.definition_id.contains("emom"))
                .map(|s| s.definition_id.as_str());

            // Pick the one we didn't do last time
            if let Some(last) = last_vo2_def {
                candidates
                    .iter()
                    .find(|d| d.id != last)
                    .copied()
                    .or_else(|| candidates.first().copied())
                    .ok_or_else(|| Error::Prescription("No VO2 definition available".into()))
            } else {
                // No previous VO2, pick first
                Ok(candidates[0])
            }
        }

        MicrodoseCategory::Gtg => {
            // Just pick the first (only one GTG definition in default catalog)
            Ok(candidates[0])
        }

        MicrodoseCategory::Mobility => {
            // Round-robin through mobility definitions
            let last_mobility = ctx.user_state.last_mobility_def_id.as_deref();

            if let Some(last) = last_mobility {
                // Find next in sequence
                let last_idx = candidates.iter().position(|d| d.id == last);
                if let Some(idx) = last_idx {
                    let next_idx = (idx + 1) % candidates.len();
                    Ok(candidates[next_idx])
                } else {
                    Ok(candidates[0])
                }
            } else {
                Ok(candidates[0])
            }
        }
    }
}

/// Compute intensity (reps/style) based on progression state
fn compute_intensity(
    definition: &MicrodoseDefinition,
    ctx: &UserContext,
) -> (Option<i32>, Option<crate::MovementStyle>) {
    if let Some(state) = ctx.user_state.progressions.get(&definition.id) {
        (Some(state.reps), Some(state.style.clone()))
    } else {
        // No progression state - use defaults from definition
        let first_block = definition.blocks.first();
        let default_reps = first_block.and_then(|b| {
            b.metrics.iter().find_map(|m| match m {
                crate::MetricSpec::Reps { default, .. } => Some(*default),
                _ => None,
            })
        });

        let default_style = first_block.map(|b| b.movement_style.clone());

        (default_reps, default_style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_default_catalog, ExternalStrengthSignal, UserMicrodoseState};
    use chrono::Utc;

    fn create_test_context() -> UserContext {
        UserContext {
            now: Utc::now(),
            user_state: UserMicrodoseState::default(),
            recent_sessions: vec![],
            external_strength: None,
            equipment_available: vec![],
        }
    }

    #[test]
    fn test_prescribe_vo2_when_no_history() {
        let catalog = build_default_catalog();
        let ctx = create_test_context();

        let prescribed = prescribe_next(&catalog, &ctx, None).unwrap();

        assert_eq!(prescribed.definition.category, MicrodoseCategory::Vo2);
    }

    #[test]
    fn test_prescribe_gtg_after_lower_strength() {
        let catalog = build_default_catalog();
        let mut ctx = create_test_context();

        ctx.external_strength = Some(ExternalStrengthSignal {
            last_session_at: Utc::now() - Duration::hours(12),
            session_type: StrengthSessionType::Lower,
        });

        let prescribed = prescribe_next(&catalog, &ctx, None).unwrap();

        assert_eq!(prescribed.definition.category, MicrodoseCategory::Gtg);
    }

    #[test]
    fn test_respects_target_category() {
        let catalog = build_default_catalog();
        let ctx = create_test_context();

        let prescribed =
            prescribe_next(&catalog, &ctx, Some(MicrodoseCategory::Mobility)).unwrap();

        assert_eq!(
            prescribed.definition.category,
            MicrodoseCategory::Mobility
        );
    }

    #[test]
    fn test_compute_intensity_with_progression() {
        let catalog = build_default_catalog();
        let def = catalog.microdoses.get("emom_burpee_5m").unwrap();

        let mut ctx = create_test_context();
        ctx.user_state.progressions.insert(
            "emom_burpee_5m".to_string(),
            ProgressionState {
                reps: 7,
                style: crate::MovementStyle::Burpee(crate::BurpeeStyle::SixCount),
                level: 10,
                last_upgraded: Some(Utc::now()),
            },
        );

        let (reps, style) = compute_intensity(def, &ctx);

        assert_eq!(reps, Some(7));
        assert!(matches!(
            style,
            Some(crate::MovementStyle::Burpee(crate::BurpeeStyle::SixCount))
        ));
    }

    #[test]
    fn test_compute_intensity_without_progression() {
        let catalog = build_default_catalog();
        let def = catalog.microdoses.get("emom_burpee_5m").unwrap();

        let ctx = create_test_context();

        let (reps, _style) = compute_intensity(def, &ctx);

        // Should use default from definition
        assert_eq!(reps, Some(3));
    }
}
