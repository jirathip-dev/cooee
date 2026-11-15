Closes #309

Part of #300 (step 2 of `docs/architecture/layout-blocks/migration.md`).

## What
- `crates/cooee-server/src/layout.rs`: `layout.json` beside `config.toml`, read every second (fingerprint: its text, so a same-size write within one mtime tick is not missed). Good/Broken: only a layout that passes the app's `Layout` rules (ported: format, root nav, known types, slots, config keys, places, unique ids, depth 6, 200 nodes, 64 KiB, locked screens dropped) is served; with no file the app's bundled default (`apple/Sources/CooeeUI/Default/layout.json`, included, one source of truth).
- `GET /layout` → `{etag, layout}` + `ETag`; 304 on a matching `If-None-Match`.
- `GET /layout/events` SSE: `changed {etag}`, `broken {line, col, why}` (also sent at once to a new subscriber while the file is broken).
- `POST /layout/reset`: copies the file to `layout.json.bak`, writes the default, answers the new `{etag, layout}`.
- `cooee layout check <file>...` (shares `plugin check`'s runner); `docs/api.md` § Layout.

Decided: a semantic refusal's line is where its offending text first appears (serde_json keeps no positions); a parse error's line/col is serde's.

## Evidence (head 178f882bb14edebe1a9fd5a67be21a7b4e7a89ee, staging 06f65751 merged in)
- `cargo test -p cooee --test layout --test plugins --test cli` → exit 0 (layout: 1 passed — bad write keeps last good + `broken` with the line, 304 on etag, reset writes default + .bak, every app fixture refused/accepted as the app does)
- `cargo test -p cooee-server` → exit 0 (38 passed)
- `cargo fmt --all --check` → exit 0; `cargo clippy --workspace --all-targets --locked -- -D warnings` → exit 0; `cargo doc` with `-D warnings` → exit 0; `scripts/neutral-core.sh` → exit 0
- `sh scripts/line-cap.sh` → exit 0: 19372 / 27500

src/ net +281 (budget: none stated) — the Rust port of the app's layout rules is about half of it. Fits the cap since #189 merged (cap ask Q1687 is moot).

Merged origin/staging (#189): conflicts in lib.rs (router args/routes) and docs/api.md (theme section removed) resolved by keeping staging's side plus this PR's layout lines.

apple: net 0.

## For humans
- The Mac now serves the owner's layout file to the apps, and only ever a version that passes the checks.
- A broken edit leaves the last good layout in place and tells the apps which line is wrong.
- "Reset layout" puts the default back and keeps the old file as a backup; `cooee layout check` checks a file before you save it.
