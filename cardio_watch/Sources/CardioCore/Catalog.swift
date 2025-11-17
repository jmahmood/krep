/// Default catalog of movements and microdose definitions.
///
/// This module provides the built-in movements and workouts for the system.
/// Direct port from Rust `cardio_core/src/catalog.rs`

import Foundation

// MARK: - Catalog Builder

/// Builds the default catalog with built-in movements and microdose definitions
public func buildDefaultCatalog() -> Catalog {
    var movements: [String: Movement] = [:]
    var microdoses: [String: MicrodoseDefinition] = [:]

    // ========================================================================
    // Movements
    // ========================================================================

    movements["kb_swing_2h"] = Movement(
        id: "kb_swing_2h",
        name: "Kettlebell Swing (2-hand)",
        kind: .kettlebellSwing,
        defaultStyle: .none,
        tags: ["vo2", "hinge", "posterior_chain"],
        referenceUrl: "https://www.youtube.com/watch?v=YSxHifyI6s8"
    )

    movements["burpee"] = Movement(
        id: "burpee",
        name: "Burpee",
        kind: .burpee,
        defaultStyle: .burpee(.fourCount),
        tags: ["vo2", "full_body", "bodyweight"],
        referenceUrl: "https://www.youtube.com/watch?v=TU8QYVW0gDU"
    )

    movements["pullup"] = Movement(
        id: "pullup",
        name: "Pull-up",
        kind: .pullup,
        defaultStyle: .band(.none),
        tags: ["gtg", "gtg_ok", "upper_body", "pull"],
        referenceUrl: "https://www.youtube.com/watch?v=eGo4IYlbE5g"
    )

    movements["hip_cars"] = Movement(
        id: "hip_cars",
        name: "Hip Controlled Articular Rotations (CARs)",
        kind: .mobilityDrill,
        defaultStyle: .none,
        tags: ["mobility", "hip", "gtg_ok"],
        referenceUrl: "https://www.youtube.com/watch?v=mJRXBZGRzKg"
    )

    movements["shoulder_cars"] = Movement(
        id: "shoulder_cars",
        name: "Shoulder Controlled Articular Rotations (CARs)",
        kind: .mobilityDrill,
        defaultStyle: .none,
        tags: ["mobility", "shoulder", "gtg_ok"],
        referenceUrl: "https://www.youtube.com/watch?v=f9y1lOJ0v4A"
    )

    // ========================================================================
    // Microdose Definitions
    // ========================================================================

    // VO2 EMOM: Kettlebell Swings (5 minutes)
    microdoses["emom_kb_swing_5m"] = MicrodoseDefinition(
        id: "emom_kb_swing_5m",
        name: "5-Min EMOM: KB Swings (2-hand)",
        category: .vo2,
        suggestedDurationSeconds: 300,
        gtgFriendly: false,
        blocks: [
            MicrodoseBlock(
                movementId: "kb_swing_2h",
                movementStyle: .none,
                durationHintSeconds: 60,
                metrics: [
                    .reps(
                        key: "reps",
                        defaultValue: 5,
                        min: 3,
                        max: 15,
                        step: 1,
                        progressable: true
                    )
                ]
            )
        ]
    )

    // VO2 EMOM: Burpees (5 minutes)
    microdoses["emom_burpee_5m"] = MicrodoseDefinition(
        id: "emom_burpee_5m",
        name: "5-Min EMOM: Burpees",
        category: .vo2,
        suggestedDurationSeconds: 300,
        gtgFriendly: false,
        blocks: [
            MicrodoseBlock(
                movementId: "burpee",
                movementStyle: .burpee(.fourCount),
                durationHintSeconds: 60,
                metrics: [
                    .reps(
                        key: "reps",
                        defaultValue: 3,
                        min: 2,
                        max: 10,
                        step: 1,
                        progressable: true
                    )
                ]
            )
        ]
    )

    // GTG: Pull-ups (banded)
    microdoses["gtg_pullup_band"] = MicrodoseDefinition(
        id: "gtg_pullup_band",
        name: "GTG: Banded Pull-ups",
        category: .gtg,
        suggestedDurationSeconds: 30,
        gtgFriendly: true,
        blocks: [
            MicrodoseBlock(
                movementId: "pullup",
                movementStyle: .band(.namedColour("red")),
                durationHintSeconds: 30,
                metrics: [
                    .reps(
                        key: "reps",
                        defaultValue: 3,
                        min: 1,
                        max: 8,
                        step: 1,
                        progressable: true
                    ),
                    .band(
                        key: "band",
                        defaultValue: "red",
                        progressable: false
                    )
                ]
            )
        ]
    )

    // Mobility: Hip CARs
    microdoses["mobility_hip_cars"] = MicrodoseDefinition(
        id: "mobility_hip_cars",
        name: "Hip CARs (3 reps each side)",
        category: .mobility,
        suggestedDurationSeconds: 120,
        gtgFriendly: true,
        blocks: [
            MicrodoseBlock(
                movementId: "hip_cars",
                movementStyle: .none,
                durationHintSeconds: 120,
                metrics: [
                    .reps(
                        key: "reps_per_side",
                        defaultValue: 3,
                        min: 2,
                        max: 5,
                        step: 1,
                        progressable: false
                    )
                ]
            )
        ]
    )

    // Mobility: Shoulder CARs
    microdoses["mobility_shoulder_cars"] = MicrodoseDefinition(
        id: "mobility_shoulder_cars",
        name: "Shoulder CARs (3 reps each side)",
        category: .mobility,
        suggestedDurationSeconds: 120,
        gtgFriendly: true,
        blocks: [
            MicrodoseBlock(
                movementId: "shoulder_cars",
                movementStyle: .none,
                durationHintSeconds: 120,
                metrics: [
                    .reps(
                        key: "reps_per_side",
                        defaultValue: 3,
                        min: 2,
                        max: 5,
                        step: 1,
                        progressable: false
                    )
                ]
            )
        ]
    )

    return Catalog(movements: movements, microdoses: microdoses)
}

