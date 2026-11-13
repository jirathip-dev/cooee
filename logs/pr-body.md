Closes #242

## For humans
Each persona now has a Style, set in its sheet: **Chat** or **Detailed**.
In Chat, a turn shows only the agent's reply, with one "Worked 1m · 5 steps" line under it that opens the list of steps. Approvals, cards and errors still show in the chat, and while the agent works, the bar says what it is doing right now. Chat personas are also asked to answer short and plain.
Detailed looks exactly as before. New personas start in Chat; the Conductor and other orchestrators, and workflow lanes, stay Detailed.

## What changed
- Server (`crates/cooee-core/src/personas.rs`): `style` (`chat` | `detailed`) on a persona. A file without one reads as `detailed` for an orchestrator and `chat` otherwise, so existing personas move to their default. In Chat, one line (`PLAIN`) goes after the brief of each new session. Documented in `docs/api.md`.
- App: `ChatItems.build(…, style:, working:)` folds a Chat turn's tool calls into one `.trace` row after its reply. Approvals, cards and notes or ended lines stay inline. While working, nothing is drawn for the steps, and `ChatItems.step` gives the pill its label. `TraceRow` opens `TraceSheet`, which lists the steps in order. The persona sheet has a segmented Style row. A thread with no persona (workflow lanes) draws Detailed.

## Decisions
- **The trace sheet shows what cooee has**: each step's title (the agent's own summary of the call, e.g. `$ cargo test`, `Read src/lib.rs`), its status and its order. shed's event stream has no thinking, tool input, output or diff events, at the pinned rev or any later one (`EventKind`: Text, ToolCall{id,title,status}, Plan, Approval…). Showing those needs shed to emit them first, which would be a follow-up issue there. For the same reason, "thinking" in the tests is the turn's tool steps; there is no thinking item to fold.
- **Errors** are the chat's note and ended lines (`The agent exited (1).`); they stay inline.
- **Style row hidden in still renders**, the same way #139's Pin toggle is: a segmented picker doesn't draw in `ImageRenderer`, and the v1 persona frame can't be re-recorded without Xcode. So no design frame changes. The screenshots come from the simulator e2e instead.
- Room members are personas, so a Chat member's first room turn also gets the line (`tests/rooms.rs` updated).

## Round 2 (merge of origin/staging + e2e fix)
- Merged `origin/staging` (#253's outbox, #216's note ids). `ChatItems.build` takes `outbox` with `style` and `working`; the brief tests expect `- [id] note` lines, then the Chat line.
- **e2e hang at d9bb27e:** `LiveTests` waits for `slow 3` to stream. Coder is a new persona, so it is Chat and its first prompt is the Chat line then `slow 3`. The fake agent matched `slow ` only at the prompt's start, so it answered `Hello` and the stream never ended (the job's 1h timeout). The fake agent now reads `slow N` from the prompt's last paragraph, the message after any brief. Probed against `apple/e2e/serve.sh` with curl: before, no words and the reply `Hello`; after, `w0 w1 w2`.
- **`ui` red at d9bb27e:** apt could not reach security.ubuntu.com and the job ran 2h15m; no code involved.

## Round 3 (merge of origin/staging again)
- Merged `origin/staging` (rooms #157, #237, #150, #288, #294). Conflicts kept both sides: `Item.id` lists `.trace` with staging's `.said` and `.divider`; `APIPersona.init` takes `machine`, `canMessage` and `style`; `ChatFeature.drawn` uses `RoomItems.build` for a room, else `ChatItems.build(…, style:, working:)`. `Item.plainText` returns nil for `.trace`, the same as for `.tools`.
- With staging's UI tests added, `LiveUITests.swift` went over SwiftLint's 250-line file and 25-line function limits. This PR's `chatStyles` and `commands` moved to `LiveUITests+Styles.swift`, and a shared `open(app, new, persona)` replaces three copies of "tap New chat, pick the persona, focus the box". The steps the test runs did not change.

## Round 4 (merges of origin/staging)
- Merged `origin/staging` twice more (#336, then #279 and #354). Both merged cleanly; this PR's code didn't change.

## Evidence (head f827e29b4b7999f50b9f05a39af26aa7e36b7f82)
- `cargo fmt --all --check && cargo clippy --workspace --all-targets -q -- -D warnings` → 0
- `cargo test --workspace` → 0
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` → 0
- `scripts/apple-check.sh` → 0 (swift format, anti-slop, SwiftLint, Swift 6 build, tests with coverage: Core 98.0%, API 97.3%, Features 97.4%, swiftlint analyze, Periphery, anti-slop's own tests)
- `scripts/line-cap.sh` on a clean checkout of the head → 0 (non-test 27107 / 27500, apple 9371 / 13000). Run inside the worktree after a Swift build, it also counts the Swift package checkouts in the untracked `apple/.build` folder.
- `scripts/neutral-core.sh`, `scripts/hygiene.sh`, `scripts/dep-direction.sh` → 0
- Not run locally: `cargo deny`, `topcoat fmt` (not installed) and the simulator e2e (CI).

Simulator screenshots (CI `e2e` job, `simulator-shots` artifact): `chat-style-chat-*`, `trace-*`, `chat-style-detailed-*` (the same `commands` turn in Coder, Chat, and Scout, Detailed), `persona-*` (Style row set to Detailed).

src/ net +34 (Rust non-test), apple +136 (budget: none stated). The app side is the trace row and sheet, the fold and the Style row.

## Why (round 2)

Decided: round 2 merges origin/staging; ChatItems.build takes staging's outbox with this PR's style and working
Because: apple/Sources/CooeeCore/Item.swift:69
Decided: the fake agent reads `slow N` from the prompt's last paragraph, after any persona brief
Because: crates/cooee/tests/support/mod.rs:767
Because: "the persona is told to answer short and plain"
Rejected: making the e2e Coder Detailed — LiveUITests needs Coder in Chat for the fold and trace screenshots
Check: e2e passes; at d9bb27e LiveTests hung because Coder's first prompt began with the Chat line, not `slow 3`
Check: the ui red at d9bb27e was apt failing to reach security.ubuntu.com, no code involved
Owner: Same feature; this round catches up with staging and fixes the test agent that made the app's live test hang.

<sub>note on 41a66b5 · git log --notes=why</sub>

## Why (round 4)

Decided: round 3 merges origin/staging; Item.id and Item.plainText cover .trace beside staging's .said and .divider
Because: apple/Sources/CooeeCore/Item.swift:58
Decided: ChatFeature.drawn uses RoomItems.build in a room, else ChatItems.build with this PR's style and working
Because: apple/Sources/CooeeFeatures/ChatFeature.swift:35
Decided: the chat-style UI test and a shared open() helper move to LiveUITests+Styles.swift
Because: apple/UITests/LiveUITests+Styles.swift:6
Rejected: raising SwiftLint limits — the repo rule caps Swift files at 250 lines
Check: apple-check passes at the head; tests/setup.rs fails only on hosts whose ephemeral ports exceed 64535
Owner: Same feature; this round catches up with staging again and keeps the app's tests within the lint limits.

<sub>note on 683fbdd · git log --notes=why</sub>

