Closes #252

## For humans
The iPhone and Mac app's Look sheet now has a Theme list: the theme packs installed on your paired server (farm, kitchen, garden, and any you add). Picking one recolours Home, chats and Terminal at once, in light and dark, and Scene draws that pack's scene and figures. Farm stays the default, and a pack you remove on the server drops out of the list.

## What changed
- Server: `GET /api/v1/looks` answers the installed themes and schemes as JSON, read from the config's `themes/` on each call, so a pack added or removed shows at once. `GET /look/{theme}/sprites.svg|scene.svg` serves a pack's art with no cookie, for the app's Scene. Both live in `crates/cooee-server/src/themes.rs` (staging removed the web client in #189; `lib.rs` only registers them and opens `/look/`, staying at its #356 arch baseline).
- Native: Settings keeps a `theme` (default farm; old saved settings read as farm). The menu loads the looks when Home starts and each time the menu opens. Root builds the palette from the picked pack's scheme tokens for the current mode, else farm. A pack other than farm draws Scene's strip and the sprites from its `scene.svg` / `sprites.svg` on the server (a WebKit view, no new dependency); farm keeps the app's own coat sprites.
- e2e: `serve.sh` installs the three first-party packs; `LiveUITests` picks each one in Modern and Scene and keeps screenshots.

## Evidence (head: 1e537a6dcccd0536568225b1fb5597c0582518a6)
- Fix round 8: CI `apple` failed at 35a2e9f on SwiftLint `type_body_length` (ClientTests 162 > 150 after staging grew it). The looks test moved unchanged to `apple/Tests/CooeeAPITests/LooksClientTests.swift`. `scripts/apple-check.sh` (the `just apple-check` recipe) at 1e537a6 → exit 0 ("apple-check: ok").
- Items below are from the earlier head 7a19210; 1e537a6 changes only those two test files.
- Fix round: `apple/e2e/map.toml` now names `LiveThemeUITests` (its test file, `packs/swag.toml` and `crates/cooee/tests/themes.rs`; Theme.swift, Root.swift, CooeeCore and the server crates already run the whole suite). `EVENT=push scripts/e2e-scope.sh` → exit 0; `ci/ci-scripts-test.sh` → exit 0 ("ci scripts test ok").
- Merged origin/staging again (#201, #318, #213, #314, #423, #465): `lib.rs` keeps this PR's themes routes and `/look/` opening, without the comments #201 strips; `themes.rs` and `tests/themes.rs` lose their non-public comments. #318 vendors the packs into git-ignored `packs/themes/` at build, which `serve.sh` builds before it copies them.
- `scripts/comments.py` → exit 0. `drover arch-check` → exit 0 (0 violations). `cargo fmt --all --check` → exit 0. `cargo clippy --workspace --all-targets --locked -- -D warnings` → exit 0.
- `cargo test -p cooee --test themes --locked` → exit 0 (4 passed). `cargo test --workspace --locked --no-fail-fast` → exit 101, from one test: `workflow.rs` `a_lane_opens_as_a_chat…` ("no 1 workflow sessions"), a timing failure under the full run. Run alone, `cargo test -p cooee --test workflow --locked` → exit 0 (3 passed). This PR doesn't touch workflow code.
- `cd apple && swift test` → exit 0 (47 tests). `swift format lint --strict -r Sources Tests UITests` → exit 0.
- `scripts/line-cap.sh` on a clean `git archive` of the head → exit 0 (apple 13295 / 14316). In the worktree it counts `apple/.build`.
- Screenshots (AC1), from the earlier head 8a9dcb34 (green e2e, run https://github.com/jirathip-dev/cooee/actions/runs/43421558743): artifact [`simulator-shots`](https://github.com/jirathip-dev/cooee/actions/runs/43421558743/artifacts/14040086204), 18 frames `theme-{kitchen,garden,farm}-{modern,chat,scene}-{dark,light}`. With the map fixed, the e2e runs again on merge-ready CI.
- Not shown: Mac screenshots (CI takes iPhone simulator shots only). Not run locally: the simulator e2e.

src/ net +156 (budget not stated): server routes, theme model, palette from tokens and pack art view.

## Why (round 7)

Decided: the looks client test moved unchanged into LooksClientTests, so ClientTests stays under SwiftLint's 150-line type body
Because: apple/Tests/CooeeAPITests/LooksClientTests.swift:5
Rejected: trimming other ClientTests cases — they are staging's tests, not this PR's
Check: scripts/apple-check.sh passes at head (CI apple job)
Owner: A test file got too long for the lint rule; one test moved to its own file, nothing in the app changed.

<sub>note on 1e537a6 · git log --notes=why</sub>

