Closes #249

## For humans

When no persona is pinned, Home no longer leaves an empty band above the chat list.
In the Terminal look, the bare "# pinned" heading is gone too.
With pinned personas, the row looks the same as before.

## What changed

- `apple/Sources/CooeeUI/HomeRows.swift`: `PinnedRow` draws nothing when `personas` is empty, in all three looks. The Terminal heading lives inside the row, so it goes with it. Home.swift is unchanged and keeps one call site.
- `apple/Tests/CooeeUITests/PinnedRowTests.swift` (commit 1 `145ea374`, the repro test; `8711a311` only splits one expression so Xcode's type checker accepts it, same assertion): for each `LookStyle`, it renders the real `HomeView` three ways: pinned plus chats, unpinned plus chats, and unpinned with no chats. It finds where the pinned row starts and where the chats start, then requires the gap to be under 48pt. Before the fix, the empty row (96–180pt depending on the look) sits in that gap.

src/ net +4 (budget not stated): a single empty guard in the row's body.

## Repro

Test command: `cd apple && swift test --filter PinnedRowTests`
- Commit 1 (`145ea374`) and its compile fix: expected to fail on every look.
- Head (`8711a3117a1831d64bdb2eb58f615f3fac612469`): expected to pass.

This host has no Xcode, only Command Line Tools, so neither run can happen here. The result comes from CI's `apple-quality` job (`just apple-check`, which runs the Swift tests). At `36106488`, `apple-quality` failed to compile the test ("unable to type-check this expression in reasonable time", plus a stray `try`). `8711a311` fixes that. At the head, `apple-quality` is still queued waiting for a macOS runner.

## Review notes (non-blocking)

- The 48pt bound is loose. It catches the bug, since the empty row was 96–180pt, but it would not catch a leftover gap of a few points. I left it alone: on a `--repro` issue the test changes only if the spec changes.
- Padding at `Home.swift:39` / `Wide.swift:80-81`: when nothing is pinned, `PinnedRow`'s body is an empty optional view. A stack lays out no child for it, so the padding should add no height. I could not check this by rendering it here, because there is no Xcode. The verify stage or the `PinnedRowTests` run on CI is the real check.

## Evidence at head 8711a3117a1831d64bdb2eb58f615f3fac612469

- `bash scripts/apple-check.sh` → exit 0 (swift format, anti-slop, TODO check, SwiftLint). It skipped build, tests, coverage and Periphery because there is no Xcode on this host.
- `cd apple && swift test --filter PinnedRowTests` → exit 1. It fails while building a dependency (`swift-perception`), before any test runs: `plugin for module 'SwiftUIMacros' not found`. That is the missing Xcode, not this change.
- CI at this head:
  - Passed: actionlint, stable/msrv (ubuntu), ts, no-node.
  - Queued: apple-quality, build (iOS/macOS), e2e, dmg, mac-setup, stable/msrv (macos).
  - `ui` failed on one web Playwright test, `[390] chat.spec.mjs:131` ("1,000 messages … no long tasks"). It found 4 long tasks over 50 ms on the runner, and the same test passed at 1440. This PR touches no `web/` file, so I read it as a timing flake that this PR did not cause.
