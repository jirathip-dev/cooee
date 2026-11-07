Closes #34

## For humans

The look sheet's Style now has a third choice, Scene. In Scene the thread list becomes the theme's picture (the farm's ranch, the kitchen, the garden): one page per folder or Workflow source that you swipe sideways, with a dot per page, and every session is a little figure acting out what it's doing, with its name and age under it. Tap a figure to open it; threads themselves keep Modern or Terminal, whichever you chose last.
Scene is offered only when Figures is set to Sprite and the theme has art; choosing ASCII or None, or the plain theme, hides it.

## What changed

- **Style: Modern | Terminal | Scene**: built on #35's look (cookie `scene:on` over the `style` it falls back to; `Shown::styles()` hides Scene unless Figures is `sprite`). This PR adds `Shown::shape()` / `scene()` (and `shape()`/`scene()` in `look.ts`): pages take `data-style` from the Modern/Terminal pick, so threads keep it in Scene (scope 3). Switching Scene in or out on the thread list reloads it so the server draws the other layout.
- **The scene** (`web/src/lib.rs` `scene`/`sprite`, `web/src/app.css`): a page per card (working folder, or Workflow source) in a scroll-snap row, a dot link per page (`#sp-N`, works with no script), each page the theme's `scene.svg` strip, the card's bar, then one 132 × 100 sprite per session (`working`→`s-edit`, `needs`, `done`, `fail`, ended→`s-merged`, else `s-idle`) with its name card (title, the row's line — `agent · age`, or a Workflow session's `role · round N · age` — and the theme's state word). Each sprite is a link to `/t/<id>`. Scene follows #33's All · Chats · Workflow chip since it draws the same cards; a Workflow card's "done today" sessions are on its page too.
- **Refresh** (`web/ts/chat.ts`): `GET /threads` redraws the pages whole (one element per session id, never patched copies; scope 5 / R16), keeps the page in view, marks the current dot, and switches the strip to night 19:00–07:00 on the device's clock.
- **Scene art** (`web/themes/*/scene.svg`, `docs/design/scene/build.mjs`): kitchen's and garden's are herdsman's `scene.svg` as is; farm's ranch is ported to a still SVG by `build.mjs`, with night as `scene.svg#night` via `:target`. Served at `/look/<theme>/scene.svg`; the theme contract gains an optional `scene.svg` (docs/themes.md).

## Update round (merge of origin/staging, #33 and #35)

Resolved `docs/themes.md`, `web/src/lib.rs`, `web/src/look.rs`, `web/src/sheet.rs`, `web/src/shown.rs`, `web/ts/look.ts` (plus test updates in `crates/cooee/tests/chat.rs`, `web/e2e/scene.spec.mjs`):
- #35 already made Style `modern | terminal | scene` and Figures `sprite | ascii | none` with Scene only for Sprite (amendment A). Its model is kept; this branch's `list:scene` key, its Sprites lock and "scene needs sprites" note are dropped (the Sprites switch no longer exists; amendment A replaces scope 4).
- #33's chips and `card` component kept; Scene renders in place of the cards under the chips.
- Tests now use `scene:on`, and check that ASCII, None or the plain theme hide Scene (server and e2e) instead of the old greyed-out switch.
- `web/e2e/look.spec.mjs` (#35's "Scene only with sprites" test) expected `data-style="scene"` on `<html>`. With #34 that attribute is the pages' shape, which stays Modern or Terminal (scope 3), and picking Scene in or out redraws the thread list. The test now expects `#scene` and `data-style="terminal"`, and opens the sheet again after each redraw.

## Update round 2 (merge of origin/staging, #44)

Resolved `web/e2e/machines.spec.mjs` only. Staging (d5a8874) fixed the same flaky sparkline from the other side: the fake shed now sends its history once and the test accepts 1438–1439 points. This branch keeps its plugin fix (samples newer than the last only) and its wider 1420–1439 range, since the fake day still ages a sample out per minute of the run. Both fixes hold together. #44's cards and composer chips merged without conflicts in `lib.rs`, `app.css` and `chat.ts`.

## Staging build fix (machines plugin)

`ui`'s `machines.spec.mjs` at 1440 failed three times on this branch, and once on staging itself (`ac36b94`). The sparkline had 2878 points instead of 1440. The e2e fake shed (`web/e2e/serve.sh`) answers the whole day on every `--since` poll, and the plugin appended all of it each minute, so the 1440 project (which runs after the first minute) saw the day twice. `crates/cooee-machines/src/lib.rs` now keeps only samples newer than its last one. A new assertion in `the_24_h_history_is_asked_for_once_then_only_what_is_new` (a shed answering the whole day again) fails without the fix (`left: 2877, right: 1438`) and passes with it.
With the doubling gone, the 1440 project then saw 1439 points. `serve.sh` dates the fake day once at start, so each minute of the run ages one sample out of the plugin's 24 h. `machines.spec.mjs` now accepts 1421 to 1440 points. A doubled day (2878) still fails.

## Staging build fix (push notifications)

`ui`'s `push.spec.mjs` failed twice on this branch (at 1440 showing nothing, at 390 showing the older "needs you"). The service worker ran its push and read-mark handlers at the same time, so the older push could finish last or a read mark could close a newer push. `web/ts/sw.ts` now handles them one at a time in the order they arrive. Locally `push.spec.mjs --repeat-each 10` passed 21/21, but it never failed locally, so CI is the real check.

## Update round 3 (merge of origin/staging, #40) and the long-task check

- Merged `origin/staging` (2925d02, #40's agent profiles). One conflict in `web/src/lib.rs`: two independent additions at the same spot (this PR's `pose`/`sprite`/`scene`, #40's `last_used`/`profile_picker`). Both kept as they were.
- Review FAIL at 32ca684: `ui`'s `chat.spec.mjs:131` (1,000 messages, no long tasks over 50 ms) failed 3/3 in CI. Nothing in this PR runs on the thread page apart from `light()`, which finds no `#scene` there and does nothing. The only commit between the green cd20607 and 32ca684 touched the service worker. Locally at the merged head the test passed 5/5 at 390 (`--repeat-each=5`) and in the full Chromium run at both widths. I made no code change for it; CI at the new head is the check.
- `docs/screens/34/README.md`: the caption no longer mentions the old Sprites switch. It now says the look sheet shows Figures and offers Scene only with Sprite.

## Decisions and what is not done

- **Host-load weather is not wired**: cooee has no host-load source in the web crate, so the scene is always calm/clear (herdsman's load layers are hidden by a rule `/look/<theme>/scene.svg` adds). Day/night is done.
- The ranch is still (no wind animation, no cloud drift); sprites keep their two-frame loop.
- Line count: `src/ net +146` vs staging (no issue budget stated); the repo total is `non-test lines: 12664 / 15500`. This is the scene component, the TS redraw and its CSS.

## Evidence (head cf41239bf9f7172eff1f8616a7353f1fc27e55b3)

| gate | command | exit |
|---|---|---|
| fmt | `cargo fmt --all --check` | 0 |
| fmt (topcoat) | `just fmt` | 127: `topcoat` isn't installed on this host; CI runs it |
| clippy | `just clippy` | 0 |
| test | `just test` | 0 |
| doc | `just doc` | 0 |
| deny | `just deny` | 0 |
| cap | `just cap` | 0 |
| deps | `just deps` | 0 |
| hygiene | `just hygiene` | 0 |
| ts | `just ts` | 0 |
| long-task check | `npx playwright test -c <copy of playwright.config.mjs with the pair project on Desktop Chrome> --project=390 chat.spec.mjs -g "1,000 messages" --repeat-each=5` | 0: 5/5 passed |
| ui (local, Chromium) | same config, `--project=390 --project=1440` | 0: 109 passed, 14 skipped |
| ui (webkit) | `just ui` | not run: WebKit isn't installed on this host; CI's `ui` job covers it |

CI at cf41239bf9f7172eff1f8616a7353f1fc27e55b3: msrv (ubuntu, macos), stable (ubuntu, macos), ts, no-node and ui (incl. WebKit, topcoat fmt, `chat.spec.mjs:131`) all SUCCESS.

Not verifiable here: AC 4 (the owner switches to Scene on the phone).
