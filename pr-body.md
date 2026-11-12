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

## Evidence at head f70f880f93b59ef2a305b07cab429024e5a1869f

This head is `137abba4` with current `origin/staging` merged in. The PR diff is still the same two files (+38 lines).

- `cd apple && swift test --filter PinnedRowTests` at this head → exit 0 (3 of 3 looks pass).
- At `137abba4`: the same command → exit 0. With the fix backed out as described under Repro → exit 1 (3 of 3 looks fail). `bash scripts/apple-check.sh` passed format, anti-slop, the Swift 6 build, every test suite and coverage. `apple-quality` and `build (generic/platform=macOS)` were green in CI.
- CI at `137abba4`: `build (generic/platform=iOS Simulator)` and `dmg` sat queued for over 5 hours with no `macos-26` runner. Every branch, staging included, had macOS jobs queued, so this is a runner backlog, not this change. `ui` failed one web timing check (`chat.spec.mjs`, 1,000 messages, one long task over 50 ms) on a PR that changes no `web/` file. This push starts a fresh CI run for both.

## Review notes (non-blocking)

- The 48pt bound is loose. It catches this bug, since the empty row was 96–180pt, but it would miss a leftover gap of a few points. I left it alone: on a `--repro` issue the test changes only if the spec changes.
