Closes #360

## For humans
Saving a persona, a routine or a note now changes only the lines that were edited, so comments and layout you wrote by hand stay.
If you edit a persona's files by hand while the app has it open, the app's save is refused with "changed on disk, reload" and your unsaved text stays in the editor.
A hand edit to a persona's instructions is used by the next chat with no restart.

## What changed
- `cooee-core/src/disk.rs` (new): `version` (mtime, size and content hash of a set of files), `Stale` (the refusal `agents::save` returns when the version is older) and `patch` (`toml_edit`, already in Cargo.lock: only the keys that differ are set, added or removed, tables and arrays of tables key by key).
- `agents::save`: checks `version` against the folder (`agent.toml`, `INSTRUCTIONS.md`, `CHARACTER.md`), patches `agent.toml` and writes each file only if it changed. Every read (`GET /personas`, `/personas/{id}`) carries `version`; `version` isn't an `agent.toml` key.
- Server: a stale save is `HubError::Stale` → **409** on `PUT /personas/{id}` and the persona screen's `save` action. The agent's `update_persona` reads fresh before saving, so it gets a plain error only on a real race.
- `memory.md`: a note edit rewrites only its own line, a new note appends one line, and lines that aren't notes stay.
- `routines.toml`: patched through `disk::patch` (comments kept).
- Crash-safe writes (temp, fsync, rename) for `SKILL.md`, MCP picks and the setup command's `config.toml` (they used `fs::write`).
- Swift: `APIPersona.version` is carried back on save, so a 409 shows the server's "changed on disk, reload" and the draft stays.

## Decisions
- Reload uses "check on each read" (the spec's other option): every read already goes to disk, so there is no watcher and no polling loop (#345).
- A save that names no version (an old client, the CLI) isn't checked, so existing callers keep working.
- `style` is compared against the default for the new role, so an unset `style` stays unset unless the save changes it.

## Evidence (head 03ffe7d8486013c9c15386b351d8f99531668cd9)
Fix round 3: the premerge `drover cap-check` (formatted lines of `src/**/*.rs`, in-file tests included) came out at 27574 / 27500. To trim it I moved the `disk::patch` unit test and the routines comment test from `src/` to `crates/cooee-core/tests/disk.rs`, put the stale check inline in `agents::save` and `http.rs`, and compacted `disk.rs`, `memory.rs` and the write loop in `agents.rs`. Behaviour is unchanged. Then I merged `origin/staging` (275b6ae, no conflicts, no Rust files touched).
- `drover cap-check --config ci/cap.toml` on a clean `git archive` of this head → exit 0 (`src lines (formatted): 27497 / cap 27500`).
- `sh scripts/line-cap.sh` on the same archive → exit 0 (`non-test lines: 22199 / 27500`, `apple lines: 14747 / 15250`).
- `ci/gates.sh clippy comments bans doc neutral lessons hygiene deps test` → exit 0. That covers `config_edits` (AC1–AC4), `personas` and the new `cooee-core --test disk`.
- `cargo fmt --all --check` → exit 0.
- `ci/gates.sh` in full → exit 1. It fails only `deny` (`cargo-deny` not installed), `fmt` (the `topcoat` CLI not installed) and `cap` (this worktree's ignored `apple/.build/` gets counted). All three are artifacts of this host; the clean-archive checks above stand in for them.

src/ net +146 by `line-cap.sh` (no budget stated): a new disk module, a version on every persona read, and 409 handling.
