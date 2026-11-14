// swift-tools-version: 6.2
import PackageDescription

let package = Package(
    name: "ChatFormatDesign",
    platforms: [.iOS(.v26), .macOS(.v26)],
    targets: [
        .target(name: "ChatFormat", swiftSettings: [.swiftLanguageMode(.v6)]),
        .executableTarget(name: "shots", dependencies: ["ChatFormat"], swiftSettings: [.swiftLanguageMode(.v6)]),
    ]
)
