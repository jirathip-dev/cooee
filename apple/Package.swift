// swift-tools-version: 6.2
// #124: the native app's views and fixtures (CooeeUI), and `shots`, which renders each design frame to a PNG with
// SwiftUI's ImageRenderer. The Xcode project (Cooee.xcodeproj) builds the iOS and macOS app on top of CooeeUI.
import PackageDescription

let package = Package(
    name: "CooeeUI",
    platforms: [.iOS(.v26), .macOS(.v26)],
    products: [.library(name: "CooeeUI", targets: ["CooeeUI"])],
    targets: [
        .target(name: "CooeeUI", resources: [.copy("Sprites")]),
        .executableTarget(name: "shots", dependencies: ["CooeeUI"]),
    ]
)
