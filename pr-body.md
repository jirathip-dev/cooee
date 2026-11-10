Design for #124: the native cooee app's v1 screens as real SwiftUI views in `apple/` (one multiplatform target, iOS 26 / macOS 26), fed by fixture data. No networking.

- 38 frames in `apple/design/v1/`, with a README that maps each frame to the decision it shows. There's also an HTML overview in `docs/design/native-v1/`.
- `.github/workflows/apple.yml` runs `xcodebuild` for the iOS Simulator and macOS on a GitHub-hosted macOS runner. That job is how acceptance criterion 1 gets proved, because this host has no Xcode.app.
- The PNGs are `ImageRenderer` stills of the real views, so Liquid Glass is a stand-in.

Design review: owner approved (q987). This round only merges current `staging` (13 commits, no conflicts).

## Evidence (head e434a92041d5fe9c66bdb05e72a9701be58805ba)

CI at this head, all SUCCESS: `build (generic/platform=iOS Simulator)`, `build (generic/platform=macOS)` (`xcodebuild -project Cooee.xcodeproj -scheme Cooee -destination …`), msrv/stable on ubuntu and macos, ts, no-node, ui.
I didn't run a local `xcodebuild` because this host only has Command Line Tools.

src/ net +0 (budget n/a): `apple/` sits outside `crates/*/src` and `web/src`, so the line cap total doesn't change.

## For humans

The native iPhone and Mac app's first screens (pairing, home, chat, persona sheet, avatar menu, and the three looks) are now in the repo, matching what you approved.
They are views only for now. Nothing connects to a server yet; the build issues start from these.

Closes #124
