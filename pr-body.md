Closes #201

## For humans

Code comments are gone across the Rust, Swift and TypeScript code, except docs on public interfaces, safety notes and TODOs that name an open issue.
CI now runs a check that refuses any new comment that breaks this rule, so the code says it with names, types and tests instead.
Nothing the app does changes.

## What changed

- `scripts/comments.py` (stdlib Python, string- and regex-aware lexer) checks every tracked `.rs`, `.swift`, `.ts`, `.js`, `.mjs` file. No `//` or `/* */`. `///` (TS `/** */`) only on a public declaration: Rust `pub`, Swift `public`/`open`, TS `export`, or a case, field or member of such an enum, trait, protocol or interface. Allowed: `// SAFETY:`, `// TODO(#N)`, Swift's `// swift-tools-version:` and `// biome-ignore <rule>: <why>`.
- Rust `//!` counts as public only on a crate root or a `pub mod`. The workspace's `missing_docs` lint needs those docs.
- `ci/comments-test.sh` runs fixtures. Passing: `///` on public items, a crate root's `//!`, SAFETY and TODO comments, and `//` inside a string or regex. Failing: a plain `//`, a `///` on a private Rust or Swift item, a `/* */` block, a stray `//!`.
- `just comments` runs both, plus checks that each `TODO(#N)` names an open issue (same as apple-check). The TODO numbers come from the checker's own lexer (`scripts/comments.py --todos`), so a `// TODO(#N)` inside a string literal (the anti-slop Swift test fixture) is not counted; `ci/comments-test.sh` covers that. It is added to `just check` and to CI's msrv job.
- `.herdsman/rules.md`: the Swift-only comment line becomes one rule for all three languages, naming its enforcers.
- Stripped 2,338 lines across 151 files, mechanically, with no renames. Then `cargo fmt`, `biome check --write`, and leading/extra blank lines dropped. `docs/plugins.md` drops the same 4 lines from the hello plugin it embeds verbatim (`plugins.rs` compares the two).
- `apple/` Swift needed no change: anti-slop `no-comments` already holds there, and the new check agrees with it.

`src/ net -1049` (scripts/line-cap.sh non-test lines 24477 → 23428; no budget given): the stripped comments.

## Evidence (update round, at 088e9969ad6483d06c90c7861a97d544a06f4cc0)

