// swift-tools-version: 6.2
import PackageDescription

let strict: [SwiftSetting] =
    [.swiftLanguageMode(.v6), .enableExperimentalFeature("StrictConcurrency=complete")]
    + (Context.environment["COOEE_STRICT"] == "1" ? [.treatAllWarnings(as: .error)] : [])
let tca = Target.Dependency.product(name: "ComposableArchitecture", package: "swift-composable-architecture")

let package = Package(
    name: "Cooee",
    platforms: [.iOS(.v26), .macOS(.v26)],
    products: [
        .library(name: "CooeeApp", targets: ["CooeeApp"]),
        .library(name: "CooeeUI", targets: ["CooeeUI"]),
    ],
    dependencies: [
        .package(url: "https://github.com/pointfreeco/swift-composable-architecture", exact: "1.26.2")
    ],
    targets: [
        .target(name: "CooeeCore", swiftSettings: strict),
        .target(name: "CooeeAPI", dependencies: ["CooeeCore"], swiftSettings: strict),
        .target(name: "CooeeFeatures", dependencies: ["CooeeCore", "CooeeAPI", tca], swiftSettings: strict),
        .target(
            name: "CooeeUI",
            dependencies: ["CooeeCore", "CooeeFeatures", tca],
            resources: [.copy("Sprites"), .copy("Default")],
            swiftSettings: strict
        ),
        .target(
            name: "CooeeApp",
            dependencies: ["CooeeCore", "CooeeAPI", "CooeeFeatures", "CooeeUI", tca],
            swiftSettings: strict
        ),
        .testTarget(
            name: "CooeeCoreTests", dependencies: ["CooeeCore"], resources: [.copy("Fixtures")], swiftSettings: strict),
        .testTarget(
            name: "CooeeAPITests", dependencies: ["CooeeAPI"], resources: [.copy("Fixtures")], swiftSettings: strict),
        .testTarget(name: "CooeeFeaturesTests", dependencies: ["CooeeFeatures", tca], swiftSettings: strict),
        .testTarget(name: "CooeeUITests", dependencies: ["CooeeCore", "CooeeUI"], swiftSettings: strict),
    ]
)
