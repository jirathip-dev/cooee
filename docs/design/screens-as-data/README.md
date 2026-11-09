# Plugin screens as data: a block set every client draws natively (#125)

A plugin returns its screens as **JSON made of cooee's blocks**, and each client draws them with its own views: the
Swift app (cooee#124), the web app, and herdsman's board page (herdsman#725). The plugin author writes no frontend
code. Any backend that answers JSON works (Python, Node, Rust, shell). Every plugin gets cooee's look, its themes,
dark mode and accessibility (Dynamic Type, VoiceOver labels, 44 pt targets) on every client, because the client
draws everything. **No third-party code runs in the app, only data.** The `web` block stays as the escape hatch.
Owner decisions: q941 and q943 (conductor thread t59). **Nothing is built** until the Swift spike (cooee#124) decides.

**Proof it fits:** drover's Queue, Asks, Fleets and design-review screens are written as fixtures
(`fixtures/*.json`), and one renderer (`sd.js`, about 150 lines, one function per block) draws them on #116's
native shell. The PNGs below are those fixtures drawn, not hand-made HTML. `fixtures/hello.json` is the real answer
of `hello/hello.py`, a whole plugin in 22 lines of Python.

## The PNGs and the decision each shows

| PNG | shows | decision |
|---|---|---|
| `queue-390-light.png`, `-dark.png` | drover's Queue from `queue.json`: `chips` (repo filter), a `stat` row with sparklines, the herdsman fleet `card` (badge `driving`, Drive now / Pause), and `section`s of `row`s, each with the theme's state figure, a stage badge and a live age. `#686` is indented under `#456` (`indent`, the blocked chain) and has a red subtitle. | Queue fits in blocks, with two row fields added: `indent` and a toned `subtitle` |
| `queue-live-390-light.png` | The same screen after `queue-stream.json`'s three events: `#722` merged to `review 1`, `#708` removed, `#456` replaced (now in impl), `#686`'s subtitle merged, and the Asks tab badge set to 3. | Live updates: ops keyed by block id (replace, merge, remove, insert), not a re-fetch |
| `asks-390-dark.png`, `-light.png` | Asks from `asks.json`: the run card (markdown with a code fence, the sha line, approve / deny, `lapses in 11m` ticking), the design ask (`images` + approve · recommended / revise / reject), a routine ask (options, other…, the default's countdown), and needs-human below the fold. | Asks is `card` + `text` + `choice`; time ticks on the client (`since` / `until`) |
| `confirm-390-dark.png` | `merge anyway`'s confirm, drawn as the client's own action sheet: the title, the message, a red Merge, and Cancel. | Destructive confirms are data (`confirm`); the client draws them and the plugin can't skip them |
| `fleets-390-dark.png`, `-light.png` | The Fleets sheet (`fleets.json`) over Queue, opened from its ⚙ toolbar action: Cancel · Fleets · Save, one `section` + `form` per fleet (toggle, pickers, text, a set `secret`), a link row, and a destructive button. | Settings are a `sheet` of `form`s; Save sends every field by `name` |
| `design-390-light.png`, `-dark.png` | The design review (`design-review.json`), pushed from Asks: `‹ Asks`, an inline title, the card with large `images` (they scroll sideways), the decisions list, and the choice with a revise note. | Images need a block (gap 1, added) |
| `hello-390-light.png`, `-dark.png` | **A third-party plugin:** `hello.py`'s screen, pushed from More: markdown, a stat, a Wave button. A tap POSTs `wave`, and the answer's `merge` op bumps the stat. | The scale: 22 lines of Python, no frontend |
| `wide-1440-light.png`, `-dark.png` | 1440: the rail, Queue as the list column, and the design review as the detail (`‹ Asks`, its toolbar ↗). The same JSON as at 390. | One JSON per screen; the client lays it out per size |
| `blocks-1440.png` | The block set: each block's JSON beside its drawing, with `hello.py` first. Full height. | The block set v1 |

`screens.html` holds the frames and `blocks.html` the catalogue. Serve the repo root over http (the pages fetch
`web/`, `packs/` and `../native-shell/ns.css`). `?f=<frame>&mode=dark|light` shows one frame.
`node docs/design/screens-as-data/shoot.mjs <chrome-headless-shell>`, run from the repo root, regenerates every PNG
and checks each one for sideways scroll, anything past the right edge, and cut text without an ellipsis. **All 15
pass.** The design review's thumbnails are the shots themselves, so a fresh checkout runs it twice.
**Look:** Modern, farm scheme, #116's shell. **Data:** illustrative, from #116's real issues.

## 1. The spec

### Shape

A screen is one JSON object. Every block has a `type`, and may have an `id`, which live updates and actions need.
Unknown fields are ignored and an unknown `type` is skipped, so old clients survive new blocks. Text is plain except
in `text.markdown`. Colour comes only as a **tone**: `ok`, `warn`, `bad`, `info`, `mute` or `accent`, or a state
(`working`, `needs`, `done`, `fail`, `idle`). The theme picks the colour, and a state also gets the theme's figure
and word ("on the trail"). A plugin can never set a colour, font or size.

**Taps** (`tap` on a row or card, a toolbar action): `{"open": "/screens/x?n=1"}` pushes a screen,
`{"sheet": "/screens/y"}` presents one, `{"action": "name", "values": {…}}` POSTs, `{"thread": "<id>"}` opens a
cooee chat (a lane's), and `{"url": "https://…"}` opens the browser, outside the app. A path is on the plugin's own
backend, reached through cooee's proxy with the plugin's token, as `cooee.fetch` is today.

**Time:** `{"since": iso}` or `{"until": iso}` in place of a string value, or `"until"` on a `text` block with `{rel}`
in its markdown. The client draws it relative (`9m`, `lapses in 11m`) and ticks it with no stream traffic.

### The blocks

**screen**: the large title, toolbar actions, its blocks, and an optional live `stream`. `"large": false` gives an
inline title (a pushed detail). The host adds `‹ back`.
```json
{ "type": "screen", "id": "lanes", "title": "Lanes", "stream": "/screens/lanes/stream",
  "actions": [ { "icon": "plus", "label": "New lane", "tap": { "sheet": "/screens/lanes/new" } } ],
  "blocks": [ { "type": "text", "markdown": "3 lanes on **mini**." } ] }
```
`icon` is a name from cooee's small fixed set (`plus`, `gear`, `share`, `info`, `filter`, …). The Swift app maps
each to an SF Symbol, and the web app to its stroke icons. `label` is the accessible name.

**section**: a header, blocks, and a footer.
```json
{ "type": "section", "header": "machines", "footer": "Seen once a minute.",
  "blocks": [ { "type": "list", "rows": [ { "type": "row", "title": "mini", "value": "online" } ] } ] }
```

**list** and **row**: an inset-grouped list (`UITableView` / `List` on iOS). A row has a `title`, a `subtitle`
(a string, or `{text, tone}`), a `badge {text, tone}`, a `figure` (`{state, tool}`: the theme's sprite; `{icon}`: a
tile; `{avatar}`: an image or initials), a `value` (right-hand, muted), an `accessory` (`chevron` by default when it
has a tap), `swipe` actions, `tap`, `indent` (0–2, for a chain), and `style` (`link` or `destructive`).
```json
{ "type": "list", "id": "lanes", "rows": [
  { "type": "row", "id": "l-722", "figure": { "state": "working", "tool": "run" }, "title": "#722 spawn (shed)",
    "subtitle": "cargo test -p herdsman-core", "badge": { "text": "impl", "tone": "working" },
    "value": { "since": "2026-11-09T09:32:00Z" }, "tap": { "open": "/screens/lane?n=722" },
    "swipe": [ { "label": "Stop", "tone": "bad", "action": "stop" } ] } ] }
```

**card**: a titled group: a `tone` edge, a `badge`, `meta` (right), a `subtitle`, header `buttons`, and its blocks. A
whole-card `tap` is allowed.
```json
{ "type": "card", "id": "fleet", "tone": "accent", "title": "herdsman", "badge": { "text": "driving", "tone": "ok" },
  "subtitle": "7 lanes on mini",
  "buttons": [ { "type": "button", "label": "Drive now", "style": "primary", "action": "drive" } ],
  "blocks": [ { "type": "text", "markdown": "Next pass in 3 min." } ] }
```

**choice**: one answer. The `recommended` option is drawn first and marked. `other` adds a free-text box (`"for":
"revise"` ties it to one option). `default` shows its countdown. An option can carry `values` (a run card's `sha`)
and a `confirm`. This is today's `POST /plugin/card` choice, as a block.
```json
{ "type": "choice", "id": "q947", "action": "answer", "question": "Move the branch out?", "recommended": "move",
  "options": [ { "value": "refuse", "label": "refuse" }, { "value": "move", "label": "move it out" } ],
  "other": { "placeholder": "other…" },
  "default": { "value": "move", "until": "2026-11-09T09:53:00Z", "label": "takes the recommendation in {rel}" } }
```

**form**: fields by `name`: `text` (`placeholder`, `keyboard`: `default|url|number`), `toggle`, `picker`
(`options`; a menu on iOS) and `secret`. A secret is masked. It is sent once and never echoed back: the plugin only
says `"set": true`, and cooee never keeps or logs the value, as the `secret` card does today. `submit` adds a button.
In a sheet, the sheet's done action sends every form in it.
```json
{ "type": "form", "id": "f", "submit": { "label": "Save", "action": "save" }, "fields": [
  { "type": "text", "name": "branch", "label": "Branch", "value": "staging" },
  { "type": "toggle", "name": "drive", "label": "Drive", "value": true },
  { "type": "picker", "name": "host", "label": "Host", "value": "mini", "options": ["mini", "air"] },
  { "type": "secret", "name": "GH_TOKEN", "label": "GH_TOKEN", "set": true } ] }
```

**button**: `style` is `primary`, `plain` or `destructive`, with an `action` and its `values`. A `confirm`
(`title`, `message`, `ok`, `destructive`) is drawn by the client as its own action sheet or alert before anything is
sent. **A destructive button without a `confirm` gets one anyway**, titled with its label.
```json
{ "type": "button", "id": "rm", "label": "Remove cooee-drover…", "style": "destructive", "action": "remove-fleet",
  "values": { "fleet": "cooee-drover" },
  "confirm": { "title": "Remove cooee-drover?", "message": "Its queue and lanes stay on disk.", "ok": "Remove" } }
```

**chips**: filters. `multi` allows more than one, and a change POSTs `action` with `{name: selected}`.
```json
{ "type": "chips", "id": "repo", "name": "repo", "selected": ["herdsman"], "action": "filter",
  "options": [ { "value": "all", "label": "all" }, { "value": "herdsman", "label": "herdsman" } ] }
```

**stat** with an optional **sparkline**: one tile (`label`, `value`, `unit`, `tone`, `spark`: up to 48 numbers), or
`items` for a row of tiles. The sparkline is drawn by the client, in the theme's accent.
```json
{ "type": "stat", "id": "s", "items": [
  { "label": "merged · 7d", "value": "14", "spark": [1, 3, 2, 0, 4, 1, 3] },
  { "label": "lanes", "value": "7", "unit": "/ 8", "tone": "warn" } ] }
```

**text**: markdown: paragraphs, **bold**, *italic*, `code`, fences, lists and links. There is no HTML and there are
no images in markdown. `style: footnote` draws it small and muted.
```json
{ "type": "text", "markdown": "Run **once** on mini:\n\n```\ngit push origin --delete wip/x\n```\n\nlapses in {rel}",
  "until": "2026-11-09T09:52:00Z" }
```

**images** (added, gap 1): a row of pictures that scroll sideways (`size`: `small` or `large`). A tap opens one full
screen, with pinch to zoom. `src` is a path on the plugin's backend, or a `data:image/` URL.
```json
{ "type": "images", "id": "pngs", "size": "large", "items": [
  { "src": "/files/125/queue-390-light.png", "alt": "Queue", "caption": "queue" } ] }
```

**sheet**: a screen that slides up (a page sheet with its grabber; a centred panel at 1440): `cancel`, `title`, and a
`done {label, action}`.
```json
{ "type": "sheet", "id": "new-fleet", "title": "New fleet", "cancel": "Cancel", "done": { "label": "Add", "action": "add-fleet" },
  "blocks": [ { "type": "form", "id": "nf", "fields": [ { "type": "text", "name": "repo", "label": "Repo", "placeholder": "owner/name" } ] } ] }
```

**web**: the escape hatch: a page on the plugin's backend in a sandboxed frame (`WKWebView` with no bridge on iOS;
`sandbox="allow-scripts"` and the CSP of today's tabs on the web) at a fixed `height`. The theme's CSS variables are
passed in, as for today's module tabs. Use it for charts or art the blocks can't express.
```json
{ "type": "web", "id": "chart", "src": "/web/load-24h.html", "height": 120 }
```

### Live: a stream of ops keyed by id

A screen with `stream` opens it as server-sent events through the proxy (today's `cooee.events`). Each event is
`{"seq": n, "ops": [...]}`:

| op | does |
|---|---|
| `{"op": "replace", "id", "block"}` | swaps the block with that id |
| `{"op": "merge", "id", "fields"}` | shallow-merges fields into it (a row's badge, a stat's value) |
| `{"op": "remove", "id"}` | removes it, animated |
| `{"op": "insert", "into", "before"\|"after", "block"}` | adds a block to the list or section `into`, at the end by default |
| `{"op": "screen", "screen"}` | replaces the whole screen (a gap in `seq`, or a big change) |
| `{"op": "badge", "count"}` / `{"op": "dot", "on"}` | the tab's count or dot |

The client resumes with `Last-Event-ID: <seq>`. A plugin that can't resume answers a `screen` op first. Without a
stream, pull to refresh re-GETs the screen. `queue-stream.json` is drawn in `queue-live-390-light.png`.

### Actions: one POST back

Any action is `POST <plugin>/action` with the screen, the block's id, the action name and the values:
```json
{ "screen": "asks", "block": "q947", "action": "answer", "values": { "choice": "move", "other": "" }, "by": "Guy's iPhone" }
```
`values` merges the button's or option's own `values`, the choice (`choice`, `other`), the form fields by `name` (in a
sheet, all of its forms), and the chips. The block is disabled with a spinner until the answer comes:

- **2xx:** `{"ops": [...]}` (the same ops as the stream, applied at once), and optionally `"toast": "Saved"`,
  `"close": true` (dismiss the sheet or pop) or `"open": "/screens/…"`.
- **409:** `{"line": "stale: answered by conductor-t6"}`: the block shows the line and stays closed, as today's cards.
- **anything else:** an error toast, and the block can be tapped again.

### Where it plugs in (a proposal: no API change in this issue)

A manifest tab gets `screen = "/screens/queue"` instead of `module = "/tab.js"` (and an `icon`). Everything else
stays: the token, the proxy, `cooee plugin add`, the index, and `/plugin/push`. A tab with a `module` keeps working,
as today's sandboxed frame. herdsman's board (#725) fetches the same paths with its own token and draws the same
blocks in its page.

## 2. What drover's screens need that the blocks can't express

1. **Images** (the design review's PNGs): not in the brief's list, so it is **added** as the `images` block.
2. **A live clock** (`lapses in 11m`, `takes the recommendation in 12m`, row ages): **added** as `since` / `until`
   values, ticked by the client. Without them a stream would have to send an op every minute.
3. **The blocked chain** (`#686` held until `#456`): **added** as row `indent` and a toned `subtitle`.
4. **A lane's live chat** (tool lines, the transcript): **not a block.** A row's `tap: {"thread": id}` opens cooee's
   own chat, which is better than any copy of it.
5. **Reordering the queue** (drag to move): **not in v1.** The Queue fixture uses swipe actions (`Top`, `Drop`).
   A `reorder` action on a list could come in v2.
6. **Scene** (the horses acting their states): theme art, cooee's own. Rows get the theme's state figure through
   `figure.state`, but a whole-field Scene is not a plugin block.
7. **The Terminal tab** and **charts you tap into** (the machines plugin's 24 h charts): the `web` escape hatch,
   or they stay first-party native tabs.

## Suggested build issues, after cooee#124 decides

1. **spec + web renderer:** the blocks, ops and action POST in `web/` (the renderer here grown up: accessible and
   tested), with a manifest `screen =` tab.
2. **Swift renderer:** the same blocks in SwiftUI (`List`, `Section`, `Menu`, `Toggle`, `confirmationDialog`,
   `.swipeActions`, `.sheet`), on the fixtures here as snapshot tests.
3. **drover on screens:** cooee-drover's Queue, Asks and Fleets serve these fixtures' shapes instead of its module.
4. **herdsman board (#725):** draw the same JSON in the lifeboat page.
5. **plugin SDK docs:** `docs/plugins.md` gets the hello example and the block reference. Optional tiny helpers
   (`cooee_screens.py`, an npm package) only if authors ask.

## Open questions for the owner

1. **Mixing module and screen tabs.** **Recommend:** keep module tabs working as they are (the escape for a whole
   tab), and put new first-party work on screens. The alternative is to deprecate modules once drover moves.
2. **Images in v1?** **Recommend:** yes, the `images` block, since a design review can't work without it. The
   alternative is a `web` block per review.
3. **One action endpoint or one per block?** **Recommend:** one `POST /action` with the block id, which is simple for
   a 20-line plugin. The alternative is a `path` per action.
4. **Validation.** **Recommend:** a JSON Schema in the repo, plus `cooee plugin check` linting a screen. The client
   still skips unknown blocks. The alternative is no schema, documented by example only.
5. **Where herdsman's own screens live.** **Recommend:** drover stays the plugin that serves them (cooee-drover), so
   herdsman's board and cooee draw one source. The alternative is herdsman serving them itself.
