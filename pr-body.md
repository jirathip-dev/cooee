Closes #236

## For humans
- In a chat, the round … button in the header now opens the persona's panel instead of doing nothing.
- On iPhone the panel slides up as a sheet; on Mac and iPad it sits as a column beside the chat, and … toggles it.
- The panel has Edit and New chat for the persona, its plugin places, an Attach row for each plugin it can take, and Stop while it is working.
- Routines are not in the panel yet (see Decisions).

## What changed
- `ChatFeature` holds the panel: `panel`, `picked`, `.panel(.show/.pick/.dismissed/.run)` (`CooeeFeatures/ChatPanel.swift`). In the iPhone sheet a pick closes the panel and runs on the sheet's `onDismiss`, so the persona sheet and the plugin/attach sheets never try to present over it; the Mac/iPad column runs a pick at once.
- New delegates `edit(id)` / `newChat(id)`: `AppFeature` opens the persona sheet or starts a new chat with that persona.
- `ChatHeader`'s … is always a `Button` ("More"); the old Menu of Attach items moves into the panel.
- `PersonaPanel` gains a live init with `PanelActions`; the footer and place rows are buttons. The design-frame fixtures render unchanged (snapshot tests green).

## Decisions
- Hidden live, per "Never a control that does nothing: an item with no action is hidden, not drawn": Edit/New chat when the chat's persona is unknown, Stop when no turn runs, Memory, Screen, Terminal, Routines.
- Routines (#75) are not drawn: a routine belongs to a chat (its place's thread, or a new_thread `who`), not a persona, and the native app has no routine screen or editor, so each row would be a dead control. Workflow appears where a plugin exposes it as a place (Conductor's drover places).

## Repro
- Commit 1 `3a65ed4b` adds only the e2e check (`personaPanel` in the drover screens flow; after staging's split it lives in `LiveScreensUITests.swift`).
- At commit 1 (local iPhone 17 simulator, `xcodebuild … -only-testing:CooeeAppUITests/LiveUITests/testPairsStreamsAndShowsHome test-without-building` against `apple/e2e/serve.sh`): exit 65, `Failed to tap "More" Button: No matches found`.
- At `ba82138d` (fix, before the staging merge) the same run passes the panel step (`Edit Conductor`, `New chat with Conductor`, no Attach row) and attaches `persona-panel-dark`; that run later failed in the untouched app-icon step (`Failed to tap "Follow the look"`: a tap right after the App icon push, no wait), which is green on staging's CI. CI's e2e job at the head decides; its `simulator-shots` artifact carries `persona-panel-{dark,light}.png`, the simulator screenshot of the open panel.

## Evidence at head 4a018f7d139044bbc42bda4c47c261ad2152ba5c
Update round: merged `origin/staging` (54b470a0c: #244, #328, #336, #365, #388, #389, #368, #277, #318). No conflicts.
- `scripts/apple-check.sh`: exit 0 (`apple-check: ok`).
- `cargo test --workspace --locked`: exit 0.
- `scripts/line-cap.sh` on a clean export of the head: exit 0, non-test 20933 / 27500, apple 13263 / 14316.
- `drover arch-check` on the same export: exit 0, 0 violations.

## Evidence at head ae81bbc3744289e4689cec5226751d2a931721f4
Update round: merged `origin/staging` (4b86b3c4e). #314 rebuilt the persona panel as `PersonaPanelBlocks` over a screen's blocks and deleted `PersonaPanel`, `PanelContent` and `PanelSections`. This PR's live panel is ported onto it rather than keeping a second panel:
- `PersonaPanelBlocks` gains `PanelActions` and a live `init(persona:screen:column:actions:)`. Close, Edit, New chat, Memory and Stop are buttons, each drawn only when it has an action. The fixture init gives Edit only when the screen has an `edit` block, plus New chat and Memory, as #314's frames draw them.
- `PersonaBlocks`' places rows are buttons (`open(index)`). The chat builds a `places` block of its plugin places followed by `Attach <plugin>…` rows. A tap opens the place or attaches the plugin.
- Design frames unchanged: `swift test --filter "ChatPanelTests|SnapshotTests"` (in `apple/`): exit 0.
- `scripts/apple-check.sh`: exit 0 (`apple-check: ok`).
- `scripts/line-cap.sh` on a clean export of the head: exit 0, non-test 20870 / 27500, apple 12895 / 14316.
- `drover arch-check` on the same export: exit 0, 0 violations.
- apple src net vs staging: +115 (+168 −53).

## Evidence at head f846ba51f2061adace559bd2787b88463782ce6c
Update round: merged `origin/staging` (7b26a56c3, #177's linked exchanges). #177 added the chat delegates `.persona(id)` (open the persona sheet) and `.start(id)` (new chat with a persona), the same as this PR's `.edit` / `.newChat`. The panel now sends #177's delegates and this PR's two are deleted, so `AppFeature.swift` equals staging's. `ChatPanelTests` receives `\.chat.delegate.persona` then `\.edit`, and `\.chat.delegate.start` then `\.start`. The state it asserts is unchanged.
- `swift test --filter ChatPanelTests` (in `apple/`): exit 0, 3 passed.
- `scripts/apple-check.sh`: exit 0 (`apple-check: ok`).
- `scripts/line-cap.sh` on a clean export of the head: exit 0, non-test 19970 / 27500, apple 12682 / 13000.
- `drover arch-check` on the same export: exit 0, 0 violations.

## Evidence at head a773fca2ac2d3e3a43451f9e65459961a086cc2e
Update round: merged `origin/staging` (418b880e1, #152's machine picker). `ChatFeature.swift` keeps the panel's case beside #152's `.machine` case.
- `scripts/apple-check.sh`: exit 0 (`apple-check: ok`).
- `scripts/line-cap.sh` on a clean export of the head: exit 0, non-test 19838 / 27500, apple 12112 / 13000.
- `drover arch-check` on the same export: exit 0, 0 violations.

## Evidence at head dbeed11a018433f1baf37aed176991b79c2db23a
Update round: merged `origin/staging` (acd779bff, #242's chat style). `Chat.swift` keeps the panel's `more` and #242's `step`. The old `attachable`/`attachPlugin` header fields stay gone.
- `scripts/apple-check.sh`: exit 0 (`apple-check: ok`).
- `scripts/line-cap.sh` and `drover arch-check` on a clean export of the head: both exit 0; non-test 19838 / 27500, apple 11806 / 13000, arch-check 0 violations.

## Evidence at head 15bb1772dc84a97ec650e4d01b42000bb2111460
Update round: merged `origin/staging` (1bd24c207, #426 groups ChatFeature's actions per area). `ChatFeature.swift` keeps `.panel` beside #426's `.delegate`, and the panel's case joins #426's per-area switch. `ChatScreen.swift` takes #426's `.outbox(.retry)`. The panel's Stop now sends `.compose(.stop)`, and `ChatPanelTests` receives `\.chat.compose.stop`. That respells the action path; the assertions are unchanged.
- `scripts/apple-check.sh`: exit 0 (`apple-check: ok`).
- `scripts/line-cap.sh` on a clean export of the head: exit 0, non-test 19664 / 27500, apple 11032 / 13000.
- `drover arch-check` on the same export: exit 0, 0 violations.

## Evidence at head 104a443340595b408db400bc0f1c88e74c7578b7
Update round: merged `origin/staging` (0cf44ba40, #425 moves `ChatScreen` out of `Root.swift`). `Root.swift` takes staging's text; the panel's `ChatScreen` changes (the … toggles the panel, the HStack column and the iPhone sheet) move with it to `ChatScreen.swift`.
- `scripts/apple-check.sh`: exit 0 (`apple-check: ok`).
- `scripts/line-cap.sh` on a clean export of the head: exit 0, non-test 19517 / 27500, apple 11002 / 13000.
- `drover arch-check` on the same export: exit 0, 0 violations.

## Evidence at head 475e87513efb9124ea8279fa3afd7a2975ced6c9
Update round: merged `origin/staging` (34c1f32c1, #229's Workflow sheet). Resolved `AppFeature.swift` (the chat's Edit delegate plus #229's `round` / `home(.loaded)` cases), `ChatFeature.swift` (panel state plus `workflow`/`workflowOpen`/`unfolded`), `Chat.swift` (`more` plus #229's workflow chip and `backTo`; the old `attachable`/`attachPlugin` header fields stay gone, the panel's Attach rows took their place), `Root.swift` (the panel HStack and sheet around #229's `actions` with `backTo` and its Workflow sheet). `ChatPanel.swift` drops an import `swiftlint analyze` now flags as unused. Staging fixed its hub.rs arch baseline, so arch-check is green again.
- `scripts/apple-check.sh`: exit 0 (`apple-check: ok`).
- `scripts/line-cap.sh` on a clean export of the head: exit 0, non-test 19517 / 27500, apple 10993 / 13000.
- `drover arch-check` on the same export: exit 0, 0 violations.

## Evidence at head 5a1ad08b2445ff654b03c2829fc0f456d488f261
Update round: merged `origin/staging` (51141ae66). #289 already made staging's shed-version test fix, so `crates/cooee/tests/chat.rs` and `setup.rs` take staging's text; this PR no longer touches `crates/`. The merge brought no `apple/` change.
- `cargo test --workspace --locked`: exit 0.
- `drover arch-check`: exit 1, `crates/cooee-server/src/hub.rs is 3120 lines, over 800 (baseline Some(3067))`. It fails the same way on plain origin/staging (staging's ci run 43728703433, arch-check red). #289 caused it; this PR touches no `crates/` file. Not fixed here: `.drover/arch.toml` says the baseline's offenders "may only shrink".

## Evidence at head 2fcf25e1ef058e0acd5a6394d8ddaeb48f90b23f
Fix round 3, staging build fix: staging's own CI is red on `crates/cooee/tests/chat.rs:353` (ubuntu msrv/stable, macOS stable), because a server-side change added ", an older shed; restart shed serve" to the SHED_VERSION line and two tests kept the old wording. `chat.rs` and `setup.rs`'s `old_shed` now expect the server's text. No `src/` change.
- `cargo test --workspace --locked`: exit 0 (it failed with 101 before the fix: `chat.rs:353`, then `setup.rs:346` on macOS).
- `cargo fmt --all --check`: exit 0.
- Apple code is unchanged since 4a6ca65; the evidence below still holds for it.

## Evidence at head 4a6ca652f93586fc0613641109826cb03164f8bb
Update round: merged `origin/staging` (b5814386a). Resolved `AppFeature.swift` (kept staging's `.pin` and the panel's New chat delegate), `ChatFeature.swift` (panel state plus staging's `ScreenFeature` rename), `Root.swift` (the panel's HStack/sheet around staging's static `ChatScreen.actions`). Follow-ups: `ChatPanelTests` uses staging's `AppFeature.State(layout:server:)` and `ScreenFeature`; the chat's Edit delegate moved into the main reducer so `navigate` stays under SwiftLint's complexity 8 / 25 lines.
- `scripts/apple-check.sh`: exit 0 (`apple-check: ok`: format, anti-slop, SwiftLint, build, tests, coverage, swiftlint analyze, Periphery, anti-slop's tests).
- `scripts/line-cap.sh` on a clean export of the head: exit 0, non-test 19471 / 27500, apple 10499 / 13000; `ci/line-cap-test.sh` exit 0.
- apple src net +126 (+187 −61) vs staging (no budget given): the panel's reducer, its live wiring and the buttons.
- Simulator screenshot: CI's e2e `simulator-shots` artifact, `persona-panel-{dark,light}.png`.

## Why (round 1)

Decided: the … is always a Button that toggles ChatFeature's panel; on iPhone the panel is a sheet, on Mac and iPad a 360-pt column beside the chat.
Because: "the … opens the persona panel, designed in #244 (iPhone full-height sheet, Mac/iPad right column)"
Decided: in the sheet a pick closes the panel first and runs on the sheet's onDismiss, so Edit's persona sheet and the plugin and attach sheets never present over it.
Because: apple/Sources/CooeeFeatures/ChatPanel.swift:30
Decided: the panel lists the persona's plugin places and an Attach row per attachable plugin, and the header Menu of Attach items is gone.
Because: "its attached plugins' places (#129: e.g. Conductor's Queue, Events, Workflow) and Attach <plugin>… for each attachable plugin"
Decided: Edit and New chat hide for a chat whose persona is unknown; Stop shows only while a turn runs; Memory, Screen, Terminal and Routines are not drawn live.
Because: "Never a control that does nothing: an item with no action is hidden, not drawn."
Rejected: a native routines list — #75's routines belong to a chat, not a persona, and the app has no routine screen or editor yet, so every row would be a dead control.
Check: LiveUITests opens Conductor's … and finds Edit Conductor and New chat with Conductor with no Attach row; the design frames' snapshots are unchanged.
Owner: The … in a chat now opens the persona panel with Edit, New chat, the persona's plugins and Stop; routines are not in it yet.

<sub>note on 4e4f924 · git log --notes=why</sub>

## Why (round 2)

Decided: merged origin/staging, keeping the persona panel on top of staging's layout, pin and ScreenFeature changes.
Because: apple/Sources/CooeeUI/Root.swift:87
Decided: the chat's Edit delegate is handled in AppFeature's reducer, not navigate, to keep navigate under SwiftLint's limits.
Because: apple/Sources/CooeeFeatures/AppFeature.swift:83
Rejected: none
Owner: The branch is up to date with staging again; the … panel works as before.

<sub>note on 4a6ca65 · git log --notes=why</sub>

## Why (round 3)

Decided: merged origin/staging with #314; the live panel is ported onto #314's PersonaPanelBlocks with a live init and actions, and the old PersonaPanel files stay deleted.
Because: apple/Sources/CooeeUI/BlockPersonaPanel.swift:48
Decided: the chat builds the panel's places block itself, its plugin places then Attach rows, and a row's index picks open or attach.
Because: "its attached plugins' places (#129: e.g. Conductor's Queue, Events, Workflow) and Attach <plugin>… for each attachable plugin"
Rejected: fetching #314's server persona screen for the live panel — its routine rows have no screen to open yet, and a new client is more than an update round.
Check: the design frames' snapshots are unchanged, and ChatPanelTests and the live UI test still find Edit and New chat with the persona.
Owner: The branch is up to date with staging; the … panel now draws through the new persona panel and works as before.

<sub>note on ae81bbc · git log --notes=why</sub>

