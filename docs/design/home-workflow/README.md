# Native Home: your personas' chats only; workflow in its plugin's place; the rest under Others (#229)

Today every shed ACP lane shows on the native Home as a bare `claude-acp` row (owner screenshot, 2026-11-11), because
the app has no rule for a session with no persona. This design gives it one generic rule. Nothing in it is about
drover except that drover is one plugin claiming one source. Owner decision: q1295 (conductor thread t59).

**The rule**, applied to every row of `GET /threads`:

1. A row with a persona and no `workflow` label is a **chat**. It goes in Home's list, as `apple/design/v1/home-*`.
2. A row with `workflow.source` set is **workflow**, never a chat. If an installed plugin claims that source, the row
   is drawn in that plugin's place on the persona it is attached to. drover claims `drover` and draws its sessions in
   Conductor's **Workflow** sheet.
3. **Everything else goes to Others:** chats with no persona, and workflow whose source no installed plugin claims.
   Nothing is ever hidden: uninstall drover and its lanes show up in Others.

## Pages and how to make the PNGs

There are three pages, built on the live design system as `docs/design/screens-as-data/` was. They link
`web/src/app.css`, `docs/design/native-shell/ns.css`, `docs/design/screens-as-data/sd.css` and the farm pack, and copy
none of them. `hw.css` adds only what this design needs (it lists its class names). `hw.js` draws the frames.

| page | frames |
|---|---|
| `home.html` | Home at 390, and the Mac window (Home as the sidebar) |
| `workflow.html` | Conductor's Workflow sheet at 390, a round opened as a chat, and the sheet as a panel on the Mac |
| `others.html` | Others with drover attached, Others with drover not installed, and the Mac window with Others in the sidebar |

To see the frames, serve the repo root over http and open a page: with no query it shows every frame as a gallery,
`?f=<frame>` shows one frame alone, and `?mode=dark|light` picks the mode. To remake every PNG, run
`node docs/design/home-workflow/shoot.mjs <chrome-headless-shell>` from the repo root. It audits each shot for
sideways scroll, anything past the right edge, and text cut without an ellipsis. **All 13 pass.**

**Look:** Modern, farm scheme. The 390 shots are 390 × 844 at 2×, in dark and light. The 1440 shots are 1440 × 900
at 1×. **Data:** illustrative. The issues are real cooee and herdsman titles, but their stages are made up.

## The PNGs and the decision each shows

| PNG | shows | decision |
|---|---|---|
| `home-390-dark.png`, `-light.png` | **Home:** the avatar, `cooee` and `+`; the pinned personas; then **only your personas' chats** (avatar, `Name  chat title`, last line with `•••` while working or a red `!` when it waits on you, age, unread). No lane rows. At the bottom, set apart by a hairline, is one **Others** row: a tray icon, the titles inside it, and its **count** (`4`) in a neutral pill, so it is not mistaken for an unread badge. | q1295 list = personas' chats only; Others is one row at the bottom with a count |
| `home-1440.png` (dark) | The Mac window: the same Home as the sidebar, with Others at its foot, and Coder's chat open beside it. | the rule is the same on Mac |
| `workflow-390-dark.png`, `-light.png` | **Conductor › Workflow.** The sheet opens below Conductor's header strip, so the **Workflow** chip that opened it is still visible, highlighted, beside **Queue** and **Events**. The chip carries the needs-you count. Under `drover · 5 issues · 1 needs you`, it shows **one row per issue** (`repo #n · stage round`, the state's figure, the state, the title, the age), grouped needs you → failed → working → ready. cooee #217 is unfolded: its rounds hang on a line (impl ✓, review 1 FAIL, fix 1 live), and the current round shows its last tool lines, `open chat ›` and `PR #228 ›`. | scope 3: a Workflow chip on the strip; a sheet; rows per issue in that order; tap a row to unfold its rounds |
| `workflow-round-390-dark.png`, `-light.png` | A round tapped: **cooee #217 · fix 1** opens as a **normal chat**, with `‹ Workflow` to go back. The header shows the round's state figure, its name and `drover · impl · mini`. drover's prompt opens the chat, then come the lane's tool rows and replies, and the composer (`Message this lane`). | scope 3: tap a round to open that session as a normal chat |
| `workflow-1440.png` (light) | The Mac window: Conductor is selected in the sidebar, and the Workflow sheet is a centred panel over its chat. It shows the same rows, with #217 unfolded. | the sheet on Mac |
| `others-390-dark.png`, `-light.png` | **Others** with drover attached. It is a pushed screen (`‹ Home`, large title, one line on what it holds), in two parts: **chats · no persona** (Bonjour probe, jq filter, New chat: each shows its agent and folder, because there is no persona to show) and **nightly · no plugin claims it**, a workflow session from a source nothing claims. A footnote says where drover's sessions went. | scope 4: one plain list of every other session |
| `others-unclaimed-390-dark.png`, `-light.png` | Others with **drover not installed**: every drover round session (`cooee #229 · design`, `herdsman #722 · review 1`, …) falls here under **drover · no plugin claims it**, each with its state word. The footnote says that installing drover and attaching it to a persona moves them to that persona's Workflow sheet. | scope 4: nothing is lost while drover is not installed; AC 2's fallback |
| `others-1440.png` (dark) | The Mac window with drover not installed: Others in the sidebar (`‹ Home`), and cooee #217 · fix 1 open beside it as a normal chat. | Others on Mac |

