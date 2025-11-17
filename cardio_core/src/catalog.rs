//! Default catalog of movements and microdose definitions.
//!
//! This module provides the built-in movements and workouts for the system.

use crate::types::*;
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Cached default catalog - built once and reused across all operations
static DEFAULT_CATALOG: Lazy<Catalog> = Lazy::new(|| build_default_catalog_internal());

/// Get a reference to the cached default catalog
///
/// This function returns a reference to the pre-built catalog, avoiding
/// the overhead of rebuilding it on every operation (~50+ allocations).
pub fn get_default_catalog() -> &'static Catalog {
    &DEFAULT_CATALOG
}

/// Builds the default catalog with built-in movements and microdose definitions
///
/// **Note**: For production use, prefer `get_default_catalog()` which returns a
/// cached reference. This function is retained for testing and custom catalog creation.
pub fn build_default_catalog() -> Catalog {
    build_default_catalog_internal()
}

/// Internal function that actually builds the catalog
fn build_default_catalog_internal() -> Catalog {
    let mut movements = HashMap::new();
    let mut microdoses = HashMap::new();

    // ========================================================================
    // Movements
    // ========================================================================

    movements.insert(
        "kb_swing_2h".into(),
        Movement {
            id: "kb_swing_2h".into(),
            name: "Kettlebell Swing (2-hand)".into(),
            kind: MovementKind::KettlebellSwing,
            default_style: MovementStyle::None,
            tags: vec!["vo2".into(), "hinge".into(), "posterior_chain".into()],
            reference_url: Some("https://www.youtube.com/watch?v=YSxHifyI6s8".into()),
        },
    );

    movements.insert(
        "burpee".into(),
        Movement {
            id: "burpee".into(),
            name: "Burpee".into(),
            kind: MovementKind::Burpee,
            default_style: MovementStyle::Burpee(BurpeeStyle::FourCount),
            tags: vec!["vo2".into(), "full_body".into(), "bodyweight".into()],
            reference_url: Some("https://www.youtube.com/watch?v=TU8QYVW0gDU".into()),
        },
    );

    movements.insert(
        "pullup".into(),
        Movement {
            id: "pullup".into(),
            name: "Pull-up".into(),
            kind: MovementKind::Pullup,
            default_style: MovementStyle::Band(BandSpec::None),
            tags: vec![
                "gtg".into(),
                "gtg_ok".into(),
                "upper_body".into(),
                "pull".into(),
            ],
            reference_url: Some("https://www.youtube.com/watch?v=eGo4IYlbE5g".into()),
        },
    );

    movements.insert(
        "hip_cars".into(),
        Movement {
            id: "hip_cars".into(),
            name: "Hip Controlled Articular Rotations (CARs)".into(),
            kind: MovementKind::MobilityDrill,
            default_style: MovementStyle::None,
            tags: vec!["mobility".into(), "hip".into(), "gtg_ok".into()],
            reference_url: Some("https://www.youtube.com/watch?v=mJRXBZGRzKg".into()),
        },
    );

    movements.insert(
        "shoulder_cars".into(),
        Movement {
            id: "shoulder_cars".into(),
            name: "Shoulder Controlled Articular Rotations (CARs)".into(),
            kind: MovementKind::MobilityDrill,
            default_style: MovementStyle::None,
            tags: vec!["mobility".into(), "shoulder".into(), "gtg_ok".into()],
            reference_url: Some("https://www.youtube.com/watch?v=f9y1lOJ0v4A".into()),
        },
    );

    // ========================================================================
    // Microdose Definitions
    // ========================================================================

    // VO2 EMOM: Kettlebell Swings (5 minutes)
    microdoses.insert(
        "emom_kb_swing_5m".into(),
        MicrodoseDefinition {
            id: "emom_kb_swing_5m".into(),
            name: "5-Min EMOM: KB Swings (2-hand)".into(),
            category: MicrodoseCategory::Vo2,
            suggested_duration_seconds: 300,
            gtg_friendly: false,
            reference_url: None,
            blocks: vec![MicrodoseBlock {
                movement_id: "kb_swing_2h".into(),
                movement_style: MovementStyle::None,
                duration_hint_seconds: 60,
                metrics: vec![MetricSpec::Reps {
                    key: "reps".into(),
                    default: 5,
                    min: 3,
                    max: 15,
                    step: 1,
                    progressable: true,
                }],
            }],
        },
    );

    // VO2 EMOM: Burpees (5 minutes)
    microdoses.insert(
        "emom_burpee_5m".into(),
        MicrodoseDefinition {
            id: "emom_burpee_5m".into(),
            name: "5-Min EMOM: Burpees".into(),
            category: MicrodoseCategory::Vo2,
            suggested_duration_seconds: 300,
            gtg_friendly: false,
            reference_url: None,
            blocks: vec![MicrodoseBlock {
                movement_id: "burpee".into(),
                movement_style: MovementStyle::Burpee(BurpeeStyle::FourCount),
                duration_hint_seconds: 60,
                metrics: vec![MetricSpec::Reps {
                    key: "reps".into(),
                    default: 3,
                    min: 2,
                    max: 10,
                    step: 1,
                    progressable: true,
                }],
            }],
        },
    );

    // GTG: Pull-ups (banded)
    microdoses.insert(
        "gtg_pullup_band".into(),
        MicrodoseDefinition {
            id: "gtg_pullup_band".into(),
            name: "GTG: Banded Pull-ups".into(),
            category: MicrodoseCategory::Gtg,
            suggested_duration_seconds: 30,
            gtg_friendly: true,
            reference_url: None,
            blocks: vec![MicrodoseBlock {
                movement_id: "pullup".into(),
                movement_style: MovementStyle::Band(BandSpec::NamedColour("red".into())),
                duration_hint_seconds: 30,
                metrics: vec![
                    MetricSpec::Reps {
                        key: "reps".into(),
                        default: 3,
                        min: 1,
                        max: 8,
                        step: 1,
                        progressable: true,
                    },
                    MetricSpec::Band {
                        key: "band".into(),
                        default: "red".into(),
                        progressable: false,
                    },
                ],
            }],
        },
    );

    // Mobility: Hip CARs
    microdoses.insert(
        "mobility_hip_cars".into(),
        MicrodoseDefinition {
            id: "mobility_hip_cars".into(),
            name: "Hip CARs (3 reps each side)".into(),
            category: MicrodoseCategory::Mobility,
            suggested_duration_seconds: 120,
            gtg_friendly: true,
            reference_url: None,
            blocks: vec![MicrodoseBlock {
                movement_id: "hip_cars".into(),
                movement_style: MovementStyle::None,
                duration_hint_seconds: 120,
                metrics: vec![MetricSpec::Reps {
                    key: "reps_per_side".into(),
                    default: 3,
                    min: 2,
                    max: 5,
                    step: 1,
                    progressable: false,
                }],
            }],
        },
    );

    // Mobility: Shoulder CARs
    microdoses.insert(
        "mobility_shoulder_cars".into(),
        MicrodoseDefinition {
            id: "mobility_shoulder_cars".into(),
            name: "Shoulder CARs (3 reps each side)".into(),
            category: MicrodoseCategory::Mobility,
            suggested_duration_seconds: 120,
            gtg_friendly: true,
            reference_url: None,
            blocks: vec![MicrodoseBlock {
                movement_id: "shoulder_cars".into(),
                movement_style: MovementStyle::None,
                duration_hint_seconds: 120,
                metrics: vec![MetricSpec::Reps {
                    key: "reps_per_side".into(),
                    default: 3,
                    min: 2,
                    max: 5,
                    step: 1,
                    progressable: false,
                }],
            }],
        },
    );

    Catalog {
        movements,
        microdoses,
    }
}

