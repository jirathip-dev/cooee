Closes #230

## For humans
The persona sheet's Agent chip now opens a menu of the agents installed on your Mac, that agent's models and its effort levels. You no longer type them. Picking another agent reloads its models and efforts.
Folder opens a browser of your roots, with sub-folders, search and scratch. It can't go outside the roots.
An empty list says why (for example: no agents found, run cooee doctor).

## What changed
- Server: `GET /agents`, the installed agents with the efforts each provider offers (`docs/api.md`). Models come from `GET /models` and folders from `GET /folders`; neither changed.
- App: the chip is a native `Menu` with inline Agent / Model / Effort pickers. Effort is hidden when the agent offers none, and Model shows short names first, then pinned ids. The Folder row opens `FolderBrowser`, and the reducer refuses any path outside the roots (`..` is standardised first). Server: `GET /folders?path=` also lists a configured root, or a folder inside one, spelled from the root as listed (it only allowed the home folder, so a root outside home couldn't be opened).
- The free-text Agent/Model/Effort/Folder fields, the popover and its `agentOpen` state are gone, along with `SettingRow`'s edit field.
- Tests: the Rust `model.rs` test covers `/agents`. Swift tests: the three lists and folders decode from the server; changing the agent reloads models and efforts; the saved persona carries the picked values; the folder browser never leaves the roots. LiveUITests now taps the chip and the folder browser and keeps `persona-menu-*` and `persona-folders-*` screenshots, and LivePersonaTests checks that the saved persona's agent and folder reached the server.

src/ net +263 (no budget given): a server route, three client calls, the reducer's choices state, a new menu and browser view, and browsing inside a root that sits outside the home folder.

## Evidence (head 3b06539320086e32557641e97ea0e77264aac92c)
- Fix round 3b06539 (the `e2e` FAIL: no folder browser after tapping Folder). Reproduced locally in the simulator against `apple/e2e/serve.sh`. Two causes:
  1. The Folder row is a plain-style button, so only its drawn parts took taps. With no folder set, the row's centre (where XCUITest, and most thumbs, tap) is empty space, and the tap did nothing. Temporary logging showed `browse` was never sent. The row now has a full-rectangle tap shape.
  2. `GET /folders?path=<root>` refused any root outside the home folder (the e2e root is in `$TMPDIR`), so the browser couldn't open `root`. The server now lists a root or a folder inside one, spelled from the root as listed. It still refuses anything outside both the home folder and the roots.
- `cargo test --locked -p cooee --test folders`: exit 0 at the head, and exit 101 with the hub change reverted (the new `browse_lists_a_root_outside_home_spelled_as_listed` fails).
- Local simulator run (iOS 27.2 sim, `xcodebuild … test-without-building`, dark): the persona steps pass (Agent menu, Folder → root → project → "Use project", `/root/project` shown, saved), with `persona-menu-dark`, `persona-folders-dark` and `persona-dark` captured. The run then fails at `LiveIconUITests.swift:20` (the app-icon test from #225, a file this PR doesn't touch, after SpringBoard's icon alert). That test is green on staging's CI (macos-26), so I take it as a local iOS 27.2 difference. CI's `e2e` is the proof.
- At 3b06539: `cargo fmt --all -- --check` exit 0; `cargo clippy --locked -p cooee-server -p cooee --all-targets -- -D warnings` exit 0; `sh scripts/neutral-core.sh` exit 0; `sh scripts/hygiene.sh` exit 0; `swift format lint --strict` on PersonaSheet.swift exit 0; `swift test --filter PersonaFeatureTests` exit 0 (5 tests); `scripts/line-cap.sh` on a clean export exit 0 (non-test 26725/27500, apple 7966/13000).
- `cargo test --locked --workspace` exit 101 once, only from `rooms.rs:375` (`two_members_naming_each_other_stop_at_the_cap_each_message`, 11 vs 12, a timing test). `cargo test -p cooee --test rooms` then passed 3 of 3 runs (exit 0). This PR doesn't touch rooms.
- Fix round 35620b3 (merge of staging 4c9f5266; the review FAIL was CI state at d1a75d4: `ui` red on `[390] chat.spec.mjs:131` (1,000-message perf window), web code this PR doesn't touch, and the macOS jobs still pending). No code change. At 35620b3: `cargo test --workspace` exit 0; `cargo fmt --check` exit 0; `scripts/neutral-core.sh` exit 0; `scripts/line-cap.sh` on a clean `git archive HEAD` export exit 0 (non-test 26576/27500, apple 7965/13000). CI's run at this head is the gate proof.
- Update round d1a75d4 (merge of staging f5bf384f; the review FAIL was CI still pending at f288a12): the merge brings in only `.herdsman/verify.md` and `crates/cooee-core/src/devices.rs` from staging, with no Swift change. `cargo test -p cooee-core devices && cargo test -p cooee --test model` exit 0 at d1a75d4. At f288a12, `ui` failed only `[1440] folders.spec.mjs:17` (removing a root in Settings), which passed at 390 in the same run. This PR doesn't touch roots or web code, so the fresh run at this head is the check.
- Fix round d1e4477 (red `dmg` at 0aa6d3c): Xcode 26.6's Release compile of CooeeUI crashed in IRGen (`SmallVector` overflow) after 0aa6d3c made the agent pick `@MainActor @Sendable` and passed that function straight as a `Binding` setter. It is now passed through a closure literal, `set: { pick($0) }`, like every other `Binding(get:set:)` in the app. `dmg` was green at e8d34c9, before that change.
- `xcodebuild … -configuration Release -destination 'generic/platform=macOS' build` (the `dmg` job's command, local Xcode 27.2): CooeeUI compiles for arm64 and x86_64. Overall exit 65, only from the "Login items" phase, because the cooee/shed binaries weren't built for this run. Xcode 27.2 does not crash at 0aa6d3c either, so CI's 26.6 `dmg` job is the proof.
- `sh scripts/apple-check.sh` at d1e4477: swift format, anti-slop, TODO, SwiftLint, the Swift 6 strict build, the tests and coverage (Core 98.2%, API 97.1%, Features 97.3%) passed. `swiftlint analyze` stalled on this host (Xcode 27.2 can't find the TCA macro plugins, in files this PR doesn't touch), so it and Periphery come from CI.
- Earlier rounds, unchanged by d1e4477 (Swift only): `cargo test --locked -p cooee --test model` exit 0; `cargo fmt --all -- --check` exit 0; `sh scripts/hygiene.sh` exit 0; `scripts/neutral-core.sh` exit 0.

Not verified locally: the Xcode 26.6 Release build (the `dmg` job) and the simulator screenshots. Both come from CI.

## Why (round 3)

Decided: the Agent picker's binding setter is a closure literal calling pick, not the stored @MainActor @Sendable function itself
Because: apple/Sources/CooeeUI/PersonaPickers.swift:28
Because: "Agent: the agents installed (the server's agent list)"
Decided: no other change; dmg passed at e8d34c98 and failed at 0aa6d3ca, whose only source change made pick @MainActor @Sendable
Rejected: reverting pick to a plain (String) -> Void — a Sendable setter literal could then not capture it under Swift 6 strict concurrency
Rejected: reproducing locally — the host has Xcode 27.2, whose compiler builds the Release app; the crash is Xcode 26.6's IRGen on CI
Check: the dmg job is green at the head; every other Binding(get:set:) in apple/Sources already passes a closure literal
Owner: Fixes the Mac app's release build, which the compiler crashed on after the last change to the agent picker.

<sub>note on d1e4477 · git log --notes=why</sub>

## Why (round 5)

Decided: merge origin/staging (f5bf384f) into the branch and push; no code change, the FAIL was only CI pending at f288a12
Because: crates/cooee-server/src/lib.rs:1115
Rejected: changing web/e2e/folders.spec.mjs — its 1440 root-removal step failed once, passed at 390 in the same run; this PR touches no roots or web code
Check: every check is green at the new head, ui included; macOS runners were queued for hours on staging too
Owner: Brings the persona sheet branch up to date so its checks can finish; nothing in the app changed this round.

<sub>note on d1a75d4 · git log --notes=why</sub>


