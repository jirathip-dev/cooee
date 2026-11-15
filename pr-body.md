Closes #432

## For humans
Tapping a pinned persona on Home now opens its latest chat with the full history, instead of an empty new chat.
Opening it marks the chat read, so the unread badge clears. A persona with no chat yet still opens a new one.
The plugin chips in a persona chat load faster and never hold up the messages.

## What changed
- `AppFeature` gains `pin(persona)`: opens the persona's newest one-to-one thread from Home's rows (rooms skipped), else falls back to `start` (new chat). Opening goes through the existing `.open` path, so the chat fetches `/threads/{id}/messages` and posts `read`.
- `PinnedRow` (Home and Wide) calls the new `HomeActions.pin`; the + menu's persona entries keep `new` → `start`.
- `ChatPlugins.load`: places and installed plugins are fetched in parallel (`async let`). The transcript already runs in its own `.task`, apart from the plugin task.

## Repro
Commit 1 `eb7243c86` holds only `apple/Tests/CooeeFeaturesTests/PinnedTests.swift`.
- Base + commit 1: `cd apple && swift test --filter PinnedTests` → exit 1 (`type 'AppFeature.Action' has no member 'pin'`).
- Head `c2c76d342b1fe6cb544b6cb8c4d79d801b73bb7c`: same command → exit 0, 2 tests pass (pin with threads opens `.thread("new")`, fetches messages, posts `read new 7`; pin with no thread starts `new:coder`).

## Evidence at c2c76d342b1fe6cb544b6cb8c4d79d801b73bb7c
- `scripts/apple-check.sh`: swift format, anti-slop, TODO, SwiftLint, strict build, tests with coverage all pass (Core 97.9%, API 97.1%, Features 97.4%); the run was killed by the lane's 30-min time limit while running `swiftlint analyze`; the remaining steps were re-run separately at the same head: `swiftlint analyze --strict` → exit 0, `periphery scan` → exit 0 ("No unused code detected").
- Not verified here: the e2e screenshot comparison of persona vs lane chat transcript timing (needs the verify stage / real server).

src/ net +11 (budget n/a): one action, one closure wired through Home/Wide/Root, plugin load split into two `async let`.
