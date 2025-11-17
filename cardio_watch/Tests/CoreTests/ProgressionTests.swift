/// Unit tests for progression logic
/// Ported from Rust `cardio_core/src/progression.rs` tests

import XCTest
@testable import CardioCore

final class ProgressionTests: XCTestCase {

    // MARK: - Burpee Progression Tests

    func testBurpeeRepsProgression() {
        var state = ProgressionState(
            reps: 3,
            style: .burpee(.fourCount),
            level: 0,
            lastUpgraded: nil
        )

        // Should increase reps until ceiling
        for expectedReps in 4...10 {
            upgradeBurpee(state: &state, repCeiling: 10)
            XCTAssertEqual(state.reps, expectedReps)
        }
    }

    func testBurpeeStyleUpgrade() {
        var state = ProgressionState(
            reps: 10,
            style: .burpee(.fourCount),
            level: 7,
            lastUpgraded: nil
        )

        // At ceiling, should upgrade to 6-count
        upgradeBurpee(state: &state, repCeiling: 10)

        if case .burpee(let style) = state.style {
            XCTAssertEqual(style, .sixCount)
        } else {
            XCTFail("Expected burpee style")
        }

        XCTAssertEqual(state.reps, 6) // Reset to lower reps
    }

    func testBurpeeFullProgression() {
        var state = ProgressionState(
            reps: 3,
            style: .burpee(.fourCount),
            level: 0,
            lastUpgraded: nil
        )

        // Progress through all styles
        for _ in 0..<7 {
            upgradeBurpee(state: &state, repCeiling: 10)
        } // 4-count at 10

        upgradeBurpee(state: &state, repCeiling: 10) // Should upgrade to 6-count
        if case .burpee(let style) = state.style {
            XCTAssertEqual(style, .sixCount)
        } else {
            XCTFail("Expected burpee style")
        }

        for _ in 0..<4 {
            upgradeBurpee(state: &state, repCeiling: 10)
        } // 6-count at 10

        upgradeBurpee(state: &state, repCeiling: 10) // Should upgrade to 6-count-2-pump
        if case .burpee(let style) = state.style {
            XCTAssertEqual(style, .sixCountTwoPump)
        } else {
            XCTFail("Expected burpee style")
        }

        for _ in 0..<5 {
            upgradeBurpee(state: &state, repCeiling: 10)
        } // 6-count-2-pump at 10

        upgradeBurpee(state: &state, repCeiling: 10) // Should upgrade to Seal
        if case .burpee(let style) = state.style {
            XCTAssertEqual(style, .seal)
        } else {
            XCTFail("Expected burpee style")
        }
    }

    // MARK: - KB Swing Progression Tests

    func testKBSwingProgression() {
        var state = ProgressionState(
            reps: 5,
            style: .none,
            level: 0,
            lastUpgraded: nil
        )

        upgradeKBSwing(state: &state, baseReps: 5, maxReps: 15)
        XCTAssertEqual(state.reps, 6)
        XCTAssertEqual(state.level, 1)

        upgradeKBSwing(state: &state, baseReps: 5, maxReps: 15)
        XCTAssertEqual(state.reps, 7)
        XCTAssertEqual(state.level, 2)
    }

    func testKBSwingRespectsMax() {
        var state = ProgressionState(
            reps: 14,
            style: .none,
            level: 9,
            lastUpgraded: nil
        )

        upgradeKBSwing(state: &state, baseReps: 5, maxReps: 15)
        XCTAssertEqual(state.reps, 15)

        // Should not exceed max
        upgradeKBSwing(state: &state, baseReps: 5, maxReps: 15)
        XCTAssertEqual(state.reps, 15)
    }

    // MARK: - Pullup Progression Tests

    func testPullupProgression() {
        var state = ProgressionState(
            reps: 3,
            style: .none,
            level: 0,
            lastUpgraded: nil
        )

        for expectedReps in 4...8 {
            upgradePullup(state: &state, maxReps: 8)
            XCTAssertEqual(state.reps, expectedReps)
        }

        // Should not exceed max
        upgradePullup(state: &state, maxReps: 8)
        XCTAssertEqual(state.reps, 8)
    }

    // MARK: - Integration Tests

    func testIncreaseIntensityCreatesState() {
        var userState = UserMicrodoseState()
        let config = ProgressionConfig()

        increaseIntensity(definitionId: "emom_burpee_5m", userState: &userState, config: config)

        XCTAssertTrue(userState.progressions.keys.contains("emom_burpee_5m"))
        if let state = userState.progressions["emom_burpee_5m"] {
            XCTAssertEqual(state.reps, 4) // Started at 3, increased to 4
            XCTAssertEqual(state.level, 1)
        } else {
            XCTFail("Expected progression state to be created")
        }
    }
}