impl Catalog {
    /// Validate the catalog for consistency and completeness
    ///
    /// Returns a list of validation errors, or empty Vec if valid.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // Check for duplicate IDs
        // (already guaranteed by HashMap, but check for empty IDs)
        for (id, movement) in &self.movements {
            if id.is_empty() || movement.id.is_empty() {
                errors.push("Movement has empty ID".to_string());
            }
            if id != &movement.id {
                errors.push(format!(
                    "Movement key '{}' doesn't match movement.id '{}'",
                    id, movement.id
                ));
            }
            if movement.name.is_empty() {
                errors.push(format!("Movement '{}' has empty name", id));
            }
        }

        for (id, def) in &self.microdoses {
            if id.is_empty() || def.id.is_empty() {
                errors.push("Microdose definition has empty ID".to_string());
            }
            if id != &def.id {
                errors.push(format!(
                    "Microdose key '{}' doesn't match definition.id '{}'",
                    id, def.id
                ));
            }
            if def.name.is_empty() {
                errors.push(format!("Microdose '{}' has empty name", id));
            }
            if def.blocks.is_empty() {
                errors.push(format!("Microdose '{}' has no blocks", id));
            }

            // Check that all referenced movements exist
            for block in &def.blocks {
                if !self.movements.contains_key(&block.movement_id) {
                    errors.push(format!(
                        "Microdose '{}' references non-existent movement '{}'",
                        id, block.movement_id
                    ));
                }

                // Validate metrics
                for metric in &block.metrics {
                    match metric {
                        MetricSpec::Reps {
                            min, max, default, ..
                        } => {
                            if default < min {
                                errors.push(format!(
                                    "Microdose '{}': default reps {} < min {}",
                                    id, default, min
                                ));
                            }
                            if default > max {
                                errors.push(format!(
                                    "Microdose '{}': default reps {} > max {}",
                                    id, default, max
                                ));
                            }
                            if min > max {
                                errors.push(format!(
                                    "Microdose '{}': min reps {} > max {}",
                                    id, min, max
                                ));
                            }
                        }
                        MetricSpec::Band { default, .. } => {
                            if default.is_empty() {
                                errors.push(format!(
                                    "Microdose '{}': band metric has empty default",
                                    id
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Check that we have at least one microdose in each category
        let has_vo2 = self
            .microdoses
            .values()
            .any(|d| d.category == MicrodoseCategory::Vo2);
        let has_gtg = self
            .microdoses
            .values()
            .any(|d| d.category == MicrodoseCategory::Gtg);
        let has_mobility = self
            .microdoses
            .values()
            .any(|d| d.category == MicrodoseCategory::Mobility);

        if !has_vo2 {
            errors.push("Catalog has no VO2 microdoses".to_string());
        }
        if !has_gtg {
            errors.push("Catalog has no GTG microdoses".to_string());
        }
        if !has_mobility {
            errors.push("Catalog has no Mobility microdoses".to_string());
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_loads() {
        let catalog = build_default_catalog();
        assert_eq!(catalog.movements.len(), 5);
        assert_eq!(catalog.microdoses.len(), 5);
    }

    #[test]
    fn test_all_referenced_movements_exist() {
        let catalog = build_default_catalog();
        for def in catalog.microdoses.values() {
            for block in &def.blocks {
                assert!(
                    catalog.movements.contains_key(&block.movement_id),
                    "Movement {} referenced but not found",
                    block.movement_id
                );
            }
        }
    }

    #[test]
    fn test_vo2_category_exists() {
        let catalog = build_default_catalog();
        let vo2_count = catalog
            .microdoses
            .values()
            .filter(|d| d.category == MicrodoseCategory::Vo2)
            .count();
        assert!(vo2_count >= 2, "Should have at least 2 VO2 workouts");
    }

    #[test]
    fn test_gtg_category_exists() {
        let catalog = build_default_catalog();
        let gtg_count = catalog
            .microdoses
            .values()
            .filter(|d| d.category == MicrodoseCategory::Gtg)
            .count();
        assert!(gtg_count >= 1, "Should have at least 1 GTG workout");
    }

    #[test]
    fn test_mobility_category_exists() {
        let catalog = build_default_catalog();
        let mobility_count = catalog
            .microdoses
            .values()
            .filter(|d| d.category == MicrodoseCategory::Mobility)
            .count();
        assert!(
            mobility_count >= 2,
            "Should have at least 2 mobility workouts"
        );
    }

    #[test]
    fn test_default_catalog_validates() {
        let catalog = build_default_catalog();
        let errors = catalog.validate();
        assert!(
            errors.is_empty(),
            "Default catalog has validation errors: {:?}",
            errors
        );
    }
}
