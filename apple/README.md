# cooee for iPhone, iPad and Mac (#124)

One multiplatform SwiftUI app (iOS 26, macOS 26, q948) against a paired cooee server (#143).

The Swift package (`Package.swift`) is layered (#147); each target imports only the layers before it:

- `CooeeCore`: the models and the pure logic that draws them (Foundation only).
- `CooeeAPI`: the client for `docs/api.md`, the SSE stream, the Keychain and Bonjour.
- `CooeeFeatures`: one TCA reducer per screen and the `@Dependency` clients they call.
- `CooeeUI`: the SwiftUI views, the three styles (Modern, Terminal, Scene), the farm palette and the design fixtures.
- `CooeeApp`: the composition root: the clients' live values, built from `CooeeAPI`.
- `Cooee.xcodeproj` + `Cooee/`: the app target on top of `CooeeApp`.

Design frames: `CooeeUITests`' snapshot test renders each one and compares it with `design/v1/`;
`COOEE_RECORD=1 swift test --filter SnapshotTests` writes them again. `design/sprites.mjs` rasterises the farm pack's
horses into `Sources/CooeeUI/Sprites/` (run from the repo root with a chrome-headless-shell path).

Build: `xcodebuild -project apple/Cooee.xcodeproj -scheme Cooee -destination 'generic/platform=iOS Simulator'
CODE_SIGNING_ALLOWED=NO build` (and `generic/platform=macOS`). CI runs both (`.github/workflows/apple.yml`).

TestFlight: CI uploads each `staging` change to `apple/` (`.github/workflows/apple-testflight.yml`); see `docs/testflight.md`.

## Rules

The checks are the rules: `just apple-check` (`scripts/apple-check.sh`, CI job `apple-quality`) runs `swift format`
(`.swift-format`), anti-slop (`tools/anti-slop-swift`, cooee's rules in `CooeeRules.swift`), SwiftLint
(`.swiftlint.yml`), the tests, coverage (`scripts/apple-coverage.swift`) and Periphery (`.periphery.yml`).
`.herdsman/rules.md` lists them in one line each.
