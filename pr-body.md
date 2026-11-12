Closes #249

## For humans

When no persona is pinned, Home no longer leaves an empty band above the chat list.
In the Terminal look, the bare "# pinned" heading is gone too.
With pinned personas, the row looks the same as before.

## What changed

- `apple/Sources/CooeeUI/HomeRows.swift`: `PinnedRow` draws nothing when `personas` is empty, in all three looks. The Terminal heading is inside the row, so it disappears with it. Home.swift is unchanged and keeps one call site.
- `apple/Tests/CooeeUITests/PinnedRowTests.swift`: for each `LookStyle`, the test renders the real `HomeView` in three states: pinned with chats, unpinned with chats, and unpinned with no chats. It finds where the pinned row starts and where the chats start, and requires the gap to be under 48pt. Before the fix, the empty row (96–180pt depending on the look) filled that gap.
  - Commit 1 `145ea374` is the repro test.
  - `8711a311` only splits one expression so Xcode's type checker accepts it.
  - `137abba4` makes the test find content edges on drawn pixels instead of the backdrop's shading, so the measurement doesn't depend on the look's background. The assertion and the 48pt bound are unchanged.

src/ net +4 (budget not stated): one empty-check guard in the row's body.

## Repro

Command: `cd apple && swift test --filter PinnedRowTests`

- Without the fix: the head's test, with `HomeRows.swift` taken from `e42b69bc` (the fix backed out), in a scratch checkout. Exit 1, all three looks fail: `Expectation failed: chatsTop - pinnedTop < 48` for `.modern`, `.scene` and `.terminal`.
- At head `137abba4`: exit 0, `✔ Test withNothingPinnedTheChatsSitUnderTheTopBar(_:) with 3 test cases passed`.

## Evidence at head 137abba4503efa431a0edfeb97cab8b409645916

- `cd apple && swift test --filter PinnedRowTests` → exit 0 (3 of 3 looks pass).
- Same command, with the fix backed out as described above → exit 1 (3 of 3 looks fail).
- `bash scripts/apple-check.sh` at this head: swift format, anti-slop, the Swift 6 build, every Swift test suite (10 + 50 + 33 + 34 tests, all passing) and coverage (CooeeCore 98.1%, CooeeAPI 97.2%, CooeeFeatures 97.5%, minimum 95%) all passed. Its last step, `swiftlint analyze` (the unused-import check), was still running on this busy host when I published this, and it prints macro-plugin errors from `AppFeature.swift`, a file this PR doesn't touch. CI's `apple-quality` job is the full run for that step.
- CI at 137abba4: actionlint, ts, no-node, stable/msrv (ubuntu), stable (macos), mac-setup, `build (generic/platform=macOS)` and `apple-quality` are green. `build (generic/platform=iOS Simulator)`, `e2e`, `msrv (macos-latest)` and `dmg` are still queued waiting for a macOS runner.
- `ui` (web Playwright) failed on one timing check: `[390] chat.spec.mjs:177` saw one 168 ms long task (the limit is 50 ms) while scrolling 1,000 messages. 176 passed, 23 skipped. This PR changes no `web/` file, so the failure is a runner timing flake. GitHub won't rerun the job until the queued macOS jobs in the same run finish.

## Review notes (non-blocking)

- The 48pt bound is loose. It catches this bug, since the empty row was 96–180pt, but it would miss a leftover gap of a few points. I left it alone: on a `--repro` issue the test changes only if the spec changes.
