/// Progression logic for increasing workout intensity.
///
/// This module implements the progression rules for different movement types:
/// - Burpees: Reps increase to ceiling, then style upgrades
/// - KB swings: Linear rep progression with configurable max
/// - Pullups: Rep progression (band selection is manual)
///
/// Direct port from Rust `cardio_core/src/progression.rs`

import Foundation

// MARK: - Movement-Specific Progression

/// Upgrade burpee intensity based on current state
///
/// Progression rules:
/// 1. Increase reps until ceiling (default 10)
/// 2. Then upgrade style and reset reps
/// 3. Style progression: 4-count → 6-count → 6-count-2-pump → seal
public func upgradeBurpee(state: inout ProgressionState, repCeiling: Int) {
    // If we haven't hit the ceiling, just increment reps
    if state.reps < repCeiling {
        state.reps += 1
        state.level += 1
        state.lastUpgraded = Date()
        print("DEBUG: Burpee progression: increased reps to \(state.reps)")
        return
    }

    // At ceiling - upgrade style and reset reps
    let (newStyle, newReps): (MovementStyle, Int)

    switch state.style {
    case .burpee(.fourCount):
        newStyle = .burpee(.sixCount)
        newReps = 6
    case .burpee(.sixCount):
        newStyle = .burpee(.sixCountTwoPump)
        newReps = 5
    case .burpee(.sixCountTwoPump):
        newStyle = .burpee(.seal)
        newReps = 4
    case .burpee(.seal):
        // Max level - just stay at ceiling
        state.reps = repCeiling
        state.level += 1
        state.lastUpgraded = Date()
        print("DEBUG: Burpee progression: at max level (Seal @ \(repCeiling))")
        return
    default:
        // Shouldn't happen, but default to 4-count
        newStyle = .burpee(.fourCount)
        newReps = 3
    }

    state.style = newStyle
    state.reps = newReps
    state.level += 1
    state.lastUpgraded = Date()

    print("DEBUG: Burpee progression: upgraded style to \(newStyle), reset reps to \(newReps)")
}

/// Upgrade KB swing intensity (simple linear progression)
///
/// Progression: base_reps + level, capped at max_reps
public func upgradeKBSwing(state: inout ProgressionState, baseReps: Int, maxReps: Int) {
    if state.reps < maxReps {
        state.reps = min(baseReps + Int(state.level) + 1, maxReps)
        state.level += 1
        state.lastUpgraded = Date()
        print("DEBUG: KB swing progression: increased to \(state.reps) reps")
    } else {
        print("DEBUG: KB swing progression: already at max (\(maxReps) reps)")
    }
}

/// Upgrade pullup GTG intensity (simple rep progression)
///
/// Progression: Increase reps up to a ceiling
/// Band selection is manual (user decides when to reduce assistance)
public func upgradePullup(state: inout ProgressionState, maxReps: Int) {
    if state.reps < maxReps {
        state.reps += 1
        state.level += 1
        state.lastUpgraded = Date()
        print("DEBUG: Pullup progression: increased to \(state.reps) reps")
    } else {
        print("DEBUG: Pullup progression: already at max (\(maxReps) reps)")
    }
}

// MARK: - Main Entry Point

/// Upgrade intensity for a specific microdose definition
///
/// This is the main entry point for progression upgrades.
public func increaseIntensity(
    definitionId: String,
    userState: inout UserMicrodoseState,
    config: ProgressionConfig = ProgressionConfig()
) {
    // Get or create progression state
    if userState.progressions[definitionId] == nil {
        // Initialize based on definition type
        let (reps, style): (Int, MovementStyle)

        switch definitionId {
        case "emom_burpee_5m":
            reps = 3
            style = .burpee(.fourCount)
        case "emom_kb_swing_5m":
            reps = 5
            style = .none
        case "gtg_pullup_band":
            reps = 3
            style = .none
        default:
            reps = 3
            style = .none
        }

        userState.progressions[definitionId] = ProgressionState(
            reps: reps,
            style: style,
            level: 0,
            lastUpgraded: nil
        )
    }

    // Apply progression rules based on definition ID
    guard var state = userState.progressions[definitionId] else {
        print("WARN: Failed to get progression state for \(definitionId)")
        return
    }

    switch definitionId {
    case "emom_burpee_5m":
        upgradeBurpee(state: &state, repCeiling: config.burpeeRepCeiling)
    case "emom_kb_swing_5m":
        upgradeKBSwing(state: &state, baseReps: 5, maxReps: config.kbSwingMaxReps)
    case "gtg_pullup_band":
        upgradePullup(state: &state, maxReps: 8)
    default:
        print("WARN: Unknown definition ID for progression: \(definitionId)")
    }

    // Save back to user state
    userState.progressions[definitionId] = state

    print("INFO: Increased intensity for \(definitionId): level \(state.level), \(state.reps) reps")
}

// MARK: - Configuration

/// Configuration for progression rules
public struct ProgressionConfig {
    public let burpeeRepCeiling: Int
    public let kbSwingMaxReps: Int

    public init(burpeeRepCeiling: Int = 10, kbSwingMaxReps: Int = 15) {
        self.burpeeRepCeiling = burpeeRepCeiling
        self.kbSwingMaxReps = kbSwingMaxReps
    }
}
