// swift-tools-version: 5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "CardioWatch",
    platforms: [
        .watchOS(.v10),
        .iOS(.v17)
    ],
    products: [
        // Core library shared between watchOS and iOS
        .library(
            name: "CardioCore",
            targets: ["CardioCore"]),
    ],
    dependencies: [
        // No external dependencies for MVP
        // Future: Add SwiftUI Charts for analytics
    ],
    targets: [
        // Core business logic (port of cardio_core Rust crate)
        .target(
            name: "CardioCore",
            dependencies: [],
            path: "Sources/CardioCore"
        ),

        // Unit tests for core logic
        .testTarget(
            name: "CoreTests",
            dependencies: ["CardioCore"],
            path: "Tests/CoreTests"
        ),
    ]
)
