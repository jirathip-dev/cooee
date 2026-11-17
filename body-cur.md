Closes #163

## For humans
An agent can now ask you for a password or API key it needs at a terminal prompt. You enter it on a card behind your passkey, and cooee types it in once without keeping it. If the agent asks to reuse it, it is saved for that persona only (keychain on a Mac, a private file on Linux) and typed again later without asking. The agent itself only ever reads "secret NAME entered".

## What changed
- New cooee MCP tool `secret {name, terminal, why, save}`. A secret the persona already saved is typed into the terminal at once. Otherwise a `secret` card is raised; answering it needs a passkey assertion, the same Face ID gate MCP proposals use.
- On answer, the value is typed into the shed terminal (`type_line`), then enter. If `save` was asked, it is first stored as `<persona>.<NAME>` through `secrets::set` (keychain, signed cooee only, time-limited; 0600 file on Linux). The agent's queue then gets `secret NAME entered`.
- Saved secrets reach the persona's environment: `Agent::env` adds the persona's saved secrets to its secret variables. That covers terminal sessions.
- Only a plain program terminal can receive a secret. An agent's ACP session and the owner's shells (`cooee.owner`) are refused.
- A secret card's block now sends its first option as the choice. Plugin cards still send `save`.

## Decided
- No browser or page field exists in the repo yet. The "field" is a shed terminal the agent names, such as a password prompt. The page-field target is left for the screen/browser work.
- The agent asks for once or save. The card says which, and the owner can dismiss it.

## Evidence (at 7c81849c8a1f99704af76cf6cae5c45d08b85f7c)
- Merged origin/staging (#219 and others). `hub/cards.rs` conflict: the proposal forms keep both `memory` (staging) and `secret` (this PR).
- CI at 474b1b2 went green once and later red in `a_saved_secret_...` ("only coder's"). Same head, so it is flaky. Every fake adapter run wrote one shared `adapter.env`, so an overlapping coder run could overwrite or mix into the writer's file. The fake adapter now also writes `adapter-<terminal>.env`, and the test reads each thread's own file. If the writer really got the value, the test still fails.
- `cargo fmt --all`: exit 0. `ci/gates/clippy.sh`: exit 0. `ci/gates/comments.sh`: exit 0. `ci/gates/bans.sh`: exit 0. `ci/gates/test.sh`: exit 0 (macOS; the saved-path test is Linux-only and runs in CI). `cargo test -p cooee --test mcp --test profiles`: exit 0. `ci/gates/fmt.sh`: exit 127, because topcoat isn't installed on this host.

Earlier, at 474b1b22311f9f9566ff801141a69173244b96b3:
- Merged origin/staging again (#518), clean. `bash ci/gates.sh`: exit 1, and only on host-tool gaps. `deny` fails because cargo-deny isn't installed. `fmt` fails because topcoat isn't installed, though `cargo fmt --all --check` alone exits 0. `doc` failed on stale build output, and `ci/gates/doc.sh` in a fresh target exits 0. clippy, test, cap, comments, bans, neutral, hygiene and themes pass.

Earlier, at 21f482cef4ba6511669a724fb9f0d2418ec8d0b6:
- Merged origin/staging (#520): `secrets.rs` conflict kept both `remove` and the persona helpers. `ci/gates.sh clippy test cap`: exit 0; `cargo fmt --all --check`: exit 0

Earlier, at 4e64f998056cb8ef8863a9e1aa1589acd81c0059:
- `cargo test -p cooee --test mcp` (macOS): exit 0, 5 passed (the saved-path test is Linux-only and runs in CI)
- CI at 4e64f99: green (10 checks; msrv and stable ubuntu, incl. the saved-path test, pass)
- Round 2: CI failed in `a_saved_secret_...` because its last check started a persona-less thread, which runs no adapter, so it read the earlier run's variables. It now starts a second persona and asserts that persona's own variable is there and the value is not.

Earlier, at 22dbe7749187bf3c35c2dc7497a06861736c2f1b:
- `ci/gates.sh clippy test cap`: exit 0
- `ci/gates.sh clippy comments doc`: exit 0
- `drover arch-check`: exit 0, 0 violations
- New tests in `crates/cooee/tests/mcp.rs`:
  - `a_secret_dropped_once_...`: drops a value once, then checks that it appears in no transcript, event, file, log or agent input.
  - `a_saved_secret_...`: the saved path, 0600 file, reuse without a card, and the variable reaching only that persona. This test runs on Linux only: on macOS an unsigned test build is refused by D4.
- `fmt` gate: `cargo fmt` is clean. The gate's `topcoat fmt` step can't run on this host because topcoat isn't installed.

src/ net +171 (no budget given): one hub module plus the tool schema and wiring.

## Why (round 3)

Decided: merge origin/staging; hub/cards.rs proposal forms keep both staging's memory and this PR's secret
Because: crates/cooee-server/src/hub/cards.rs:53
Decided: the saved-secret test reads each terminal's own adapter env, so a concurrent run can't overwrite it
Because: crates/cooee/tests/support/shed.rs:120
Rejected: polling the shared adapter.env — a later run of another persona still wins the race
Check: crates/cooee/tests/mcp.rs saved test still fails if the writer persona's own env holds the value
Owner: merged the latest staging and fixed a flaky test; the feature is unchanged

<sub>note on 7c81849 · git log --notes=why</sub>