Merged origin/staging (6d75396f..ecdeb73d: #293, #336, #355, #329, #245, #278). Conflicts in `login.rs`, `tests/plugin_manager.rs`, `justfile` and `web/e2e/chat.spec.mjs` keep staging's code with rule-breaking comment lines dropped; the justfile `check` list keeps staging's `ci-scripts` plus `comments`. Staging's new rule-breaking doc comments in `tests/group.rs`, `manage.rs` and `tests/support/mod.rs` (4 lines) are stripped.

- `mise exec just -- just comments` → exit 0 (`no comments break the rule`, `comments test ok`)
- `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings` → exit 0
- `cargo test --workspace` → exit 0
- `git diff -w origin/staging HEAD` on the 7 touched files adds only the branch's own `comments` recipe lines in the justfile

## Evidence (update round, at a5127c7802934f7b0af56daa89c5b832801d588f)

Merged origin/staging (6d75396f); the `crates/cooee-server/src/manage.rs` conflict keeps staging's code (the new health-check supervisor and its tests) with its 44 rule-breaking comment lines dropped.

- `python3 scripts/comments.py` → exit 0 (no comments break the rule)
- `mise exec just -- just comments` → exit 0
- `cargo fmt --check` → exit 0
- `cargo test -p cooee-server manage` → exit 0 (4 passed)
- `cargo test --workspace` → exit 0
- `git diff -w origin/staging HEAD -- crates/cooee-server/src/manage.rs` adds no lines, only deletes comment lines

## Evidence (fix + update round, at 006e68ac3d80de29dbd4483dc676e0d10550bd20)

The review FAIL at 957f2ca was CI infra, not code: `build (generic/platform=iOS Simulator)`, apple-quality and e2e were still queued for a macOS runner, and the ubuntu `stable`/`msrv` jobs died with "The runner has received a shutdown signal" mid-test (every test that ran passed). No code change for it. Merged origin/staging (e6317df7, #255): a clean merge touching only 5 Swift files, which are byte-identical to staging's (`git diff origin/staging HEAD` on them is empty). Rust is unchanged since 957f2ca, so its gates there still hold.

- `just comments` (comments.py, ci/comments-test.sh, TODO(#N) open-issue loop) → exit 0
- `scripts/line-cap.sh` (non-test 25875 / 27500, apple 9005 / 13000), `scripts/hygiene.sh` → exit 0 each

## Evidence (update round, at 957f2ca695b0e933c2b98438d6e6e518c14195fd)

Merged origin/staging (7f6b7d19: #150 machine check, #157/#208 room screens). The conflicts in `crates/cooee-server/src/lib.rs`, `login.rs`, `crates/cooee/src/main.rs`, `tests/rooms.rs` and `tests/support/mod.rs` keep staging's code with every rule-breaking comment line dropped. Staging's new rule-breaking comments in `crates/cooee/src/machine.rs`, `tests/login.rs` and `tests/machine_check.rs` are stripped too (942 checker findings in all, removed mechanically with the checker's own lexer). A follow-up commit drops the 3 leading blank lines that stripped file-header comments left behind. `git diff -w origin/staging HEAD` on those files adds no lines; the only "added" line is the existing rustfmt reflow of an `assert!` whose inner comment was removed.

- `python3 scripts/comments.py` → exit 0; `bash ci/comments-test.sh` → exit 0; TODO(#N) open-issue loop → exit 0
- `cargo fmt --all --check` → exit 0
- `cargo clippy --workspace --all-targets -- -D warnings` → exit 0
- `cargo test --workspace` → exit 0
- `scripts/line-cap.sh` (non-test 25875 / 27500), `scripts/neutral-core.sh`, `scripts/hygiene.sh` → exit 0 each

## Evidence (fix + update round, at 485bc338c88517e16a8b8258c124e07d36665d39)

The review FAIL at d0b1c75 was `build (generic/platform=iOS Simulator)`: every macOS job (both builds, apple-quality, e2e, mac-setup, dmg, macOS stable/msrv) was cancelled after ~14 min with zero steps run, never picked up by a runner; no code change for it. Merged origin/staging (3ffbe659, #181): the `crates/cooee-server/src/hub.rs` and `crates/cooee/tests/plugins.rs` conflicts keep staging's code with every rule-breaking comment line dropped (130 in hub.rs, 33 in plugins.rs), plus 2 in staging's new `crates/cooee/tests/chat.rs` lines. `git diff -w origin/staging` on those three files adds no lines.

- `python3 scripts/comments.py` → exit 0
- `bash ci/comments-test.sh` → exit 0
- `cargo fmt --all --check && cargo clippy -p cooee-server -p cooee --all-targets -- -D warnings` → exit 0
- `cargo test -p cooee-server -p cooee` → exit 0

## Evidence (update round, at d0b1c756a3158f58ca0af106601364d72c9828c3)

Merged origin/staging (4c9f5266, #226). The personas.rs, manage.rs and check.rs conflicts keep staging's code. 46 lines that break the comment rule are dropped, in those three files and in tests/plugin_manager.rs and tests/plugins.rs. `git diff -w origin/staging` on those five files adds no lines.

- `python3 scripts/comments.py` → exit 0
- `bash ci/comments-test.sh` → exit 0
- `cargo fmt --all` → exit 0
- `cargo clippy --workspace --all-targets -- -D warnings` → exit 0
- `cargo test --workspace --no-fail-fast` → exit 0. The first run failed 5 `tests/setup.rs` tests with exit 101: `cooee: no port + 1000 for the terminal plugin`. The test binds port 0, macOS was handing out ports above 64535, and the code's `port + 1000` overflowed a u16. An origin/staging checkout failed the same 5 tests at the same time. The rerun passed.

## Evidence (update round, at 4f4e0d922e2235c58025305cda0d4d6255a86fe3)

Merged origin/staging (f5bf384f). One conflict (`crates/cooee-core/src/devices.rs`): kept staging's code (#294's cache test) and dropped its rule-breaking `///` lines on private test fns and on the private `Stamp` struct.

- `python3 scripts/comments.py` → exit 0
- `bash ci/comments-test.sh` → exit 0 (`comments test ok`)
- `scripts/comments.py --todos` → prints nothing (no TODO(#N) to check)
- `cargo test -p cooee-core devices` → exit 0 (7 passed)
- `cargo fmt --check` → exit 0
- `git diff -w origin/staging HEAD -- crates/cooee-core/src/devices.rs` → 30 deletions, 0 additions (comment lines only)
- `just` itself is not installed on this host, so the `comments` recipe's steps were run one by one

## Evidence (update round, at ba9476b63e3415cda8a7b776f769a544b35b8128)

Merges origin/staging (597473a8: #224, #267, #282, #247, #248, #238). Conflicts in `crates/cooee-server/src/hub.rs`, `lib.rs`, `push.rs` and `web/e2e/look.spec.mjs` keep staging's code with every rule-breaking comment line dropped; the 82 rule-breaking comment lines staging brought in elsewhere (routines in cooee-core/cooee-server/tests, `web/src/agents.rs`, `web/src/routines.rs`, `web/ts/chat.ts`, `docs/design/app-icons`, `docs/design/screen`) are stripped the same way.

- `python3 scripts/comments.py` → exit 0; `just comments` → exit 0
- `cargo fmt --all` → exit 0 (`just fmt` exit 127 locally: `topcoat` not installed on this host; its `cargo fmt --check` step passed)
- `just clippy` → exit 0; `just test` → exit 0; `just ts` → exit 0
- `just hygiene` / `just neutral` / `just cap` → exit 0
- `node --check` on the six stripped JS/MJS files → exit 0
- `git diff -w origin/staging HEAD` on the four conflicted files: no added lines, only deleted comment lines

## Evidence (fix round, at 26b0fac5cc4ce739c8a2bd7edd120d4aa17bb2c9)

The review FAIL was CI pending on the macOS runners at 968f0a2 (queued, not failing). Meanwhile staging moved (d0bff277, #268/#271), so this round merges origin/staging (clean) and strips the 21 rule-breaking comment lines in the merged persona-text mockup scripts (`docs/design/persona-text/pt.js`, `shoot.mjs`).

- `scripts/comments.py` → exit 0
- `ci/comments-test.sh` → exit 0
- `node --check docs/design/persona-text/pt.js` → exit 0; `node --check docs/design/persona-text/shoot.mjs` → exit 0
- `git diff -w` of the strip commit: 21 deletions, 0 additions

## Evidence (update round, at 968f0a2c0ae22b8bc039119a718277a3e87b7e47)

Merged `origin/staging` (#149 sessions on the chosen machine). Conflict in `crates/cooee-machines/src/html.rs`: staging's side kept (it makes `ago` `pub`, so its `///` is allowed). Staging's new comments that break the rule stripped (26 lines: `html.rs` incl. its `//!` on a private mod, `hub.rs`, `tests/remote.rs`, `tests/personas.rs`, `tests/support/mod.rs`). `git diff -w origin/staging HEAD` on those files adds no lines, only deletes comment lines.

CI note: the previous FAIL was `build (generic/platform=iOS Simulator)` still queued on macOS runners at de3c52b, not failed.

| command | exit |
|---|---|
| `python3 scripts/comments.py`, `ci/comments-test.sh` | 0 each |
| `cargo fmt --all --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked --no-fail-fast` | 0 |
| `scripts/line-cap.sh`, `scripts/neutral-core.sh`, `scripts/hygiene.sh` | 0 each |
| `cd web && npm run check` | 0 |

## Evidence (update round, at de3c52b1f1f86d9302cc92b4469d8efdef44adff)

Merged `origin/staging` (#218 skills, #253, #232, #251). Conflicts in `crates/cooee-server/src/hub.rs` and `crates/cooee-server/src/lib.rs`: staging's code kept with its rule-breaking comment lines dropped; staging's new rule-breaking comments elsewhere (`skills.rs`, `tests/skills.rs`, `tests/chat.rs`, `docs/design/home-v2` and `native-feel` JS) stripped (80 lines found by the checker, plus the conflicted hunks' own comment lines). `git diff -w origin/staging HEAD` adds no lines in the merged Rust files; it only deletes comment lines.

| command | exit |
|---|---|
| `python3 scripts/comments.py`, `ci/comments-test.sh`, TODO(#N) open check | 0 |
| `cargo fmt --all --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` | 0 |
| `scripts/line-cap.sh`, `scripts/neutral-core.sh`, `scripts/hygiene.sh` | 0 each |
| `cd web && npm run check` | 0 |

## Evidence (update round, at 43df8e89ef65aa26eb0d71bc45b3cac65663c7a6)

Merged `origin/staging`. Conflicts in `crates/cooee-server/src/lib.rs`, `crates/cooee-server/src/mcp.rs` and `crates/cooee/tests/personas.rs`: staging's code kept, and every comment line in them that breaks the rule stripped (162 + 12 + 48 lines). `git diff -w origin/staging` on those three files is only deleted comment lines.

| command | exit |
|---|---|
| `python3 scripts/comments.py`, `ci/comments-test.sh`, TODO(#N) open check | 0 |
| `cargo fmt --all --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `cargo test --workspace --no-fail-fast` | 0 (a first plain run hit `chat_first` `an_agent_has_one_ongoing_chat…` once; that file differs from staging by comments only, and 3 reruns plus the full run passed) |
| `scripts/line-cap.sh`, `scripts/neutral-core.sh`, `scripts/hygiene.sh` | 0 each |
| `cd web && npm run check` | 0 |

## Evidence (update round, at 3495a33acb6cf4e2ffcacd646ebba402bcf74c40)

Merged `origin/staging` (#217 persona history tool). `crates/cooee-server/src/lib.rs` conflict: staging's code kept, comments dropped. Staging's new comments that break the rule stripped (6 lines: `crates/cooee-core/src/history.rs`, `crates/cooee/tests/history.rs`); `git diff -w origin/staging` on both files is only those deleted comment lines.

| command | exit |
|---|---|
| `python3 scripts/comments.py`, `ci/comments-test.sh` | 0 each |
| `cargo fmt --all --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 |
| `scripts/line-cap.sh`, `scripts/neutral-core.sh`, `scripts/hygiene.sh` | 0 each |
| `cd web && npm run check` | 0 |

## Evidence (update round, at a67e21511c43ab0eca5c48af1fefd946b3805c15)

Merged `origin/staging` (#159 personas). Conflicts in `crates/cooee-core/src/personas.rs` and `crates/cooee/tests/personas.rs`: staging's code kept, and the 44 comment lines that break the rule stripped (7 and 37). `git diff -w` against staging for both files is deleted comment lines only.

| command | exit |
|---|---|
| `scripts/comments.py && ci/comments-test.sh` | 0 |
| `cargo fmt --all --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 |
| `scripts/line-cap.sh`, `scripts/neutral-core.sh`, `scripts/hygiene.sh` | 0 each |

CI note: the `ui` job's `chat.spec.mjs` "1,000 messages … no long tasks" check fails intermittently on staging too (same assertion in 5 of its last 12 `ci` runs, including the staging head `9ab48cbf`); this PR changes only comments in that code and test.

## Evidence (update round, at 3312615442e64c064124c2fb735b9ccd425a9420)

Merged `origin/staging` (#167 secrets, #184 fleets config). Conflicts in `crates/cooee-core/src/mcp.rs` (staging's move of the keychain code to `secrets.rs`), `crates/cooee/src/keychain.rs`, `crates/cooee/tests/plugins.rs` and `docs/design/screens-as-data/sd.js`: staging's code kept, comments dropped. Staging's new comments that break the rule were stripped (34 lines: `crates/cooee-core/src/secrets.rs`, `crates/cooee/tests/secrets.rs`, `crates/cooee/tests/setup.rs`, `docs/design/fleets-config/shoot.mjs`, `docs/design/screens-as-data/sd.js`).

| command | exit |
|---|---|
| `just comments` | 0 |
| `just clippy` | 0 |
| `just test` | 0 |
| `just neutral`, `just cap`, `just hygiene`, `just ts` | 0 each |
| `just fmt` | 127: `cargo fmt --all --check` passed; `topcoat` is not installed here (CI runs it) |

## Evidence (fix round, at c0c147e4b17940245399cc702dc024d928f030f5)

| command | exit |
|---|---|
| `just comments` recipe body (`scripts/comments.py; ci/comments-test.sh;` the `--todos` open-issue loop), run with `sh -ec` (no `just` on this host) | 0 |
| `python3 scripts/comments.py --todos` | 0, prints nothing (no TODO comments in the tree; the old grep matched #147 in a Swift string) |

## Evidence (first round, at 9846a56912021f16325bf622a4138cc7c70af2f2; the fix round changes only `scripts/comments.py`, `ci/comments-test.sh` and the justfile recipe)

| command | exit |
|---|---|
| `python3 scripts/comments.py` | 0 (`no comments break the rule`) |
| `ci/comments-test.sh` | 0 |
| `cargo fmt --all --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` | 0 |
| `cargo test --workspace --locked --no-fail-fast` | 101 at this head: 247 passed, 1 failed, `reliability::a_background_start_answers_before_the_agent_runs_and_sends_once` (its "connecting" state check is timing-bound; that file changes only by comment lines). Rerun alone 4×: exit 0 each. The earlier `plugins` doc-copy failure is fixed by docs/plugins.md |
| `cargo test -p cooee --test chat_first` | 0, four runs in a row (one earlier full run hit a timing flake in its unread-marker check; the file changes only by comment lines) |
| `scripts/line-cap.sh`, `ci/line-cap-test.sh`, `scripts/hygiene.sh`, `scripts/neutral-core.sh`, `ci/neutral-core-test.sh`, `scripts/dep-direction.sh` | 0 each |
| `cd web && npm run check` (tsc strict + Biome) | 0 |
| `swift build --package-path tools/anti-slop-swift` | 0 |

Only comments differ: every changed source file, with all comments and whitespace removed by the checker's own lexer, is identical at HEAD~ and HEAD. Ran in a scratch script: 151 files, 0 differ, exit 0.

Not verified locally: `topcoat fmt --check` (not installed here, CI runs it), `swift test` of anti-slop (XCTest needs Xcode; the apple job runs it), and the Playwright `ui` job (only comment lines in web/e2e and web/ts).

## Why (round 2)

Decided: just comments takes the TODO(#N) numbers from the checker's own lexer (comments.py --todos), not a raw grep over the source
Because: ci/comments-test.sh:26
Rejected: rewording the anti-slop Swift fixture so the grep misses it — the grep would still count any comment-shaped text in a string
Decided: --todos reuses the same Lexer the rule check uses, so a string literal's "// TODO(#147)" is never a comment
Because: scripts/comments.py:218
Check: ci/comments-test.sh:26 fails unless --todos prints only 201 for a file with a TODO in a Swift multi-line string and one real TODO comment
Check: the msrv job's comments step is green at the head
Owner: the comment check no longer trips over a TODO written inside a test's text, so CI's comment step passes

<sub>note on c0c147e · git log --notes=why</sub>

## Why (round 3)

Decided: merged origin/staging (#217); the lib.rs conflict keeps staging's code, and history.rs's 6 rule-breaking comment lines are stripped
Because: "Strip every comment that breaks it: plain comments, and doc comments on non-public items"
Rejected: committing the superseded lane's wip snapshot as is — it carried gate logs and pr-body.md into git
Decided: no code change for the ui job's chat.spec.mjs long-task check
Because: "Behaviour is unchanged: tests pass, only comments differ"
Check: git diff -w origin/staging HEAD on both history.rs files is only deleted comment lines
Check: build (generic/platform=macOS) and ui are green at the head; ui's long-task check also fails intermittently on staging
Owner: the branch is up to date with staging again, and the new history tool's code follows the comment rule

<sub>note on 3495a33 · git log --notes=why</sub>

## Why (round 4)

Decided: merged origin/staging; the lib.rs, mcp.rs and personas.rs conflicts keep staging's code with every rule-breaking comment line stripped
Because: "Strip every comment that breaks it: plain comments, and doc comments on non-public items"
Rejected: keeping the branch side of the conflicts — it would drop staging's new code
Decided: no code change for the review's pending iOS Simulator build check; it was queued on macOS runners, not failing
Because: "Behaviour is unchanged: tests pass, only comments differ"
Check: git diff -w origin/staging HEAD on the three conflicted files is only deleted comment lines
Check: CI's macOS jobs and ui are green at the head
Owner: the branch is up to date with staging again, and the newly merged server code follows the comment rule

<sub>note on 43df8e8 · git log --notes=why</sub>

## Why (round 5)

Decided: merged origin/staging (#149); the html.rs conflict keeps staging's side, whose ago is now pub so its /// stays
Because: "Strip every comment that breaks it: plain comments, and doc comments on non-public items"
Rejected: keeping the branch side of the conflict — it would drop staging's pub and must_use on ago
Decided: no code change for the pending iOS Simulator build; it was queued on macOS runners, not failing
Because: "Behaviour is unchanged: tests pass, only comments differ"
Check: git diff -w origin/staging HEAD on html.rs, hub.rs and the remote, personas and support tests is only deleted comment lines
Check: the iOS Simulator and macOS build checks are green at the head
Owner: the branch is up to date with staging again, and the new remote-machine code follows the comment rule

<sub>note on 968f0a2 · git log --notes=why</sub>

## Why (round 6)

Decided: merged origin/staging (#271); the new persona-text mockup scripts' 21 comment lines are stripped, nothing else changes
Because: "Strip every comment that breaks it: plain comments, and doc comments on non-public items"
Rejected: excluding docs/design from the check — the rule covers "every tracked source file", and earlier mockup scripts were stripped the same way
Decided: no code change for the review's pending iOS Simulator build; it was queued on macOS runners, not failing
Because: "Behaviour is unchanged: tests pass, only comments differ"
Check: git diff -w 968f0a2's merge of staging to HEAD on pt.js and shoot.mjs is only deleted comment lines, and scripts/comments.py exits 0
Owner: the branch is up to date with staging again, and the new persona-text mockup scripts follow the comment rule

<sub>note on 26b0fac · git log --notes=why</sub>

## Why (round 7)

Decided: merged origin/staging (f5bf384f); the devices.rs conflict keeps staging's code with its rule-breaking comment lines dropped
Because: "Strip every comment that breaks it: plain comments, and doc comments on non-public items"
Rejected: keeping the branch side of the conflict — it would drop #294's new cache test
Decided: the /// on the private Stamp struct is stripped; its meaning is in Stamp::of
Because: crates/cooee-core/src/devices.rs:167
Check: git diff -w origin/staging HEAD on devices.rs is only deleted comment lines, and scripts/comments.py exits 0
Owner: the branch is up to date with staging again, and the new device-cache code follows the comment rule

<sub>note on 4f4e0d9 · git log --notes=why</sub>

## Why (round 8)

Decided: merged origin/staging (#226); the personas.rs, manage.rs and check.rs conflicts keep staging's code with every rule-breaking comment line dropped
Because: "Strip every comment that breaks it: plain comments, and doc comments on non-public items"
Rejected: keeping the branch side of the conflicts — it would drop #226's attach-backend code
Decided: no code change for the 5 tests/setup.rs failures in the first local run
Because: crates/cooee/src/setup.rs:197
Check: git diff -w origin/staging HEAD on manage.rs, check.rs, personas.rs and tests/plugin_manager.rs and plugins.rs is only deleted comment lines, and scripts/comments.py exits 0
Check: setup tests fail only when macOS hands out a bound port above 64535; an origin/staging checkout failed them the same way, and the rerun passed
Owner: the branch is up to date with staging again, and the new plugin-manager code follows the comment rule

<sub>note on d0b1c75 · git log --notes=why</sub>

## Why (round 9)

Decided: merged origin/staging (#150, #208); the lib.rs, login.rs, main.rs, rooms.rs and support/mod.rs conflicts keep staging's code with every rule-breaking comment line dropped
Because: "Strip every comment that breaks it: plain comments, and doc comments on non-public items"
Rejected: keeping the branch side of the conflicts — it would drop #150's machine check and #208's room code
Decided: staging's new comments in machine.rs, tests/login.rs and tests/machine_check.rs are stripped, and the blank first lines they left are dropped
Because: crates/cooee/src/machine.rs:1
Check: git diff -w origin/staging HEAD on the conflicted files adds no lines, only deletes comment lines, and scripts/comments.py exits 0
Check: the macOS and iOS Simulator jobs run and pass at the head
Owner: the branch is up to date with staging again, and the new machine-check and room code follows the comment rule

<sub>note on 957f2ca · git log --notes=why</sub>

## Why (round 10)

Decided: no code change for the review's CI FAIL at 957f2ca; the iOS Simulator build was queued on macOS runners and the ubuntu jobs got a runner shutdown signal mid-test
Because: "Behaviour is unchanged: tests pass, only comments differ"
Rejected: retrying CI on the old head alone — staging had moved, so the merge reruns every job anyway
Decided: merged origin/staging (#255); a clean merge of 5 Swift files kept exactly as staging has them
Because: apple/Sources/CooeeUI/Sheets.swift:1
Check: git diff origin/staging HEAD on the 5 merged Swift files is empty, and just comments exits 0
Check: the macOS, iOS Simulator and ubuntu jobs run and pass at the head
Owner: the branch is up to date with staging again; the earlier red CI was runners going away, not the change

<sub>note on 006e68a · git log --notes=why</sub>

## Why (round 11)

Decided: merged origin/staging (6d75396f); the manage.rs conflict keeps staging's health-check supervisor code with its 44 rule-breaking comment lines dropped
Because: "Strip every comment that breaks it: plain comments, and doc comments on non-public items"
Rejected: keeping the branch side of the conflict — it would drop staging's hung-backend health checks and their tests
Check: git diff -w origin/staging HEAD on manage.rs adds no lines, only deletes comment lines, and scripts/comments.py exits 0
Owner: the branch is up to date with staging again, and the new plugin health-check code follows the comment rule

<sub>note on a5127c7 · git log --notes=why</sub>