// MARK: - Catalog Validation

extension Catalog {
    /// Validate the catalog for consistency and completeness
    ///
    /// Returns a list of validation errors, or empty array if valid.
    public func validate() -> [String] {
        var errors: [String] = []

        // Check for duplicate IDs (already guaranteed by Dictionary, but check for empty IDs)
        for (id, movement) in movements {
            if id.isEmpty || movement.id.isEmpty {
                errors.append("Movement has empty ID")
            }
            if id != movement.id {
                errors.append("Movement key '\(id)' doesn't match movement.id '\(movement.id)'")
            }
            if movement.name.isEmpty {
                errors.append("Movement '\(id)' has empty name")
            }
        }

        for (id, definition) in microdoses {
            if id.isEmpty || definition.id.isEmpty {
                errors.append("Microdose definition has empty ID")
            }
            if id != definition.id {
                errors.append("Microdose key '\(id)' doesn't match definition.id '\(definition.id)'")
            }
            if definition.name.isEmpty {
                errors.append("Microdose '\(id)' has empty name")
            }
            if definition.blocks.isEmpty {
                errors.append("Microdose '\(id)' has no blocks")
            }

            // Check that all referenced movements exist
            for block in definition.blocks {
                if !movements.keys.contains(block.movementId) {
                    errors.append("Microdose '\(id)' references non-existent movement '\(block.movementId)'")
                }

                // Validate metrics
                for metric in block.metrics {
                    switch metric {
                    case .reps(_, let defaultValue, let min, let max, _, _):
                        if defaultValue < min {
                            errors.append("Microdose '\(id)': default reps \(defaultValue) < min \(min)")
                        }
                        if defaultValue > max {
                            errors.append("Microdose '\(id)': default reps \(defaultValue) > max \(max)")
                        }
                        if min > max {
                            errors.append("Microdose '\(id)': min reps \(min) > max \(max)")
                        }
                    case .band(_, let defaultValue, _):
                        if defaultValue.isEmpty {
                            errors.append("Microdose '\(id)': band metric has empty default")
                        }
                    }
                }
            }
        }

        // Check that we have at least one microdose in each category
        let hasVO2 = microdoses.values.contains { $0.category == .vo2 }
        let hasGTG = microdoses.values.contains { $0.category == .gtg }
        let hasMobility = microdoses.values.contains { $0.category == .mobility }

        if !hasVO2 {
            errors.append("Catalog has no VO2 microdoses")
        }
        if !hasGTG {
            errors.append("Catalog has no GTG microdoses")
        }
        if !hasMobility {
            errors.append("Catalog has no Mobility microdoses")
        }

        return errors
    }
}
