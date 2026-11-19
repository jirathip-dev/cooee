Closes #601

## For humans
The server now gzips text, JSON and SVG replies over 1 KB when the client asks for it, so the big sprite sheets download about 15x smaller.
Look art (theme SVGs) now carries a version tag, so a client that already has it gets a quick "not modified" reply instead of the whole file again.

## What changed
- `Gzip` layer (crates/cooee-server/src/gate.rs): when `Accept-Encoding` lists gzip (q>0), buffers text/*, application/json and image/svg+xml replies, gzips those over 1 KB, sets `Content-Encoding: gzip` and `Vary: accept-encoding`, drops `Content-Length` and weakens a strong ETag. SSE (`text/event-stream`) is passed through untouched: buffering a live stream would never end.
- `/look/<pack>/sprites.svg` art (themes.rs): `ETag` (sha256 of the SVG), `Cache-Control: no-cache`, and a 304 on a matching `If-None-Match` (strong or weak).
- flate2 =1.1.10 added, gzip only (q2045).
- Test: `crates/cooee/tests/themes.rs` drives the real server: gzip header, size under a quarter of the plain SVG, round-trip decode, weak ETag, and 304 for both tags.

src/ net +70 (budget n/a): one layer plus ETag/304 on look art.

## Evidence (head 1700568428537f425a152abdd8dfa70a48a23ef9)
- `cargo fmt --all --check` → 0
- `bash ci/gates/clippy.sh` → 0
- `bash ci/gates/test.sh` → 0
- `cargo test -p cooee --test themes` → 0 (10 passed)
- `bash scripts/line-cap.sh` → 0 (29709 / 32000)
- `ci/gates/comments.sh`, `bans.sh`, `deps.sh` → 0
- Not run locally: `ci/gates/fmt.sh` (needs the `topcoat` CLI) and `deny.sh` (needs `cargo-deny`), since neither is on this host; CI runs both.
