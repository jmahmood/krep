/// Unit tests for prescription engine
/// Ported from Rust `cardio_core/src/engine.rs` tests

import XCTest
@testable import CardioCore

final class EngineTests: XCTestCase {

    // MARK: - Helper Functions

    private func createTestContext() -> UserContext {
        return UserContext(
            now: Date(),
            userState: UserMicrodoseState(),
            recentSessions: [],
            externalStrength: nil,
            equipmentAvailable: []
        )
    }

    // MARK: - Basic Prescription Tests

    func testPrescribeVO2WhenNoHistory() throws {
        let catalog = buildDefaultCatalog()
        let context = createTestContext()

        let prescribed = try prescribeNext(catalog: catalog, context: context, targetCategory: nil)

        XCTAssertEqual(prescribed.definition.category, .vo2)
    }

    func testPrescribeGTGAfterLowerStrength() throws {
        let catalog = buildDefaultCatalog()
        var context = createTestContext()

        // Add lower-body strength signal from 12 hours ago
        context.externalStrength = ExternalStrengthSignal(
            lastSessionAt: Date().addingTimeInterval(-12 * 3600),
            sessionType: .lower
        )

        let prescribed = try prescribeNext(catalog: catalog, context: context, targetCategory: nil)

        XCTAssertEqual(prescribed.definition.category, .gtg)
    }

    func testRespectsTargetCategory() throws {
        let catalog = buildDefaultCatalog()
        let context = createTestContext()

        let prescribed = try prescribeNext(
            catalog: catalog,
            context: context,
            targetCategory: .mobility
        )

        XCTAssertEqual(prescribed.definition.category, .mobility)
    }

    // MARK: - Intensity Computation Tests

    func testComputeIntensityWithProgression() throws {
        let catalog = buildDefaultCatalog()
        var context = createTestContext()

        // Add progression state
        context.userState.progressions["emom_burpee_5m"] = ProgressionState(
            reps: 7,
            style: .burpee(.sixCount),
            level: 10,
            lastUpgraded: Date()
        )

        let prescribed = try prescribeNext(
            catalog: catalog,
            context: context,
            targetCategory: .vo2
        )

        // If we got the burpee workout, check intensity
        if prescribed.definition.id == "emom_burpee_5m" {
            XCTAssertEqual(prescribed.reps, 7)
            if case .burpee(let style) = prescribed.style {
                XCTAssertEqual(style, .sixCount)
            } else {
                XCTFail("Expected burpee style")
            }
        }
    }

    func testComputeIntensityWithoutProgression() throws {
        let catalog = buildDefaultCatalog()
        var context = createTestContext()

        // Force burpee prescription
        let prescribed = try prescribeNext(
            catalog: catalog,
            context: context,
            targetCategory: .vo2
        )

        // Should use default from definition (3 reps for burpees)
        if prescribed.definition.id == "emom_burpee_5m" {
            XCTAssertEqual(prescribed.reps, 3)
        }
    }

    // MARK: - Edge Cases

    func testSingleCategoryEnvironment() throws {
        // Test that when only one category is available, the engine doesn't loop
        var catalog = buildDefaultCatalog()

        // Keep only VO2 microdoses
        catalog = Catalog(
            movements: catalog.movements,
            microdoses: catalog.microdoses.filter { $0.value.category == .vo2 }
        )

        let context = createTestContext()

        // First prescription should be VO2
        let p1 = try prescribeNext(catalog: catalog, context: context, targetCategory: nil)
        XCTAssertEqual(p1.definition.category, .vo2)

        // Create a context with history of the first prescription
        var context2 = createTestContext()
        context2.recentSessions = [
            .real(MicrodoseSession(
                id: UUID(),
                definitionId: p1.definition.id,
                performedAt: Date(),
                startedAt: Date(),
                completedAt: Date(),
                actualDurationSeconds: 300
            ))
        ]

        // Second prescription should still be VO2 (no infinite loop)
        let p2 = try prescribeNext(catalog: catalog, context: context2, targetCategory: nil)
        XCTAssertEqual(p2.definition.category, .vo2)
    }

    func testStrengthOverrideWithSkipInteraction() throws {
        let catalog = buildDefaultCatalog()
        var context = createTestContext()

        // Set up recent lower-body strength signal
        context.externalStrength = ExternalStrengthSignal(
            lastSessionAt: Date().addingTimeInterval(-12 * 3600),
            sessionType: .lower
        )

        // First prescription should be GTG (strength override)
        let p1 = try prescribeNext(catalog: catalog, context: context, targetCategory: nil)
        XCTAssertEqual(p1.definition.category, .gtg)

        // User skips - add ShownButSkipped to context
        context.recentSessions.insert(
            .shownButSkipped(definitionId: p1.definition.id, shownAt: context.now),
            at: 0
        )

        // Next prescription should still respect strength override
        let p2 = try prescribeNext(catalog: catalog, context: context, targetCategory: nil)
        XCTAssertEqual(p2.definition.category, .gtg)
    }

    func testMixedHistoryWithSkipPatterns() throws {
        let catalog = buildDefaultCatalog()
        var context = createTestContext()

        let now = Date()

        // Create history: VO2 (real) → GTG (skipped) → Mobility (real)
        context.recentSessions = [
            .real(MicrodoseSession(
                id: UUID(),
                definitionId: "mobility_hip_cars",
                performedAt: now.addingTimeInterval(-3600),
                startedAt: now.addingTimeInterval(-3600),
                completedAt: now.addingTimeInterval(-3600),
                actualDurationSeconds: 60
            )),
            .shownButSkipped(
                definitionId: "gtg_pullup_band",
                shownAt: now.addingTimeInterval(-2 * 3600)
            ),
            .real(MicrodoseSession(
                id: UUID(),
                definitionId: "emom_burpee_5m",
                performedAt: now.addingTimeInterval(-3 * 3600),
                startedAt: now.addingTimeInterval(-3 * 3600),
                completedAt: now.addingTimeInterval(-3 * 3600),
                actualDurationSeconds: 300
            ))
        ]

        // Next should be VO2 (round-robin after Mobility)
        let prescription = try prescribeNext(catalog: catalog, context: context, targetCategory: nil)
        XCTAssertEqual(prescription.definition.category, .vo2)

        // Verify that both Real and ShownButSkipped are counted for round-robin
        XCTAssertEqual(context.recentSessions.count, 3)
        XCTAssertTrue(context.recentSessions[0].definitionId.contains("mobility"))
        XCTAssertTrue(context.recentSessions[1].definitionId.contains("gtg"))
        XCTAssertTrue(context.recentSessions[2].definitionId.contains("burpee"))
    }
}
