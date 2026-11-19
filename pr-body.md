Closes #613

## For humans
When shed restarts and a chat's agent is lost, sending a message now brings the agent back (same persona, same folder) and delivers the message. If the agent's CLI can pick up its old session, it does. If it can't, the chat shows one line saying the agent started fresh.
The "The agent is gone" line now has a Restart button that does the same without sending anything. Nothing else on the chat screen changes.

## What changed
- Server: the thread keeps the agent's ACP session id (`Thread.session`, saved each time a session comes up). The new `Hub::restart` (`hub/restart.rs`) starts the agent under a new terminal name with `shed run --acp … --resume <id>`. If resuming fails, or there is no id, it starts fresh, adds one note and sends the persona's brief before the next message. It then leaves the `ended` state (state `done`). It does nothing while the agent is running, so a second tap is harmless.
- `POST /threads/{id}/send`: a send refused because the terminal is gone (`no_such_terminal`, which used to be a 410) restarts the agent, then sends again once. `POST /threads/{id}/restart` is new. `retry`, `restart` and `send` share one recipe builder (`Again`) instead of copying it.
- Events replayed by a resumed session (`replay: true`) are skipped, so old turns aren't stored twice.
- An agent that exited, or a chat that was closed, still refuses a send as before. Only a gone terminal restarts.
- App: a Restart button on the "The agent is gone" line, shown only while the chat is `ended` (same pattern as the "Updated … View" line). It goes through `ThreadsClient.restart` → `POST /threads/{id}/restart`.

