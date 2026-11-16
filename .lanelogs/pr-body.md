Closes #270

## For humans
A persona can now be set up by chat. The persona form has a "Set up by chat instead" link (a saved persona has "Refine by chat"); it saves the persona and opens a chat in which the persona interviews you and writes its own Character and Instructions.
In that setup chat each change is saved at once and shows an "Updated …" line with View. In any other chat, a persona that wants to change itself shows you a card with the diff, and nothing changes until you approve.
Changes reach the persona's next session, not the one in progress.

## What changed
- **Setup mode**: `POST /threads` takes `setup: true` (a 400 without a persona); the thread keeps it, `Row.setup` shows it, and its brief gets one interview line after the persona's Character, Instructions and notes (`personas::SETUP`). The brief is still built only at session start.
- **`update_persona {character?, instructions?}`** on cooee's own MCP server → `POST /mcp/agent/persona` (`crates/cooee-server/src/hub/setup.rs`). It acts only on the calling thread's own persona: the arguments have no field that names a persona (`deny_unknown_fields`), and the thread's token is checked. In a setup thread it saves at once and posts `Updated Character` / `Updated Instructions` notes. In any other thread it raises an approval card (`form: "persona"`, `detail` = the diff, `approve` / `deny`), and saves only on approve. Deny saves nothing, and so does a card left unanswered.
- **Apple**: `PersonaFeature.chat` saves, then delegates `.chat(persona)`. `AppFeature` opens a `ChatFeature` with the new target `.setup(persona)`, whose first message starts the thread with `setup: true`. The sheet's link reads "Set up by chat instead" (new) or "Refine by chat" (saved). An `Updated …` note has a View button that opens the persona sheet.
- `docs/api.md`: `setup` on `POST /threads` and `Row`, the tool, and `/mcp/agent/persona`.

## Decisions
- `update_persona` takes no persona argument, so a thread can't name another persona. An unknown field is a 400 (the tool's error), not a silent ignore.
- The new sheet link is hidden in design stills (`look.still`), like "Pin to Home" and the Clear memory menu, so the `persona-*` design frame stays unchanged. This host has no Xcode and can't re-record snapshots.
- An unanswered card is held in memory, the same as `propose_server`'s; after a restart, approving it says it ran out.

## Evidence (head 469d8195c4b347f445777cd357b73dcbdfc03bfb)
Update round 6: merged orig## Evidence (head b5978d16f74ce954ec5b8e431cdfcd98f546a8e4)
Update round 7: merged origin/staging (345 commits: hub split into submodules, PWA web client removed #189, one comment rule #201, persona panel/sheet as core screens #314, machine picker on new chats).
Resolved: `hub.rs` (staging's split; `setup` re-applied in `hub/start.rs`, `hub/threads.rs`, the persona card arm in `hub/cards.rs`, `updates` beside staging's `registry`), `lib.rs` and `relaunch.rs` (`attach`/`retry` moved; they pass `t.setup`), `docs/api.md` (`setup` beside staging's `asked_by`), `web/src/agents.rs` and `web/ts/chat.ts` (deleted with the web client, so this PR's web diff colouring goes too), Apple `start`/`newThread` (now `machine` and `setup`), `PersonaFeature` (staging saves via the core screen's `save` action; `chat` rides the same path), `PersonaActions.chat` in `BlockPersonaEdit`/`PersonaScreen`, and the chat's View button now sends staging's `.delegate(.persona(id))` (the PR's `viewPersona` case removed). The PR's private-item doc comments are stripped per #201. Two test expectations follow staging's persona style line (`contains`/`starts_with` instead of exact).
- `cargo fmt --all --check` → 0
- `sh ci/gates/clippy.sh` → 0
- `cargo test --workspace` → 0
- `scripts/comments.py` → 0
- `sh ci/gates/{doc,neutral,hygiene,ci-scripts,lessons,themes,deps}.sh` → 0
- `sh scripts/line-cap.sh` → non-test 20280 / 27500; apple count locally includes the ignored `apple/.build/` cache (≈13.9k without it, cap 14316)
- `cd apple && COOEE_STRICT=1 swift test` → 0
- `swift format lint --strict` on the touched Swift files → 0
- **Not run locally:** `topcoat fmt` (not installed here), xcodebuild iOS/mac, apple-quality (SwiftLint, Periphery, coverage); CI runs them.

de staging's new shed_approve(&live, &terminal, ..) signature
Because: crates/cooee-server/src/hub.rs:1713
Decided: ThreadsClient keeps start's setup argument beside staging's new room and seat closures
Because: apple/Sources/CooeeFeatures/Clients.swift:31
Rejected: re-run CI without merging — the PR was CONFLICTING with staging, and the red checks were runner cancellations plus an unrelated ui timing test
Check: apple/Sources/CooeeApp/Live.swift wires start with setup, and room and seat as staging wrote them
Owner: Same feature; this round catches up with staging, which added rooms and machine checks.

<sub>note on 28067ba · git log --notes=why</sub>

## Why (round 3)

Decided: merge origin/staging again (clean) and re-run CI; no code change this round
Because: crates/cooee-server/src/hub.rs:270
Rejected: a code fix for the FAIL — the red checks at 28067ba were runner shutdowns (cancelled), not test failures
Rejected: committing the previous lane's wip snapshot — it held only local log files, which never go in git
Check: CI at the new head is green, including build (generic/platform=iOS Simulator)
Owner: Same feature; this round only catches up with staging and re-runs the cancelled checks.

<sub>note on ba4a1b3 · git log --notes=why</sub>

