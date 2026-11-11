Closes #249

## For humans

When no persona is pinned, Home no longer leaves an empty band above the chat list.
In the Terminal look, the bare "# pinned" heading is gone too.
With pinned personas, the row looks the same as before.

## What changed

- `apple/Sources/CooeeUI/HomeRows.swift`: `PinnedRow` draws nothing when `personas` is empty, in all three looks. The Terminal heading lives inside the row, so it goes with it. Home.swift is unchanged and keeps one call site.
- `apple/Tests/CooeeUITests/PinnedRowTests.swift` (commit 1, the repro test): for each `LookStyle`, it renders the real `HomeView` three ways: pinned plus chats, unpinned plus chats, and unpinned with no chats. It finds where the pinned row starts and where the chats start, then requires the gap to be under 48pt. Before the fix, the empty row (180pt in Scene, the heading plus strip in Terminal, the strip in Modern) sits in that gap.

src/ net +4 (budget not stated): a single empty guard in the row's body.

## Repro

Test command: `cd apple && swift test --filter PinnedRowTests`
- Commit 1 (`145ea374`, test only): expected to fail on every look.
- Head (`36106488`): expected to pass.

Both CI runs (`apple`/`apple-mac` on each sha) were still **queued** with no runner when this was written, so neither result exists yet. This host has no Xcode, so the tests can't run here (see below). The reviewer should read the two CI results for the repro.

## Review notes (non-blocking)

- Padding at `Home.swift:39` / `Wide.swift:80-81`: when nothing is pinned, `PinnedRow`'s body is an empty optional view. A parent stack lays out no child for an empty view, so the `.padding`/`.scaleEffect` modifiers on it add no height. `PinnedRowTests` measures the rendered `HomeView`, not the code, so a leftover gap would show up there.

## Evidence at head 36106488b0a1fdbea202ee8ff78503601c9d887e

- `bash scripts/apple-check.sh` → exit 0 (swift format, anti-slop, TODO check, SwiftLint). It skipped build, tests, coverage and Periphery because there is no Xcode on this host.
- `cd apple && swift test --filter "PinnedRowTests|SnapshotTests"` → exit 1. It fails while building a dependency, before any test runs: `plugin for module 'SwiftUIMacros' not found` (Command Line Tools only, no Xcode). Not a result for this change.
- CI at this head: queued, not yet run.
