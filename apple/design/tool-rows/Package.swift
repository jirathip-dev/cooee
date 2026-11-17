// swift-tools-version: 6.2
import PackageDescription

let package = Package(
    name: "ToolRowsDesign",
    platforms: [.iOS(.v26), .macOS(.v26)],
    targets: [
        .target(name: "ToolRows", swiftSettings: [.swiftLanguageMode(.v6)]),
        .executableTarget(name: "shots", dependencies: ["ToolRows"], swiftSettings: [.swiftLanguageMode(.v6)]),
    ]
)