## Tests (evidence at head e8f5e9dbda54bc22e7efeac3aa93e41b72d2a846)
- Round 5 (review FAIL on 9e7cd5750: the `apple` job's xcodebuild found no "iPhone 17" simulator on the runner, exit 70. That destination comes from staging's .github/workflows/apple.yml, which this PR doesn't touch, so the runner Mac needs that simulator installed). Merged origin/staging (#647, no conflict). The merge made two changes needed: `ChatFeature.outbox` went over SwiftLint's 25-line function limit, so its retry case moved to its own function, and the restart test now expects the toast #647 added after a failure. At e8f5e9dbda54bc22e7efeac3aa93e41b72d2a846: `scripts/apple-check.sh` → 0, `ci/gates/clippy.sh` → 0, `ci/gates/comments.sh` → 0, `cargo test -p cooee --test chat` → 0, `ci/gates/cap.sh` in a clean checkout → 0 (Rust 30661 / 32000, Apple 18452 / 18700). `cargo fmt --all --check` → 0. `ci/gates/fmt.sh` → 127 only because `topcoat` isn't installed here.
- Round 4 (review FAIL on 393a5f98a: the `apple` job's xcodebuild found no iPhone 17 simulator because "CoreSimulator is out of date. Current version (1051.55.0) is older than build version (1174.21.0)", exit 70. That is the CI runner's simulator install, not this code). Merged origin/staging (#651, #652, #653, no conflict) at 9e7cd575008e3c74e1ab5296742e279f8676f92f: `cargo fmt --all --check` → 0, `ci/gates/clippy.sh` → 0, `ci/gates/comments.sh` → 0, `cargo test -p cooee --test chat` → 0, `scripts/apple-check.sh` → 0. `ci/gates/test.sh` → 101: only staging's perf test `with_500_threads_of_1000_items_the_list_answers_within_300_ms_of_start` failed (355 ms), at a host load average of 145; rerun alone, `cargo test -p cooee --test perf` → 0. `ci/gates/cap.sh` → 1 here only because it also counts the ignored local `apple/.build` package checkout; leaving that out, Apple ≈18.1k / 18700 and Rust 30661 / 32000.
- Round 3 (review FAIL on e7737053a: the `apple` job stopped at "You have not agreed to the Xcode license agreements", exit 69. That is the CI runner's Xcode setup, not this code). Merged origin/staging (#645, #650, no conflict) at 393a5f98a2b42cd06e243996e0f28cc93610dacb: `cargo fmt --all --check` → 0, `ci/gates/clippy.sh` → 0, `ci/gates/comments.sh` → 0, `cargo test --locked -p cooee --test chat --test reliability` → 0, `cargo test --locked --workspace` → 0, `scripts/apple-check.sh` → 0.
- Round 2 fix (review on a2ce5d32e): after a real shed restart, the new shed numbers its events from 1 again, so the hub dropped every event from the restarted agent and no reply ever showed. `Hub::restart` now sets the chat's applied seq to 0. `Chat::seq`, which seeds that value when cooee starts, now counts only items after the chat's last "ended" line, so a later cooee restart doesn't drop the new shed's events either. The fake shed's `vanish` now also starts its event numbering again, as a real shed does, and the restart test also gets an answer after a cooee restart.
- At 2faf8d126 (only the test changes): `cargo test --locked -p cooee --test chat gone` hangs. All three gone tests were still waiting for the reply after 60 s (killed).
- At 12c10904e (the fix): `cargo test --locked -p cooee --test chat` → exit 0 (15 passed, 1 ignored). `cargo test --locked -p cooee --test reliability --test workflow` → exit 0 (17 + 3 passed). `cargo test --locked --workspace` → exit 0. `ci/gates/clippy.sh` → 0, `ci/gates/comments.sh` → 0. `ci/gates/cap.sh` in a clean checkout → 0 (Rust 30397 / 32000, Apple 17629 / 18700). `cargo fmt --all` made no changes. `topcoat` isn't installed here, so `ci/gates/fmt.sh` → 127 (CI runs it).
- Merged origin/staging (#648, #649, Apple only, no conflict) at e7737053a662da67e04b105451b02a641b34bfaa: `scripts/apple-check.sh` → exit 0.
- Merged origin/staging (through #646) at a2ce5d32e. It conflicted only in FrameClientTests.swift, and both tests are kept (staging's file test and this branch's restart test). At a2ce5d32e: `scripts/apple-check.sh` → exit 0, `cargo fmt --all --check && ci/gates/clippy.sh && cargo test -p cooee --test chat` → exit 0 (15 passed, 1 ignored).
- Merged origin/staging (through #643, merge head 69960d4e5). Two files conflicted. ThreadsClient+Live.swift: kept staging's shared `client(server)` and added `restart` to it. Chat.swift: kept staging's `.equatable()` ItemView and this branch's `restart:` argument. ItemView's `==` now also checks whether Restart is offered, so the button goes away once the chat stops being ended. `scripts/apple-check.sh` at the first merge commit → exit 0 (#643 touches only Rust server files). At 69960d4e5: `cargo fmt --check` → 0, `ci/gates/clippy.sh` → 0, `cargo test -p cooee --test chat` → 0 (15 passed, 1 ignored), `cargo test -p cooee-server` → 0.
- Merged origin/staging again (ae4836bf2): staging added only a doc-comment line in config.rs and docs. `cargo test -p cooee --test chat` at ae4836bf2 → exit 0 (15 passed, 1 ignored).
- Merged origin/staging. It conflicted only in HeaderRenderTests.swift (kept both tests); `scripts/apple-check.sh` at b0cb5b41a → exit 0. Staging touched only Swift since 3266d2c7, so the Rust results below (run at 3266d2c7) carry over.
- First commit ca3bd3ceb has only the failing tests. Running `cargo test -p cooee --test chat --test reliability gone` there gives exit 101, with sends refused 410 and restart returning 404.
- `cargo test -p cooee --test chat --test reliability` → exit 0 (the new tests are `a_send_to_a_gone_agent_starts_a_fresh_one_in_the_chat_and_delivers`, `a_gone_agent_that_reported_its_session_resumes_it_on_send` (asserts `--resume sess-1`) and `restart_on_the_gone_line_starts_the_agent_and_leaves_ended`).
- `ci/gates/test.sh` (whole workspace) → exit 0. One earlier run failed `r10_an_ask_made_during_the_start_is_listed_and_answerable` once under load; it passed 3 of 3 runs alone and the full rerun was green.
- `ci/gates.sh clippy comments bans neutral hygiene cap` → clippy, comments, bans, neutral and hygiene pass. Locally, cap counts SwiftPM's `apple/.build` checkouts. Counting tracked files gives apple 17023 / 18000 and Rust 29521 / 32000.
- `cargo fmt --all --check` → exit 0. `topcoat fmt` isn't installed on this host, so CI runs it.
- `scripts/apple-check.sh` → exit 0 (swift format, anti-slop, SwiftLint, Swift 6 build, tests with coverage). New tests: `restartOnTheGoneLineStartsTheAgentAgain`, `restartPostsToTheThreadsRestart` and `theGoneLineDrawsRestartOnlyWhileTheAgentIsGone`.

## Screenshots (390 wide)
`theGoneLineDrawsRestartOnlyWhileTheAgentIsGone` writes `chat-gone-restart-390.png` (the gone line with Restart) and `chat-restarted-390.png` (after a restart: no Restart button, plus the fresh-start line) into `COOEE_RENDERED`. CI uploads them in the apple job's `rendered-frames` artifact.

## Budget
src/ net +178 (no budget given in the issue). Most of it is the restart path in the hub and the shared route helper.

## Not verified
- A real agent CLI's `session/load` through real shed. The fake shed only checks that `--resume <id>` is passed.
- The live simulator e2e run.

## Why (round 3)

Decided: merged origin/staging (#645 startup, #650 report mode); no conflict, Rust gates and apple-check green at the merge
Because: crates/cooee-server/src/hub/restart.rs:74
Decided: no code change for the apple FAIL; the CI runner stopped at the Xcode license prompt (exit 69) before building
Because: apple/Sources/CooeeUI/Items.swift:36
Rejected: changing the workflow to accept the license — it needs sudo on the runner host, an owner call
Check: the apple job reruns on this push; if it fails again at the license step, the runner needs `sudo xcodebuild -license accept`
Owner: The branch now includes the latest staging; the CI Mac's Xcode license may still need accepting on the runner.

<sub>note on 393a5f9 · git log --notes=why</sub>

## Why (round 4)

Decided: merged origin/staging (#651, #652, #653); no conflict, Rust gates, chat tests and apple-check green at the merge
Because: crates/cooee-server/src/hub/restart.rs:74
Decided: no code change for the apple FAIL; the runner's CoreSimulator (1051.55.0) is older than its Xcode needs, so no iPhone 17 simulator exists there
Because: apple/Sources/CooeeUI/Items.swift:36
Rejected: changing the workflow's simulator destination — the runner's simulator runtime needs updating on the host, an owner call
Check: the apple job reruns on this push; if xcodebuild again says CoreSimulator is out of date, the runner Mac needs its Xcode components or simulator runtime updated
Owner: The branch now includes the latest staging; the CI Mac's iPhone simulator is out of date and may need updating before the apple check can pass.

<sub>note on 9e7cd57 · git log --notes=why</sub>

