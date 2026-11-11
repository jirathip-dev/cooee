Closes #167

## For humans

On a Mac, cooee's secrets (persona secrets, API keys) now live in the keychain, readable only by cooee signed with your Apple Development certificate; a rebuilt cooee still reads them with no prompt, other programs and agent shells can't.
`cooee secrets` moves any old secret files into the keychain, checks each one, then shreds the files. The installer signs cooee; it stops with a clear message if no signing certificate is installed.
Linux keeps the private 0600 file. A keychain that doesn't answer within 5 seconds is a visible error, never a hang.

## What changed

- `crates/cooee-core/src/secrets.rs` (new): one store for `secret:<NAME>` values. macOS: in-process keychain item (security-framework, q1169), service `cooee-secret`, account keyed by the data folder; prompts disabled; every call time-limited (`limited`, 5 s → `TimedOut` error). Saving refuses an unsigned cooee (code requirement: Apple anchor + identifier `cooee` + leaf OU = team, q1261). Linux: `<data>/secrets/<NAME>` 0600. `migrate` moves each file, reads it back, then shreds it.
- `profiles.rs`, `mcp.rs`: read secrets through that module; the old `security` subprocess path in mcp.rs is deleted.
- `cooee secrets` command (`keychain.rs`, `main.rs`): migrate, then read each secret back; prints names only. `serve` runs the migration in the background at start.
- `scripts/install.sh`: on macOS signs the installed binary with the "Apple Development" identity, `--identifier cooee`, each call time-limited.
- Tests: `crates/cooee/tests/secrets.rs` (signed b1 migrates + shreds; rebuilt b2 with a different cdhash reads with no prompt; unsigned build is refused; `security` from a shell gets nothing), unit tests for the timeout, Linux file path and unsigned refusal.

src/ net +241 (budget not stated): new secrets module replaces the old file reads and the `security` subprocess helper.

## Evidence (head 12673eed080b15490548985438b8888ef2268441)

| command | exit |
|---|---|
| `cargo fmt --all --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` | 0 |
| `scripts/line-cap.sh` | 0 |
| `scripts/neutral-core.sh` | 0 |
| `scripts/hygiene.sh` | 0 |
| `scripts/dep-direction.sh` | 0 |
| `cargo deny --locked check` | not run: cargo-deny not installed on the Mini (CI runs it) |

Fix round 1: the Linux clippy job failed because `prefix` (macOS-only caller) was dead code on Linux and a Linux-only test used `assert!(...is_empty())`. `prefix` is now `#[cfg(target_os = "macos")]` and the test uses `assert_eq!`. Clippy for a Linux target can't run on the Mini (no Linux std installed), so CI's `msrv (ubuntu-latest)` job is what proves it. Merged `origin/staging` (#139 persona `pinned`). In one full run `chat_first::an_agent_has_one_ongoing_chat...` failed on timing (it caught a "working" state); it then passed 3 of 3 runs on its own and passed in the full rerun above. It doesn't touch secrets.

**Not verified yet:** `only_the_signed_cooee_reads_its_secrets_across_rebuilds` reported `ok` but **skipped** (it prints `skipped: no Apple Development signing identity on this Mac`): `security find-identity -v -p codesigning` shows 0 identities on the Mini. Per q1261 the owner adds the Apple Development cert to the Mini; the test must then be run there for the signed/rebuild/unsigned/agent-shell acceptance. CI runners have no cert either, so it skips there too.

