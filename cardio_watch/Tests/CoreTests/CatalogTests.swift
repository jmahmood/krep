/// Unit tests for catalog functionality
/// Ported from Rust `cardio_core/src/catalog.rs` tests

import XCTest
@testable import CardioCore

final class CatalogTests: XCTestCase {

    func testCatalogLoads() {
        let catalog = buildDefaultCatalog()
        XCTAssertEqual(catalog.movements.count, 5)
        XCTAssertEqual(catalog.microdoses.count, 5)
    }

    func testAllReferencedMovementsExist() {
        let catalog = buildDefaultCatalog()
        for definition in catalog.microdoses.values {
            for block in definition.blocks {
                XCTAssertTrue(
                    catalog.movements.keys.contains(block.movementId),
                    "Movement \(block.movementId) referenced but not found"
                )
            }
        }
    }

    func testVO2CategoryExists() {
        let catalog = buildDefaultCatalog()
        let vo2Count = catalog.microdoses.values.filter { $0.category == .vo2 }.count
        XCTAssertGreaterThanOrEqual(vo2Count, 2, "Should have at least 2 VO2 workouts")
    }

    func testGTGCategoryExists() {
        let catalog = buildDefaultCatalog()
        let gtgCount = catalog.microdoses.values.filter { $0.category == .gtg }.count
        XCTAssertGreaterThanOrEqual(gtgCount, 1, "Should have at least 1 GTG workout")
    }

    func testMobilityCategoryExists() {
        let catalog = buildDefaultCatalog()
        let mobilityCount = catalog.microdoses.values.filter { $0.category == .mobility }.count
        XCTAssertGreaterThanOrEqual(mobilityCount, 2, "Should have at least 2 mobility workouts")
    }

    func testDefaultCatalogValidates() {
        let catalog = buildDefaultCatalog()
        let errors = catalog.validate()
        XCTAssertTrue(
            errors.isEmpty,
            "Default catalog has validation errors: \(errors)"
        )
    }
}
