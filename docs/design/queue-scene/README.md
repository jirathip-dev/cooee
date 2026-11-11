# Queue in Scene: a `scene` block (a horse per lane) and fold-open rounds in Queue rows (#181)

drover's Queue sheet (#125) gets two things, both from data the plugin already has. **A `scene` block** at the top: the
theme's scene strip with one horse per lane, each acting its state over its name card; a tap opens the lane's chat.
**Rows that fold open** as the web Workflow tab does (#116/#122, `../native-shell/workflow-390-dark.png`): the issue's
rounds on a line, the live one showing its lane, harness and model, host, last tool lines, `lane chat ›` and `PR #n ›`,
and a needs-you round showing the ask's buttons. Owner: q1087.

**Proof it fits:** every frame is `fixtures/queue.json` (#125's `queue.json` plus the `scene` block and each row's
`rounds`) drawn by one mockup renderer (`qs.js`, one function per block, as #125's `sd.js`) on the design system:
`web/src/app.css`, `web/schemes/farm*.json`, #116's `ns.css`, #125's `sd.css`; `qs.css` adds only the strip and the
fold. **Nothing is copied:** the horses and the strip are the native app's own PNGs, referenced in place from
`apple/Sources/CooeeUI/Sprites/` (`horse-<coat>-<pose>.png`, `scene-strip.png`, rasterised from the farm pack by
`apple/design/sprites.mjs`); the Modern row figures are the farm pack's `sprites.svg`; the Terminal figures are
`CooeeCore.LaneState.figure`.

## Frames

| PNG | What it shows | Decision |
|---|---|---|
| `scene-390-light.png`, `scene-390-dark.png` | The Queue sheet in **Scene (farm)**. The scene block is first: the farm strip at 180 pt, as Home's pinned row (`HomeRows.PinnedRow`), with the seven lanes as horses in their coats, each over a name card (`#730 design`) and its theme word. They are sorted into places: **at the gate** first (#730, needs you: red word, red-ringed card), then **refused** at the fence (#655), **on the trail** ploughing (#641, #722, #609), **in the stable** (#708 ready), **grazing** (#702 idle). Then #125's chips, stats and the fleet card, whose rows carry the same horse as their figure. | The app sorts and places the lanes; the plugin sends only `id`, `title`, `stage`, `state` (and `tool`, `since`, `tap`). Needs-you is leftmost so it is visible without scrolling; the strip scrolls sideways past four and a half lanes. The name card is light in both modes, as the app's pins, because the art is daylight. |
| `lane-390-light.png`, `lane-390-dark.png` | **A lane tapped from the strip** (#641): `tap: {thread: "herdsman-impl-641"}` pushes the lane's chat on the sheet, `‹ Queue` back. Its head is the horse, `#641 · fix 1`, `Claude Code · opus 5.5 · mini · 6m` and `PR #704 ›`; then the lane's messages, its tool rows (the running one in amber) and the composer. | The tap is #125's existing `thread` tap; nothing new in the schema. On iPhone it pushes inside the sheet, so `‹ Queue` returns to the same strip. |
| `scene-1440-light.png`, `scene-1440-dark.png` | The Mac window: sidebar, the Conductor chat with drover's Queue and Events chips, and the Queue as #125's centred panel. The strip is 200 pt and shows all seven lanes; #641 is folded open under it. | One JSON per screen; the app lays it out per size. On the Mac a horse tap opens the lane's chat in the detail (as #116 at 1440) and closes the panel. |
| `rows-modern-390-dark.png`, `-light.png` | **Modern**, the sheet scrolled to the in-progress rows: #641 **unfolded** (chevron down, its subtitle gone). Its rounds hang off a line: `impl ✓ 3 commits 2h ›`, `review 1 FAIL · 2 findings 51m ›`, and the current `fix 1 editing hosts/bootstrap.rs`, open: `herdsman-impl-641 · Claude Code · opus 5.5 · mini`, its last three tool lines (the live one marked `▍` in amber), `lane chat ›`, `PR #704 ›`. The other rows (#722, #609, #708, #702) stay **folded**, with #125's one-line subtitle. **No strip:** in Modern the scene block is skipped. | Rows are #116's Workflow row, inside #125's fleet card. A tap on a row with `rounds` folds it in place on the client (no request); a past round's `›` follows its own `tap` (the verdict screen, the commits). |
| `ask-modern-390-dark.png`, `-light.png` | **Needs you**, Modern: #730's `design` round unfolded with the ask in place: the lane line, the design's images, `approve · recommended`, `revise`, `reject` (red, confirms), a revise note, `lane chat ›`. Below, #655 (round limit) folded. | A needs-you round's buttons are #125's `images` and `choice` blocks inside the round (`blocks`), so the same answer goes through `POST /action` as the Conductor's ask card. One ask, answerable in either place. |
| `rows-terminal-390-dark.png`, `-light.png` | The same unfolded #641 in **Terminal**: SF Mono, the ASCII lane figures (`~o=`, `(!)`, `x_x`, `\o/`, `-.-`) as the row figures, the round line dashed with a glyph per state (`+` done, `x` failed, `>` working, `!` needs), bracketed buttons. No strip. | Terminal keeps the fold; only the figures and the line change. |
| `ask-terminal-390-dark.png`, `-light.png` | #730's needs-you round in Terminal: `[approve]` `[revise]` `[reject]`, the note, `[lane chat ›]`; #655 below with `x_x`. | As above. |
| `rows-modern-1440-dark.png`, `rows-terminal-1440-dark.png` | The Mac panel in Modern and Terminal with #730 (the ask) and #641 (live) both unfolded. | Several rows may be open at once; the app keeps which are open per sheet. |

`scene.html` holds the Scene frames and `rows.html` the Modern / Terminal ones (`?look=modern|terminal`). Serve the repo
root over http and open `docs/design/queue-scene/scene.html?mode=light`. To shoot (repo root):
`node docs/design/queue-scene/shoot.mjs <chrome-headless-shell>`. It audits each shot for sideways scroll, text past
the right edge and cut lines without an ellipsis; all 16 pass.

## The schema (for impl, after #140)

**`scene`** (new block). The plugin sends lanes and states; the app picks the art from the theme.

```json
{ "type": "scene", "id": "lanes", "label": "herdsman's lanes", "lanes": [
  { "id": "l-641", "title": "#641", "stage": "fix 1", "state": "working", "tool": "edit",
    "since": "2026-11-09T09:35:00Z", "tap": { "thread": "herdsman-impl-641" } } ] }
```

- `state` is the lane state the app already has (`CooeeCore.LaneState`: `working`, `needs`, `done`, `idle`, `fail`;
  `merged` leaves the strip on the next op). The pose is `LaneState.pose`, the word the theme's.
- The **coat** is the app's, kept stable per issue; the plugin never sends art, coat or place.
- **Modern and Terminal skip the block** (an unknown or skipped block is already allowed, #125 §Blocks): the rows under
  it carry every lane, so a second list would show each lane twice.
- Live: `{"op": "merge", "id": "l-641", "fields": {"state": "fail", "stage": "review 2"}}` moves the horse to its new
  place; `remove` takes a merged lane off; `insert` into `lanes` adds a spawned one (#125's ops, keyed by id).

**`rounds`** (new field on `row`). A row with rounds folds open on tap; without, it is #125's row.

```json
{ "type": "row", "id": "q-641", "title": "#641 hosts bootstrap…", "subtitle": "impl ✓ · review 1 ✗ · fix 1 ▸ …",
  "rounds": [
    { "id": "r-641-impl", "name": "impl", "state": "done", "summary": "✓ 3 commits", "since": "…", "tap": { "url": "…/commits" } },
    { "id": "r-641-fix1", "name": "fix 1", "state": "working", "summary": "editing hosts/bootstrap.rs", "since": "…",
      "lane": { "name": "herdsman-impl-641", "harness": "Claude Code", "model": "opus 5.5", "host": "mini", "lines": ["…", "$ cargo test …"] },
      "blocks": [],
      "links": [ { "label": "lane chat", "tap": { "thread": "herdsman-impl-641" } }, { "label": "PR #704", "tap": { "url": "…" } } ] } ] }
```

- The **last round is current** and the only one drawn open; its `lane`, `blocks` (any block, e.g. `images` and
  `choice` for an ask) and `links` are its body. Past rounds are one line with an optional `tap`.
- Live: `merge` on a round id updates its summary or lane lines in place; `insert` into the row's `rounds` starts a new
  round (it becomes current); `replace` on the row id swaps it whole. A row that was open stays open.

## Open questions for the owner

1. **Who decides the strip shows: plugin or app?** The issue says drover "sends the `scene` block at the top when the
   look is Scene". **Recommend:** drover always sends it and the app skips it outside Scene, so the plugin never needs
   to know the look and one cached answer (and the offline frame) works after a look switch. The alternative is a
   `look` query on `GET /screens/queue`. The frames are the same either way.
2. **Props for the places.** The native sprites act working (plough) and idle (graze), but `needs`, `done` and `fail`
   stand without a gate, stable or fence. Here the place is carried by the order, the word and the red card.
   **Recommend:** a follow-up that adds the gate, stable and fence to the farm pack's `s-scene-needs|done|fail` (as
   #116's `horses.svg` drew them) and re-runs `apple/design/sprites.mjs`; no schema change.
3. **Fold state.** **Recommend:** the app remembers which rows are open per sheet for the session, and opens a row
   whose current round turns needs-you. The alternative is the plugin's `open: true` on a row.
4. **The strip's name card** shows `#n stage`, not the issue title (it does not fit at 92 pt). The row below has the
   title. Confirm that is enough, or add the repo when the chips are on `all` (`herdsman #641`).
