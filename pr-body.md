Closes #319

## For humans
A theme pack can no longer carry scripts, event handlers, embedded HTML or links to outside addresses in its pictures: the theme check refuses it and says which file and why.
Hiding a script inside a comment, a CDATA block or a processing instruction no longer gets it past the check.
Farm, kitchen, garden and the starter pack pass exactly as before.

## What changed
- `crates/cooee-server/src/look.rs` (moved from `web/src/look.rs` by #189): `file()` runs `active()` on `sprites.svg` and `scene.svg`; it fails `<script>`, `<foreignObject>`, any `on*` attribute, any `href`/`xlink:href` that is not a local `#id`, and an `<!ENTITY` declaration. Each reason is one gate line naming the file, e.g. `sprites.svg: <script> (active content)`.
- Round 6 fix: the scan used to strip `<!-- … -->` before anything else, so `<![CDATA[ <!-- ]]><script>…</script><![CDATA[ --> ]]>` hid a live `<script>`. It now walks the file left to right and skips whichever of a comment, a CDATA section or a processing instruction (`<? … ?>`, the same hole) opens first, to its own close.
- Served-file headers dropped: staging's #189 deleted `web/src/lib.rs` and with it the `/look/<name>/*.svg` route (and the test helpers that fetched it). Nothing serves a pack's SVG any more, so there is no response to put the CSP/nosniff headers on; the merge takes staging's deletion and removes that half of the test. If a route comes back, it needs those headers.

## Repro
Commit ab2d05b6 adds only two cases to `the_gate_fails_active_content…` in `crates/cooee/tests/themes.rs` (CDATA-wrapped and PI-wrapped `<!--` around a `<script>`); on it `cargo test -p cooee --test themes the_gate_fails_active` exits 101 (`<![CDATA[ <!-- ]]><script>alert(1)</script>…: ok   …/active5`). At the head it exits 0.

## Evidence at c0a1734151490f191c90d283078c5c538a5347c0
- `cargo test -p cooee --test themes` → exit 0 (4 passed)
- `cargo test -p cooee-server look` → exit 0 (7 passed)
- `cooee theme check` on packs/themes/farm, kitchen, garden and packs/starter → exit 0 each (ok ×4)
- `cargo fmt --all --check` → exit 0; `cargo clippy --workspace --all-targets --locked -- -D warnings` → exit 0
- `scripts/line-cap.sh` → exit 0; `scripts/neutral-core.sh` → exit 0

src/ net +70 (no budget given): a stdlib tag/attribute scanner, no XML dependency.
