Closes #369

## For humans

Home and an open chat now keep their last copy on the phone. When the server can't be reached they still show it, with "as of 3m ago".
A small dot shows the link: green live, amber reconnecting, grey offline. A chat that loses its stream says "reconnecting…" and catches up when it is back.
Errors that used to be silent now show a short message that clears itself. Pull down to refresh Home or a chat, and both reload when the app comes back after 30 s away.
After Stop, the working pill reads "Stopping…" until the turn ends.

## What changed

- **Kept copy (row 1):** a new `KeptCopies` dependency (Live: the existing `KeptScreens`, made generic, in its own `kept/` folder) saves Home's list on every load and a thread's slice on load and on each new item. A failed load with nothing on screen draws the copy and sets `asOf`.
- **Sync dot (rows 2, 25):** `Client.events` yields `.sync(.reconnecting)` on a failed attempt (`.offline` for no network) and `.sync(.live)` once it reconnects. Resume still sends `Last-Event-ID`. Home goes amber on its first failed poll and grey after that. The dot (`SyncBadge`) sits in the chat header and under Home's title, and the working pill reads Reconnecting… or Offline.
- **Strip (row 3):** the existing loading bar now fades in only after 300 ms (`SlowStrip`). It ignores taps, and still frames draw it at once.
- **Toasts (row 4):** `error` in Home and chat is drawn as a toast and cleared by a 3 s clock effect. Every chat failure path now toasts, including `.failed`, refused card answers and undelivered sends. Home toasts only the first failure after being live.
- **Refresh (row 5):** pull to refresh on Home and chat (the existing `Refreshing` modifier). `AppFeature` reloads Home and the open thread when the scene goes active after ≥ 30 s in the background.
- **Stopping… (row 25):** [Stop] sets `status.stopping`. The flag clears when the turn stops working or a call fails, and the Stop button is disabled meanwhile.
- Tests that hit an error path now give the toast its clock and receive `toasted`.

Decided: `KeptCopies` is a plain struct whose test value keeps nothing, not a `@DependencyClient`. The macro reports every unstubbed endpoint as unimplemented, which would add stubs to 30+ unrelated tests.

src/ net +356 (budget: none stated). It is spread over the six rows: about 60 lines of kept copy, 50 of sync, 40 of toasts and the rest refresh, stopping and the views. Apple lines would be 18425 / 18700 with this PR.

## Evidence (head a65c6d563a98bc1f8028a7b690e9407f458a7464)

Staging build fix: staging's `ci` is red at 4e338bb. #653 gave the `card.fold` fixture in `docs/design/screens-as-data/fixtures/blocks.json` a toned subtitle object, and #652 switched `cooee plugin check` to the blocks crate's schema, which only takes a string subtitle. The fixture's subtitle is now the plain string `"paused by owner"`. The fold test checks only the fold height and colour, so it still passes.

- At a65c6d5: `cargo test -p cooee --test plugins`: exit 0, 20 passed
- At a65c6d5: `scripts/apple-check.sh`: exit 0, `apple-check: ok` (includes `aPausedFleetsCardFoldsToOneLineAndDrawsInItsOwnColour` passed)

Round 7 (at 048b21e):

