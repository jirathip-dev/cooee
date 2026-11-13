Closes #319

## For humans
A theme pack can no longer carry scripts, event handlers, embedded HTML or links to outside addresses in its pictures: the theme check refuses it and says which file and why.
Pictures from a theme are also served with a strict "run nothing" policy, so even a file that slipped past the check cannot run code as cooee.
Farm, kitchen, garden and the starter pack look and pass exactly as before.

## What changed
- `web/src/look.rs`: `file()` runs `active()` on `sprites.svg` and `scene.svg`; it fails `<script>`, `<foreignObject>`, any `on*` attribute, any `href`/`xlink:href` that is not a local `#id`, and an `<!ENTITY` declaration (an entity can expand into markup the scan would not see). Comments are skipped. Each reason is one gate line: `sprites.svg: <script> (active content)`.
- `web/src/lib.rs`: every `/look/<name>/<file>` response carries `Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'` and `X-Content-Type-Options: nosniff`.

## Repro
Commit 1 (aa4ed87b) adds only the test `the_gate_fails_active_content_in_a_packs_svg_and_served_svg_runs_no_script` in `crates/cooee/tests/themes.rs`; on it `cargo test -p cooee --test themes the_gate_fails_active` exits 101 (`<script>alert(1)</script>: ok   …/active0`, the gate passed it). At the head it exits 0.

## Evidence at 14117a5f459db21c4acb00c4d91b821eb1810d16
Round 4: the FAIL was `msrv (macos-latest)` timing out in `crates/cooee/tests/screens.rs` (`never: its screen` after 229 s, the 20 s wait). That test and the plugin screen path are not touched by this PR; locally it passes in 1.9 s. This head merges `origin/staging` (#245) so CI runs again; the code for this issue is unchanged.
- At 14117a5f: `cargo test -p cooee --test screens` → exit 0 (1 passed, 1.87 s)
- At 14117a5f: `cargo test -p cooee --test themes` → exit 0 (5 passed)
- At 14117a5f: `cargo fmt --all --check` → exit 0; `cargo clippy --workspace --all-targets -- -D warnings` → exit 0
Earlier evidence (82059de / d9a64b1, same issue code):
(round 3)  the FAIL was again CI cancelled by a runner shutdown signal (`dmg`, macOS and ui cancelled; ubuntu jobs killed mid-test with "The runner has received a shutdown signal"), not a test failure. This head merges `origin/staging` again (#308, #292) so CI runs again; the code for this issue is unchanged.
- At 82059de: `cargo test -p cooee --test themes` → exit 0 twice (5 passed). The first run right after the merge exited 101 (the `<script>` case reported ok) and did not reproduce on the next two runs; reported here, not hidden.
- At 82059de: `cooee theme check` on packs/themes/farm, kitchen, garden and packs/starter → exit 0 each; `cargo fmt --check` → exit 0
- `cargo test -p cooee --test themes` → exit 0 (5 passed, incl. `the_gate_fails_active_content_in_a_packs_svg_and_served_svg_runs_no_script`)
- `cargo test -p cooee-web` → exit 0
- `cargo clippy --workspace --all-targets --locked -- -D warnings` → exit 0
- `cargo fmt --all --check` → exit 0
- `cargo run --quiet --locked -p cooee -- theme check packs/themes/* packs/starter` → exit 0 (ok farm, garden, kitchen, starter)
- `scripts/line-cap.sh` → exit 0 (27081 / 27500); `scripts/hygiene.sh`, `scripts/neutral-core.sh` → exit 0
- `cargo test --workspace --locked` → exit 101 locally, only in `crates/cooee/tests/setup.rs` (5 tests: `cooee: no port + 1000 for the terminal plugin`). That test binds an OS-chosen port; on this host it is above 64535, so `port + 1000` overflows in `crates/cooee/src/setup.rs:210`. Neither file is touched by this PR, and those tests passed in CI at e4d5f64. Every other workspace test passed.
- `just` is not installed on this host, so its recipes were run by hand as above.

src/ net +68 (no budget given): a small tag/attribute scanner (stdlib only, no XML dep) plus two headers.

## Why (round 2)

Decided: round 2 changes no code; the FAIL was CI's ubuntu jobs cancelled by a runner shutdown signal, so this head merges origin/staging to re-run CI
Decided: the scan runs inside file(), so sprites.svg and scene.svg are checked wherever the gate reads them
Because: web/src/look.rs:337
Decided: a stdlib tag and attribute scanner, no XML crate, skipping comments
Because: "no external or `javascript:`/`data:` references"
Rejected: adding an XML parser dependency — a new dependency needs the owner and the scanner is enough with the CSP behind it
Decided: the CSP and nosniff headers go on every look file response, not only the two SVGs
Because: web/src/lib.rs:147
Check: cargo test -p cooee --test themes passes at the head; theme check passes farm, kitchen, garden and packs/starter
Check: local setup.rs test failures are a host port above 64535 overflowing port + 1000 in crates/cooee/src/setup.rs:210, a file this PR does not touch
Owner: A theme pack can no longer carry scripts or outside links in its pictures, and its pictures are served so they cannot run code.

<sub>note on d9a64b1 · git log --notes=why</sub>

## Why (round 3)

Decided: round 3 changes no code; the FAIL was CI cancelled by a runner shutdown signal again, so this head merges origin/staging to re-run CI
Decided: the scan runs inside file(), so sprites.svg and scene.svg are checked wherever the gate reads them
Because: web/src/look.rs:337
Decided: a stdlib tag and attribute scanner, no XML crate, skipping comments
Because: "no external or `javascript:`/`data:` references"
Rejected: adding an XML parser dependency — a new dependency needs the owner and the scanner is enough with the CSP behind it
Check: cargo test -p cooee --test themes passes at the head; theme check passes farm, kitchen, garden and packs/starter
Check: one themes run right after the merge failed the script case and did not recur in two reruns; watch CI for it
Owner: A theme pack can no longer carry scripts or outside links in its pictures, and its pictures are served so they cannot run code.

<sub>note on 82059de · git log --notes=why</sub>

