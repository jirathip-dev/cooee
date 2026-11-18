# cooee for iPhone, iPad and Mac (#124)

One multiplatform SwiftUI app (iOS 26, macOS 26, q948) against a paired cooee server (#143).

The Swift package (`Package.swift`) is layered (#147); each target imports only the layers before it:

- `CooeeCore`: the models and the pure logic that draws them (Foundation only).
- `CooeeAPI`: the client for the API (`docs/README.md` and each area's doc), the SSE stream, the Keychain and Bonjour.
- `CooeeFeatures`: one TCA reducer per screen and the `@Dependency` clients they call.
- `CooeeUI`: the SwiftUI views, the three styles (Modern, Terminal, Scene), the farm palette and the design fixtures.
- `CooeeApp`: the composition root: the clients' live values, built from `CooeeAPI`.
- `Cooee.xcodeproj` + `Cooee/`: the app target on top of `CooeeApp`.

Design frames: `CooeeUITests`' snapshot test renders each one and compares it with `design/v1/`;
`COOEE_RECORD=1 swift test --filter SnapshotTests` writes them again. `design/sprites.mjs` rasterises the farm pack's
horses into `Sources/CooeeUI/Sprites/` (run from the repo root with a chrome-headless-shell path).

Build: `xcodebuild -project apple/Cooee.xcodeproj -scheme Cooee -destination 'generic/platform=iOS Simulator'
CODE_SIGNING_ALLOWED=NO build` (and `generic/platform=macOS`). CI runs both (`.github/workflows/apple.yml`).

The Mac app is a client only (#484): its first run is the iPhone's pairing flow (Nearby, an address or a code, then
waiting for approval). A cooee answering on this Mac's port (`~/.config/cooee/cooee/config.toml`, default 4747) is listed
first in Nearby, as This Mac. The server on a Mac is the CLI install (`cooee setup`), as on Linux; approve the app with
`cooee pair approve <id>` or from a paired device. An older app that ran the server as login items
(`com.jirathip.cooee.serve`, `.shed`) has them unregistered on the new version's first launch, which says so once;
`cooee setup` then reuses the same config and data folders.

TestFlight: a merge to `main` that touches `apple/` uploads the iPhone and the Mac app, sandboxed
(`Cooee-mac.entitlements`), in one run of `.github/workflows/apple-testflight.yml`; see `docs/testflight.md`.

## Rules

The checks are the rules: `just apple-check` (`scripts/apple-check.sh`, in CI job `apple`) runs `swift format`
(`.swift-format`), anti-slop (`tools/anti-slop-swift`, cooee's rules in `CooeeRules.swift`), SwiftLint
(`.swiftlint.yml`), the tests, coverage (`scripts/apple-coverage.swift`) and Periphery (`.periphery.yml`).
`.herdsman/rules.md` lists them in one line each.