Update round 7: merged `origin/staging` (4e338bb: #652 crates blocks and #653 apple Queue cards, no conflicts). The review FAIL at 53d33f8 was `actionlint` in a `ci` run that was canceled when a newer run at the same head replaced it. That newer run was still queued. This branch changes no workflow files.

- At 048b21e: `scripts/apple-check.sh`: exit 0, `apple-check: ok`
- At 048b21e: `drover arch-check`: exit 0, 2136 files, 0 violations

Round 6 (at 53d33f8):

Update round 6: merged `origin/staging` (ca94eb9, which adds ChatDeck.swift, no conflicts). The review FAIL at 5f9abfa was CI `lessons` still pending; it then passed. The other red checks at 5f9abfa were a canceled superseded run, a runner without the iPhone 17 simulator (the rerun passed), and a macOS-only Rust routines test this branch does not touch (it has no `crates/` changes; staging is green at the same base).

- At 53d33f8: `scripts/apple-check.sh`: exit 0, `apple-check: ok`
- At 53d33f8: `drover arch-check`: exit 0, 2125 files, 0 violations

Round 5 (at 5f9abfa):

- `scripts/apple-check.sh`: exit 0; `drover arch-check`: exit 0

Round 4 (at 48ae05e):

Update round 4: merged `origin/staging` (report screen). `AppFeature.swift` and `Root.swift` conflicted on adjacent lines (this branch's `scene` action and foreground reload vs staging's `reporting`/`report`); both sides kept. To stay under SwiftLint's limits, `navigate` moved from the struct body into the file's extension, and the `moved` helper lost its `persona` argument (its only caller passed the chat's own persona).

- At 48ae05e: `scripts/apple-check.sh`: exit 0, `apple-check: ok`
- At 48ae05e: `drover arch-check`: exit 0, 0 violations
- CI `apple` is red for a runner reason: the runner has not accepted the Xcode license (exit 69); staging 1b1a94c fails the same way.

Round 3 (at 3e16f70):

Merged `origin/staging` (1b1a94c: older messages on scroll #372, colour schemes #375). No content conflicts left. Staging's older-page tests pushed `ChatFeatureTests.swift` past SwiftLint's 250-line file limit, so they and the setup-mode test moved unchanged to `ChatOlderTests.swift`. The `store` helper and `calls` are now internal so the extension in that file can use them.

- At 3e16f70: `scripts/apple-check.sh`: exit 0, `apple-check: ok`
- At 3e16f70: `drover arch-check`: exit 0, 0 violations

Round 2 update (at 1c837e0, merged d9fe9ec):

Update round: merged `origin/staging` (d9fe9ec). Resolved `ChatFeature.swift` (both `drafts` and `kept` dependencies), `ChatOutbox.swift` (a failed send restores the draft, saves it and toasts), `ChatScreen.swift` (sync/stopping actions plus staging's edit/photos/quote actions). This branch's thread-copy helper is renamed `keepSlice` so it does not clash with staging's draft `keep`. Four `ChatFeature.State` computed properties moved to an extension to keep the struct under SwiftLint's 150-line limit. Staging's `ChatComposeTests` now expects this branch's `.link(.dropped)` on a failed load.

- At 1c837e0: `scripts/apple-check.sh`: exit 0, `apple-check: ok`

Round 1 evidence (at 9b647a1):
- `scripts/apple-check.sh`: exit 0, `apple-check: ok` (swift format, anti-slop, SwiftLint, Swift 6 strict build, tests with Core/API coverage, anti-slop tests)
- `swift test` (apple/): exit 0, all suites green (175 Features, 110 Core, 61 API, 46 UI, 1 App)
- `ci/gates.sh comments cap hygiene neutral lessons`: comments, hygiene, neutral and lessons ok. cap failed locally only because the script's `find apple` counts the local `apple/.build` checkout (397243 lines). Leaving `apple/.build` out, the same count is 17714 / 18700.
- `drover arch-check`: exit 0, 0 violations

New reducer tests: `HomeFeatureTests.offlineHomeDrawsTheListItLastLoadedWithItsTimeAndToastsOnlyTheFirstFailure`, `ChatFeatureTests.aThreadThatCannotLoadDrawsTheSliceItLastLoadedWithItsTime`, `aFailedLoadToastsOnceGoesOfflineAndTriesAgain`, `aFailedCallToastsWhyUntilItsTimeIsUp` (test clock), `aStopReadsStoppingUntilTheTurnEnds`, `AppFeatureTests.homeAndTheOpenChatReloadWhenTheAppComesBackAfterThirtySecondsAway`, `EventsTests.aServerErrorABadEventOrNoNetworkIsRetriedAndSaysSoUntilItIsBack`.

Not verified: acceptance 2 and 3 are phone checks on a TestFlight build (airplane mode, Stop on a live turn). I could not run them from this lane.

