/// Prescription engine for selecting microdose workouts.
///
/// This module implements the v1.1 prescription logic:
/// - Check strength signal for recent lower-body work
/// - Check time since last VO2 session
/// - Round-robin selection for categories and definitions
///
/// Direct port from Rust `cardio_core/src/engine.rs`

import Foundation

// MARK: - Main Prescription Function

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
public func prescribeNext(
    catalog: Catalog,
    context: UserContext,
    targetCategory: MicrodoseCategory? = nil
) throws -> PrescribedMicrodose {
    // Determine category to prescribe
    var category = targetCategory ?? determineCategory(context: context)

    print("INFO: Prescribing microdose from category: \(category)")

    // Fallback if the determined category doesn't exist in catalog
    // Try in order: suggested → Vo2 → Gtg → Mobility → error
    if !hasCategory(catalog: catalog, category: category) {
        print("WARN: Category \(category) not found in catalog, trying fallbacks")

        let fallbackOrder: [MicrodoseCategory] = [.vo2, .gtg, .mobility]

        guard let fallback = fallbackOrder.first(where: { hasCategory(catalog: catalog, category: $0) }) else {
            throw EngineError.noCategoriesAvailable
        }

        category = fallback
        print("INFO: Using fallback category: \(category)")
    }

    // Select definition from category
    let definition = try selectDefinitionFromCategory(
        catalog: catalog,
        context: context,
        category: category
    )

    // Compute intensity based on progression state
    let (reps, style) = computeIntensity(definition: definition, context: context)

    return PrescribedMicrodose(definition: definition, reps: reps, style: style)
}

// MARK: - Category Selection

/// Determine which category to prescribe from based on context
private func determineCategory(context: UserContext) -> MicrodoseCategory {
    // Rule 1: Recent lower-body strength → prefer GTG or Mobility
    if let strength = context.externalStrength {
        let timeSinceStrength = context.now.timeIntervalSince(strength.lastSessionAt)
        let hoursSinceStrength = timeSinceStrength / 3600

        if timeSinceStrength < 24 * 3600 && strength.sessionType == .lower {
            print("INFO: Recent lower-body strength detected (\(Int(hoursSinceStrength)) hours ago), preferring GTG/Mobility")
            return .gtg
        }
    }

    // Rule 2: Check time since last VO2 session
    if let lastVO2 = findLastSessionByCategory(sessions: context.recentSessions, category: "vo2") {
        let timeSinceVO2 = context.now.timeIntervalSince(lastVO2.timestamp)
        let hoursSinceVO2 = timeSinceVO2 / 3600

        if timeSinceVO2 > 4 * 3600 {
            print("INFO: Last VO2 session was \(Int(hoursSinceVO2)) hours ago (> 4h), prescribing VO2")
            return .vo2
        }
    }
    // If no VO2 in history, fall through to round-robin

    // Rule 3: Default round-robin based on last category
    let lastCategory = context.recentSessions.first.flatMap { session -> MicrodoseCategory? in
        // Infer category from definition ID
        let defId = session.definitionId
        if defId.contains("vo2") || defId.contains("emom") {
            return .vo2
        } else if defId.contains("gtg") {
            return .gtg
        } else if defId.contains("mobility") {
            return .mobility
        } else {
            return nil
        }
    }

    let nextCategory = lastCategory?.next() ?? .vo2 // Default to VO2 if unknown

    print("INFO: Round-robin selection: \(nextCategory)")
    return nextCategory
}

/// Helper to check if a catalog has any microdoses in a category
private func hasCategory(catalog: Catalog, category: MicrodoseCategory) -> Bool {
    return catalog.microdoses.values.contains { $0.category == category }
}

// MARK: - Definition Selection

/// Select a specific definition from a category
private func selectDefinitionFromCategory(
    catalog: Catalog,
    context: UserContext,
    category: MicrodoseCategory
) throws -> MicrodoseDefinition {
    // Get all definitions in the category
    var candidates = catalog.microdoses.values.filter { $0.category == category }

    guard !candidates.isEmpty else {
        throw EngineError.noCandidatesInCategory(category)
    }

    // Sort for deterministic selection
    candidates.sort { $0.id < $1.id }

    // Handle category-specific selection logic
    switch category {
    case .vo2:
        // Round-robin between VO2 definitions
        let lastVO2Def = context.recentSessions
            .first { session in
                let defId = session.definitionId
                return defId.contains("vo2") || defId.contains("emom")
            }
            .map { $0.definitionId }

        // Pick the one we didn't do last time
        if let last = lastVO2Def,
           let different = candidates.first(where: { $0.id != last }) {
            return different
        } else {
            // No previous VO2 or all are the same, pick first
            return candidates[0]
        }

    case .gtg:
        // Just pick the first (only one GTG definition in default catalog)
        return candidates[0]

    case .mobility:
        // Round-robin through mobility definitions
        if let last = context.userState.lastMobilityDefId {
            // Find next in sequence
            if let lastIdx = candidates.firstIndex(where: { $0.id == last }) {
                let nextIdx = (lastIdx + 1) % candidates.count
                return candidates[nextIdx]
            } else {
                return candidates[0]
            }
        } else {
            return candidates[0]
        }
    }
}

// MARK: - Intensity Computation

/// Compute intensity (reps/style) based on progression state
private func computeIntensity(
    definition: MicrodoseDefinition,
    context: UserContext
) -> (Int, MovementStyle) {
    if let state = context.userState.progressions[definition.id] {
        return (state.reps, state.style)
    } else {
        // No progression state - use defaults from definition
        let firstBlock = definition.blocks.first
        let defaultReps = firstBlock?.metrics.compactMap { metric -> Int? in
            if case .reps(_, let defaultValue, _, _, _, _) = metric {
                return defaultValue
            }
            return nil
        }.first ?? 3

        let defaultStyle = firstBlock?.movementStyle ?? .none

        return (defaultReps, defaultStyle)
    }
}

// MARK: - Helper Functions

/// Find the last session that matches a category string
private func findLastSessionByCategory(sessions: [SessionKind], category: String) -> SessionKind? {
    // Sessions should already be sorted newest first
    return sessions.first { $0.definitionId.contains(category) }
}

// MARK: - Error Types

public enum EngineError: Error, LocalizedError {
    case noCategoriesAvailable
    case noCandidatesInCategory(MicrodoseCategory)

    public var errorDescription: String? {
        switch self {
        case .noCategoriesAvailable:
            return "No microdoses available in catalog"
        case .noCandidatesInCategory(let category):
            return "No microdoses found in category \(category)"
        }
    }
}