## What is reused and what is new

- **Reused:** the schemes and tokens, `.av`, `.bd`, `.ch`, `.chip`, the chat (`.tc .blk .tw`), the farm's state
  figures and words; from #116's `ns.css`, the phone and window frames, the nav bar and large title (`.nb .lt`), the
  persona row (`.pr`) and the **issue row with its rounds (`.wr .wh .rns .rn .rnb`)**, unchanged from #116's
  Workflow tab; from #125's `sd.css`, Home's pinned row and bar (`.pin .nb.hm`), the chat header with its **header
  strip** (`.ch2 .hs .hsc .hsn`), the sheet (`.ps .psh`) and the Mac window (`.wd.mac .side`).
- **New** (`hw.css`): the Home row's `Name  title` line (`.hwr`), the working dots and waits-on-you mark (`.hwk .hnd`),
  the Others row and its count (`.hwo .hwn`), the neutral avatar for a session with no persona (`.oav`; a state
  figure for workflow, `.fgv`), the selected strip chip (`.hsc.on`), and drover's prompt in a round's chat (`.hwb`).

## For impl (scope 5: native only, plus the server and API change it needs)

- **Classify once, in Core:** `Home(chats, others)` and `Workflow(source → sessions)` are built from `[Row]` and the
  installed plugins' claims, in one pure function. AC 2's tests drive that function, and Home's reducer. A row is
  exactly one of `chat(persona)`, `workflow(claimed by plugin)` or `other`, so the type cannot put a row in two places.
- **The claim:** a plugin's manifest gets `claims = ["drover"]` (`cooee-plugin.toml`, in `screens.schema.json`'s
  `places` and `attach` sections). Attach shows it as one more `it may` line: *Shows sessions labelled
  source=drover in Conductor's Workflow sheet*.
- **The sheet's data:** `GET /threads` already carries `workflow: {source, group, item, role, round, url}`. Grouping
  by `group`+`item` gives one row per issue, and `role`+`round` give its rounds. The state and age come from the row's
  `state` and `at`.

## Open questions for the owner

1. **Where the sheet's data comes from.** The issue names `GET /workflow` (#127), but `docs/api.md` has no such route
   today. **Recommend:** build the sheet from `GET /threads`' existing `workflow` labels, grouped by group+item: the
   stage is the newest round's `role`, and the title is its thread's `title`. That means no new server route. The
   alternative is to add `GET /workflow` (one row per item, with its rounds), with its `docs/api.md` entry, in
   #229's PR.
2. **How a plugin claims a source.** **Recommend:** `claims = ["<source>"]` in the manifest, shown at attach. The
   alternative is a new place type, `workflow-sheet`, with `source` on it, which would also let a plugin put the
   sheet on another persona's strip.
3. **What Others' count counts.** **Recommend:** every session in Others, drawn as a neutral pill, plus the normal
   unread badge on the row when any of them is unread. The alternative is unread only.
4. **A no-persona chat's avatar.** **Recommend:** its title's initial on a neutral disc, as drawn, so it never looks
   like a persona. The alternative is the agent's own mark, which would mean naming a provider in the UI.
5. **Workflow chip count.** **Recommend:** the needs-you count only (`1`), in red as Queue's is. The alternative is
   no count, with the needs-you dot left to Conductor's chat row on Home.
