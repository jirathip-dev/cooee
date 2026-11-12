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
- Commit 1 (`145ea374`) plus its compile fix (`8711a311`): expected to fail on every look.
- `e42b69bc` takes the fix back out of `8711a311`'s tree, so CI runs the test with no fix in place. Expected red.
- Head (`2d1790f1fe52dd35f32609d26f1262b4c469a4d1`, the fix restored, same tree as `8711a311`'s fix): expected to pass.

This host has no Xcode, only Command Line Tools, so neither run can happen here. The result comes from CI's `apple-quality` job (`just apple-check`, which runs the Swift tests) at `e42b69bc` (red) and at the head (green).

## Review notes (non-blocking)

- The 48pt bound is loose. It catches the bug, since the empty row was 96–180pt, but it would not catch a leftover gap of a few points. I left it alone: on a `--repro` issue the test changes only if the spec changes.
- Padding at `Home.swift:39` / `Wide.swift:80-81`: when nothing is pinned, `PinnedRow`'s body is an empty optional view. A stack lays out no child for it, so the padding should add no height. I could not check this by rendering it here, because there is no Xcode. The verify stage or the `PinnedRowTests` run on CI is the real check.

## Evidence at head 2d1790f1fe52dd35f32609d26f1262b4c469a4d1

- `bash scripts/apple-check.sh` → exit 0 (swift format, anti-slop, TODO check, SwiftLint: no violations). It skipped build, tests, coverage and Periphery because there is no Xcode on this host.
- `cd apple && swift test --filter PinnedRowTests` cannot run on this host: the build stops on a dependency's macros (`plugin for module 'SwiftUIMacros' not found`) before any test runs. That is the missing Xcode, not this change.
- CI at this head (as of 2026-11-12 00:13 UTC): all three workflows (`ci`, `apple`, `apple-mac`) are still queued. They have waited about 1h40m for a runner and have not started. The same holds for `e42b69bc`. So I have no `PinnedRowTests` result to show yet, red or green. The merge gate needs the head's `apple-quality` to pass, and the reviewer's `Repro: run` block should take `e42b69bc`'s and the head's `apple-quality` logs once they exist.
- At `8711a311` (same tree as the head), `ui` failed once on a web timing test (`[390] chat.spec.mjs:131`). This PR touches no `web/` file, and the reviewer saw it pass on rerun.
