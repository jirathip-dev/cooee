Closes #229

## For humans

Home now lists only your personas' chats. Sessions started by a workflow tool (drover's lanes) no longer show up as bare rows: once drover is attached to Conductor they appear in a new **Workflow** chip on Conductor's header, with one row per issue that unfolds into its rounds, and a round opens as a normal chat. Everything else (chats with no persona, and workflow sessions no attached plugin claims) sits under one **Others** row at the bottom of Home, with a count, so nothing is hidden.

## What changed

- **Server/API** (scope 5): a plugin manifest may say `claims = ["drover"]`; `GET /plugins` returns each installed plugin's `claims` (`docs/api.md`, `docs/plugins.md`, `screens.schema.json`). The attach sheet lists it as one more "may" line.
- **Core** (`apple/Sources/CooeeCore/Workflow.swift`): one pure function, `Listing.home(rows:personas:plugins:now:)`, puts each `GET /threads` row in exactly one place: `chats` (a persona's, no `workflow` label), `workflow[persona]` (a `source` claimed by a plugin attached to that persona), or `others`. `Workflow.issues` groups a persona's sessions by group+item into one row per issue, ordered needs you → failed → working → the rest, rounds oldest first.
- **Features**: Home also loads `GET /plugins`; a chat carries its persona's workflow sessions (kept fresh on each Home poll); `.workflow(.show/.unfold/.open)` in `ChatFeature`; Others is a path (`AppFeature.others`) whose chats open above it.
- **UI**: Others row at the foot of Home (phone and Mac sidebar), the Others screen, the Workflow chip with the needs-you count, and the Workflow sheet.
- **e2e**: the fixture drover in `apple/e2e/serve.sh` claims `drover` and the fake shed runs its three lanes (`FAKE_SHED_LANES`); `LiveUITests` keeps `home`, `others`, `workflow` (one issue unfolded), `workflow-round` and `home-no-lanes` shots.

Decided (q1341 approved the design README's recommendations): the sheet is built from `GET /threads`' existing `workflow` labels, no new `GET /workflow` route; the claim is `claims` in the manifest; Others' count counts every session in it; the chip shows the needs-you count. A plugin that claims a source but is attached to no persona leaves those sessions in Others.

`src/ net +414` (Rust +8, apple +406; no budget stated in the issue): the classification, two reducer paths, the Workflow sheet and Others views. `apple_cap` holds now that #223 is on staging: apple 8179 / 13000; line_cap 26477 / 27500.

## Round 4 (verify FAIL at fa649a0)

- Blocking: Home's Others row now has the titles it holds on a second line and a `›` beside its count pill, as in `home-390-dark.png`. The row gets the titles and counts them, so the count and the line always agree.
- Non-blocking: the Workflow chip draws its branch icon before `Workflow`.
- Non-blocking: the e2e's Others step no longer returns silently. When Home has no Others row it opens Conductor and requires drover's Workflow chip, which only holds when drover claims the lanes. When the row is there it requires the `NO PLUGIN CLAIMS IT` section and `cooee #33`.
- Left as is: the neutral disc behind the state figure in the sheet's issue rows (non-blocking, visual only).
- Merged origin/staging (#223, #225, #75, #165, #244).

## Round 5 (FAIL at 9270373)

- Blocking: Conductor's Workflow chip now draws its needs-you count in red, as the approved design's open question 5 recommends ("in red as Queue's is", approved in q1341). The shared count badge takes an optional tint; Home's unread badges stay blue.
- Non-blocking, left as is: the unfolded round's tool-lines box and PR link (the thread list carries no PR, as the reviewer notes); the icon-step flake from #225 (not this PR's code); `ui`'s Playwright flake (no web/ change here).

## Round 7 (FAIL at 7bf8697: CI pending)

- No code change: the FAIL was every macOS job sitting queued for 2.5 h (runner availability). Merged origin/staging (#226, #284, #298) so CI runs again at the new head. `apple/` is byte-identical to 7bf8697.
- #226 moved drover to herdsman's repo; its manifest's `claims = ["drover"]` is herdsman's to add. Until then drover lanes fall into Others, as scope 4 intends. The e2e fixture drover still claims it.

## Round 8 (FAIL at f63632e: CI red, staging conflict)

- Merged origin/staging (#157 rooms, #150, #288). Six apple files conflicted; kept both sides: a thread row carries #157's room members and this PR's workflow label.
- A room (a chat with members, no single persona) counts as a persona chat: it stays in Home's list, not in Others.
- #157's room test now stubs the plugin list that Home's refresh fetches; AppFeature's merged body is trimmed one line to stay within SwiftLint's type length.
- The red check at f63632e was `apple-quality` timing out after 2 h 25 m (runner), not a failing step; `build (iOS Simulator)` shows green on a rerun.

## Round 9 (FAIL at 3b1a288: evidence named an older head)

- Evidence below re-run at the new head. Merged origin/staging (#355) first so the head would not go stale again; it brings staging's CI scripts, design files and a web e2e test. `git diff 3b1a288 HEAD -- apple/Sources` is empty.

## Round 12 (verify FAIL at 1bc91a8: Others row showed agent names)

- Home's Others row now lists the titles of the chats inside it (`Bonjour probe, …`), as the approved design shows, not each chat's agent (`fake-acp, fake-acp`). The titles come from one Core property on the home lists, and the Others screen is unchanged.
- WorkflowTests asserts those titles for a no-persona chat and two unclaimed workflow sessions; the old mapping would give `fake-acp` for the loose chat.
- Merged origin/staging (#293); it brings no apple changes.
- Non-blocking sheet height / round age: left for a design pass. The LiveIconUITests alert is #225's, from staging.

## Round 13 (FAIL at d8b2a38: stale evidence; update round: staging conflict)

- Merged origin/staging (#310 LayoutFeature/SlotView, #181 rounds in Queue rows, #396, #397, #413, #414, #430, #432, #439, #440). #310 moved Home and the chat out of `Root.swift` into `SlotView.swift`, so this PR's Home pieces now live there: Home's list from the persona/workflow/Others split, the Others row and its titles, the Others screen (phone stack and Mac detail), and the "Workflow" back label on a round.
- #181 made a lane tap in a plugin screen replace the open chat. A Workflow round tap still pushes onto the Conductor chat, so Back returns to the open sheet. The chat now sends a separate `round` delegate for this, and each tap keeps its own test.
- `AppFeature`'s helpers moved into an extension in the same file, keeping it under SwiftLint's 150-line type limit after the merge.
- The evidence below was produced at the new head.

## Round 16 (verify FAIL at 25e3229: design deviations)

- Blocking: an unfolded issue's rounds now hang on a vertical line, each with its state dot, stage, state word, age and `›`. The current round adds a card with its last line, `open chat ›` and a link chip: `PR #n ›` for a pull URL, `issue #n ›` for an issue URL, from the workflow label's `url` that `GET /threads` already sends.
- The card shows the round's single last line, not several tool lines: the thread list carries only each thread's last line, and fetching every round's items would be a new call for one card.
- Blocking: the Others screen has its inline `Others` nav title beside `‹ Home`, and ends with one footnote per workflow source: "<plugin> claims source <s>: its sessions are in <persona> › Workflow.", or, when nothing places it, a line saying to install a plugin that claims it and attach it to a persona. The footnotes come from the same Core pass that sorts the rows.
- The outlined chip style moved out of `LinkChip` into one `OutlineChip` modifier that both chips use.

`src/ net`: apple 10876 / 13000, line_cap 19479 / 27500 (no budget stated in the issue).

## Round 17 (FAIL at 5ade829: CI red)

- Staging build fix: staging's own CI was red on `a_send_to_a_shed_on_another_protocol_says_why_in_the_chat_then_goes_through_once_fixed` (ubuntu, macOS, msrv). The #397 tests (`chat.rs`, and on macOS `setup.rs`) still expected the old "protocol 0; fix:" wording, but #393 changed it to "protocol 0, an older shed; restart shed serve; fix:". I updated the two expectations to match the code, as `chat.rs:307` already did. Staging's #289 (51141ae) then made the same test fix, so after merging it both files match staging and this PR no longer touches them.
- apple-quality: `swiftlint analyze` flagged this PR's unused `import Foundation` in `AppFeature.swift`. I removed it.
- Merged origin/staging 51141ae (#431, #289).

## Evidence (head 9123c79552182ece05d6ebf9626fc9efd40727aa)

- `cargo fmt --all -- --check`: exit 0. `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0. `cargo test --workspace --locked`: exit 0 (every suite 0 failed, `chat` 11 passed, `setup` passed).
- `sh scripts/apple-check.sh`: exit 139. Format, anti-slop, TODO, SwiftLint, build with warnings as errors, tests and coverage pass. Then `swiftlint analyze` segfaults locally reading the TCA module, as in earlier rounds, so CI apple-quality decides that step (where it caught the unused import). Run at 3343b1a; the staging merge after it changes nothing under `apple/`.
- `scripts/line-cap.sh` on a clean `git archive` of 9123c79: exit 0 (`non-test lines: 19524 / 27500`, `apple lines: 10875 / 13000`).
- 0 commits behind origin/staging.

## Why (round 5)

Decided: Home's Others row now shows the titles it holds on a second line and a chevron beside its count, per the approved design
Because: "one row at the bottom of the list, with a count"
Because: apple/Sources/CooeeUI/Workflow.swift:17
Decided: HomeActions carries the Others titles instead of a bare count; the count is derived from them, so the two cannot disagree
Decided: the Workflow chip gets its branch icon (non-blocking finding)
Decided: the e2e no longer passes silently without an Others row; with none it must find drover's Workflow chip on Conductor
Because: apple/UITests/LiveWorkflowUITests.swift:11
Rejected: the bare state figure in the sheet's issue rows — non-blocking, left for a design pass
Check: in the light e2e run (drover already attached by the dark run) the else path opens Conductor and asserts the Workflow chip
Check: the line cap measured on a clean git archive; a local run also counts untracked SPM .build sources
Owner: Home's Others row now previews what is inside it, like the design.

<sub>note on 9270373 · git log --notes=why</sub>

## Why (round 6)

Decided: Conductor's Workflow chip draws its needs-you count in red, through an optional tint on the shared count badge
Because: q1341
Because: apple/Sources/CooeeUI/Workflow.swift:220
Rejected: a second badge view for red counts — one optional tint keeps one badge
Check: the chip's count pill is red in workflow-dark.png's strip; Home rows' unread badges stay blue
Owner: The Workflow button's count is now red, so it reads as "needs you" and not "unread".

<sub>note on 7bf8697 · git log --notes=why</sub>

## Why (round 7)

Decided: round 7 changes no code; it merges origin/staging so CI reruns on a new head
Because: "Every gate is green; `apple_cap` holds."
Rejected: a code change for the FAIL — it was only macOS CI jobs queued for 2.5 h, not a red check
Decided: drover's real manifest now lives in herdsman (#226); its claims line is herdsman's change, until then its lanes land in Others
Because: plugins/index.toml:8
Check: git diff 7bf8697 HEAD -- apple is empty, so the apple-check from round 6 still holds
Check: the macOS jobs (build, apple-quality, e2e, simulator-shots) finish green at this head
Owner: No change to the feature; this round catches up with staging so the Mac CI jobs run again.

<sub>note on f63632e · git log --notes=why</sub>

## Why (round 8)

Decided: round 8 merges origin/staging (#157 rooms); a thread keeps both its room members and its workflow label
Because: apple/Sources/CooeeCore/Models.swift:44
Decided: a room counts as a persona chat, so it stays in Home's list and never folds into Others
Because: "the pinned personas row, then only chats that belong to a persona"
Because: apple/Sources/CooeeCore/Workflow.swift:82
Decided: #157's room test stubs the installed-plugins call that Home's refresh now makes
Because: apple/Tests/CooeeFeaturesTests/RoomFeatureTests.swift:121
Rejected: moving room handling out of AppFeature to free lines — one line trimmed in .home(.loaded) is enough
Check: the f63632e red was apple-quality timing out at 2h25m, not a failing step
Check: CI's macOS jobs (build, apple-quality, e2e, simulator-shots) are green at 898ab42
Owner: Same feature, caught up with the new group-room screens; rooms stay in your Home list.

<sub>note on 898ab42 · git log --notes=why</sub>

## Why (round 11)

Decided: round 9 changes no feature code; it merges origin/staging (#355) and refreshes the PR evidence to the new head
Because: "Every gate is green; `apple_cap` holds."
Rejected: refreshing the body at 3b1a288 only — staging had moved again, so the head would have gone stale at the next update
Decided: the line cap is measured on a git archive of the head; a worktree run counts untracked SPM .build sources
Because: scripts/line-cap.sh:1
Check: git diff 3b1a288 HEAD -- apple/Sources is empty; the merge only brings staging's CI scripts, design files and the web e2e test
Check: the macOS jobs (build, apple-quality, e2e, simulator-shots) run at this head and give the AC3 screenshots
Check: local swiftlint analyze segfaults reading the TCA module (exit 139, twice); CI apple-quality at this head decides that step
Owner: No change to the feature; this round catches up with staging and refreshes its test evidence.

<sub>note on 1bc91a8 · git log --notes=why</sub>

## Why (round 12)

Decided: Home's Others row lists the titles of the chats inside it, not each chat's agent
Because: docs/design/home-workflow/README.md:40
Because: apple/Sources/CooeeCore/Workflow.swift:28
Decided: the titles come from one Core property, so a test covers them; the Others screen keeps its name-plus-agent rows
Because: apple/Sources/CooeeUI/Root.swift:70
Rejected: changing Listing's row title for no-persona chats — the Others screen shows the agent beside the name, which the reviewer called right
Rejected: the sheet height and round age findings — non-blocking, left for a design pass
Check: WorkflowTests expects the titles T impl, T loose, T nightly, where the old mapping gave fake-acp for the loose chat
Check: simulator Home light shot shows Bonjour probe in the Others row
Owner: Home's Others row now previews the chat titles inside it, like the design.

<sub>note on 7d960a0 · git log --notes=why</sub>

## Why (round 14)

Decided: round 13 merges origin/staging; #310 moved Home and the chat into SlotView, so Others and the Workflow back label move there too
Because: apple/Sources/CooeeUI/SlotView.swift:108
Decided: a Workflow round tap sends its own round delegate and pushes; #181's lane tap still replaces the open chat
Because: apple/Sources/CooeeFeatures/ChatPlugins.swift:30
Because: "tap a round to open that session as a normal chat"
Rejected: one shared open delegate — #181's test wants the lane tap to replace the path, while a round must return to the sheet
Decided: AppFeature's helpers sit in a same-file extension to stay under SwiftLint's 150-line type body
Check: apple-check passes every step up to swiftlint analyze, which segfaults locally (exit 139); CI apple-quality decides it
Check: e2e others and workflow shots at this head, now drawn through SlotView
Owner: Same feature, caught up with staging's new layout system; Others and the Workflow sheet now work inside it.

<sub>note on 7ed5a47 · git log --notes=why</sub>

## Why (round 16)

Decided: an unfolded issue's rounds hang on a line with their state dot, age and chevron; the current round adds a card with its last line, open chat and the label's PR or issue link
Because: docs/design/home-workflow/README.md:42
Because: apple/Sources/CooeeUI/WorkflowRounds.swift:4
Decided: the link comes from the workflow label's url, which GET /threads already sends; a pull url reads PR #n, an issue url issue #n
Because: apple/Sources/CooeeCore/Workflow.swift:13
Rejected: the last three tool lines — the thread list carries only each thread's last line; fetching every round's items is a new call for one card
Decided: Others gets its inline nav title and one footnote per workflow source, from the same Core pass that sorts the rows
Because: "so nothing is lost while drover is not installed"
Because: apple/Sources/CooeeCore/Workflow.swift:106
Decided: the outlined chip style moved out of LinkChip into one modifier both chips share
Because: apple/Sources/CooeeUI/BlockRounds.swift:137
Check: the e2e workflow shot shows the round line and its card; the others shot shows the nav title and the drover footnote
Check: apple-check passes up to swiftlint analyze, which segfaults locally (exit 139); Periphery and anti-slop's tests run by hand pass
Owner: The Workflow sheet's rounds and the Others screen now look like the approved design.

<sub>note on 5ade829 · git log --notes=why</sub>

