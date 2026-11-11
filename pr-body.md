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

Decided: merged origin/staging; conflicts keep staging's code with comments dropped, and staging's new rule-breaking comments are stripped
Because: "Strip every comment that breaks it: plain comments, and doc comments on non-public items"
Rejected: none
Check: just comments is green at the head and the merge's diff -w against staging is only deleted comment lines
Owner: the branch is up to date with staging and the new secrets and fleets-config code follows the same comment rule

<sub>note on 3312615 · git log --notes=why</sub>

