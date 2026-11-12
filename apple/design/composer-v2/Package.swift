// swift-tools-version: 6.2
import PackageDescription

let strict: [SwiftSetting] = [.swiftLanguageMode(.v6), .enableExperimentalFeature("StrictConcurrency=complete")]

let package = Package(
    name: "ComposerV2",
    platforms: [.iOS(.v26), .macOS(.v26)],
    dependencies: [.package(name: "Cooee", path: "../..")],
    targets: [
        .target(
            name: "ComposerV2",
            dependencies: [.product(name: "CooeeUI", package: "Cooee")],
            swiftSettings: strict
        ),
        .testTarget(name: "ShotTests", dependencies: ["ComposerV2"], swiftSettings: strict),
    ]
)
