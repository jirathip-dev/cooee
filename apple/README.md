# cooee for iPhone, iPad and Mac (#124)

One multiplatform SwiftUI app (iOS 26, macOS 26, q948). For now it has views and fixtures only, with no API client.

- `CooeeUI` (Swift package, `Package.swift`, `Sources/CooeeUI/`): every screen, the three styles (Modern, Terminal,
  Scene), the farm palette, and fixtures shaped like `docs/api.md`. No new dependency.
- `Cooee.xcodeproj` + `Cooee/`: the app target (`CooeeApp`, `RootView`, the Xcode previews) on top of `CooeeUI`.
- `shots` (`Sources/shots/`): renders each design frame to `design/v1/` with `ImageRenderer`: `swift run shots`.
- `design/sprites.mjs`: rasterises the farm pack's horses into `Sources/CooeeUI/Sprites/` (run from the repo root
  with a chrome-headless-shell path).

Build: `xcodebuild -project apple/Cooee.xcodeproj -scheme Cooee -destination 'generic/platform=iOS Simulator'
CODE_SIGNING_ALLOWED=NO build` (and `generic/platform=macOS`). CI runs both (`.github/workflows/apple.yml`).

TestFlight: CI uploads each `staging` change to `apple/` (`.github/workflows/apple-testflight.yml`); see `docs/testflight.md`.
