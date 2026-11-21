Closes #249

Replaces #259 (DECISION q1680: a fresh branch whose commit 1 is a test that fails, with no force-push).

## For humans

When no persona is pinned, Home no longer leaves an empty band above the chat list.
In the Terminal look, the bare "# pinned" heading is gone too.
With pinned personas, the row looks the same as before.

## What changed

- `apple/Tests/CooeeUITests/PinnedRowTests.swift` (commit 1, `d9719d32`, test only): for each look, the test renders the real `HomeView` and requires the chats to start less than 48pt below where the pinned row would start when nothing is pinned.
- `apple/Sources/CooeeUI/HomeRows.swift` (commit 2, `5e993df9`): `PinnedRow` draws nothing when `personas` is empty, in all three looks. The Terminal `# pinned` heading is inside the row, so it disappears with it. Home.swift is unchanged.

The diff is the same two-file, +38-line change that #259 had, rebuilt on current `origin/staging`.

src/ net +4 (budget not stated): one empty-check guard in the row's body.

## Repro

Command: `cd apple && swift test --filter PinnedRowTests`

- Base plus commit 1 (`d9719d32`): exit 1. `Expectation failed: chatsTop - pinnedTop < 48` for `.modern`, `.scene` and `.terminal`.
- Head `5e993df9`: exit 0, `✔ Suite PinnedRowTests passed`.

## Evidence at head 5e993df9259839f758dfb82d336dc7fe01cdbc31

- `cd apple && swift test --filter PinnedRowTests` → exit 1 at `d9719d32`, exit 0 at `5e993df9`.
- `bash scripts/apple-check.sh`: format, anti-slop, TODO check, SwiftLint, the Swift 6 build, every test suite (including SnapshotTests) and coverage (Core 98.0%, API 97.1%, Features 97.3%) all passed. The run then exited 139 because `swiftlint analyze` (the unused-import step) segfaulted while reading the CasePaths module. That is a crash in the tool, not a lint finding, and this diff adds no imports. A retry 2 minutes later crashed the same way, so I'm leaving the unused-import and Periphery steps to CI. Both crashes happen while reading modules built against SDK 26B118 with the 27.2 SDK, which points to a mismatch on this host.
