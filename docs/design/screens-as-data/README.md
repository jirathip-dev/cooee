# Plugin screens as data: a block set the native app draws (#125)

A plugin returns its screens as **JSON made of cooee's blocks**, and the native app (SwiftUI, iPhone and Mac,
iOS/macOS 26; q946/q947) draws them with its own views. The plugin author writes no frontend code. Any backend that
answers JSON works (Python, Node, Rust, shell). Every plugin gets cooee's look, its themes, dark mode and
accessibility (Dynamic Type, VoiceOver labels, 44 pt targets) for free, because the app draws everything. **No
third-party code runs in the app, only data.** The `web` block stays as the escape hatch. The frozen PWA keeps its
module tabs as they are, and herdsman's lifeboat page (#725) is plain and draws no blocks. Owner decisions: q941,
q943, q946/q947, q954, and the answers to revision 0's questions (below). **Nothing is built** until the Swift spike
(cooee#124) decides.

**Proof it fits:** drover's Queue, Events and asks are written as fixtures (`fixtures/*.json`). One mockup renderer
(`sd.js`, one function per block) draws them in the native app's new home (q954) on the design system. The PNGs
below are those fixtures drawn, not hand-made HTML. `fixtures/hello.json` is the real answer of `hello/hello.py`, a
whole plugin in 22 lines of Python.

## Revision 1 (q946, q947, q954)

- **The native app is the only client.** The web renderer here is a mockup, not a client. The PWA is frozen, its
  module tabs unchanged (Q1), and herdsman's board draws no blocks (Q5: no).
- **No tab bar:** home is the owner's avatar (its menu: Settings, Machines + Terminal, Plugins), a pinned personas
  row, then the chat list. **Plugins attach to a persona, not the app.** drover attaches to the Conductor only: its
  **Queue** (issues holding their lanes and rounds) and **Events** open as sheets from buttons in the Conductor
  chat's header, and **asks are chat cards** in that chat, with no screen of their own. A new spec section,
  **Places** (below), says where a plugin's screens go.
- **Fleets** is no longer a place of its own: it is a sheet under the Queue sheet's `⋯` (`fleets.json` is unchanged).
  The design review is a chat card, no longer a pushed screen. Their frames are gone.
- **Answered:** images in v1 (Q2, yes), one `POST /action` (Q3), and a JSON Schema plus `cooee plugin check` (Q4, yes).

## The PNGs and the decision each shows

| PNG | shows | decision |
|---|---|---|
| `home-390-light.png`, `-dark.png` | Home (q954): the owner's avatar, `cooee`, `+`, the pinned personas, the chats. The avatar menu is open: Settings, Machines + Terminal, then **plugins**: drover (attached to `Conductor`), hello, and Add a plugin…. | A plugin's optional `menu-row` lives under the avatar menu's Plugins |
| `conductor-390-light.png`, `-dark.png` | The Conductor chat. The header carries the persona, then drover's **Queue** (count `7`) and **Events** buttons. drover's asks are **chat cards** at the end of the chat: the design card (images, approve · recommended / revise / reject, a revise note), a routine ask (options, other…, its countdown), and needs-human. The run card is just above. | `header-button` and `chat-cards` places; asks are cards, not a screen |
| `queue-390-light.png`, `-dark.png` | `queue.json` as a sheet over the chat: `⋯` (Fleets…, Drive all now), Queue, Done. Then the repo `chips`, a `stat` row with sparklines, and the fleet `card` with its rows. **Each issue row holds its lane and rounds** (`impl ✓ · review 1 ✗ · fix 1 ▸ herdsman-impl-641 · mini`), the stage badge and a live age. The blocked chain is indented. | A header button opens the plugin's screen as a sheet; Fleets moves under `⋯` |
| `queue-live-390-light.png` | The same sheet after `queue-stream.json`'s three events: `#722` merged to `review 1` (its new lane), `#708` removed, `#456` replaced (now in impl), `#686`'s subtitle merged, and the header button's count set to 6. | Live updates: ops keyed by block id, not a re-fetch |
| `events-390-light.png`, `-dark.png` | `events.json` as a sheet: kind chips, today's events newest first (merge, verdict, ask, spawn, FAIL, round limit), each with the theme's state figure and its time. A tap opens the PR, or a lane's chat (`tap: thread`). | Events is a plain list screen; its stream inserts rows |
| `confirm-390-dark.png` | `merge anyway` on the needs-human card: the app's own action sheet (title, message, red Merge, Cancel). | Destructive confirms are data; the app draws them and the plugin can't skip them |
| `hello-390-light.png`, `-dark.png` | **A third-party plugin:** `hello.py`'s screen as a sheet from its avatar-menu row: markdown, a stat and a Wave button. A tap POSTs `wave`, and the answer's `merge` op bumps the stat. | The scale: 22 lines of Python, no frontend |
| `wide-1440-light.png`, `-dark.png` | The Mac window: the sidebar (home: avatar, pinned personas, chats), the Conductor chat with its header buttons, and the Queue sheet as a centred panel. The same JSON as on the iPhone. | One JSON per screen; the app lays it out per size |
| `blocks-1440.png` | The block set: each block's JSON beside its drawing, with `hello.py` first. Full height. | The block set v1 |

`screens.html` holds the frames and `blocks.html` the catalogue. Serve the repo root over http (the pages fetch
`web/`, `packs/` and `../native-shell/ns.css` for the tokens). `?f=<frame>&mode=dark|light` shows one frame.
`node docs/design/screens-as-data/shoot.mjs <chrome-headless-shell>`, run from the repo root, regenerates every PNG
and checks each one for sideways scroll, anything past the right edge, and cut text without an ellipsis. **All 15
pass.** The design card's thumbnails are the shots themselves, so a fresh checkout runs it twice.
**Look:** Modern, farm scheme, drawn on the design system's tokens. The home layout (q954) is drawn here for the first
time, small; its own design is not this issue. **Data:** illustrative, from real issues.

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
`icon` is a name from cooee's small fixed set (`plus`, `gear`, `share`, `info`, `filter`, …). The app maps each to an
SF Symbol. `label` is the accessible name.

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

**web**: the escape hatch: a page on the plugin's backend in a sandboxed `WKWebView` with no bridge and no cookies,
at a fixed `height`. The theme's colours are passed in as CSS variables, as for today's module tabs. Use it for charts or art the blocks can't express.
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
| `{"op": "badge", "count"}` / `{"op": "dot", "on"}` | the count or dot on the place's header button (or menu row) |

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

### Places: where a plugin's screens go (q954)

A plugin declares its places in its manifest (or answers `GET /cooee/places` in the same shape, as profiles do
today). `fixtures/places.json` is drover's:
```json
{ "places": [
  { "type": "header-button", "id": "queue", "persona": "conductor", "label": "Queue", "icon": "queue", "screen": "/screens/queue" },
  { "type": "header-button", "id": "events", "persona": "conductor", "label": "Events", "icon": "events", "screen": "/screens/events" },
  { "type": "chat-cards", "persona": "conductor" },
  { "type": "menu-row", "label": "drover", "subtitle": "herdsman's fleets", "screen": "/screens/fleets" } ] }
```

- **`header-button`**: an icon button in a persona's chat header (at most 2 per persona) that opens `screen` as a
  sheet: a page sheet on the iPhone, a centred panel on the Mac. The sheet's bar is the screen's toolbar actions
  (`⋯` menus), its title, and Done. A pushed screen inside it (`tap: open`) gets `‹`. The button's count comes from
  the screen's stream (`{"op": "badge"}`).
- **`chat-cards`**: the plugin may post cards into that persona's chats: `POST /plugin/card {"thread", "card":
  <card block>}`, with the card's blocks (text, images, choice, buttons). Later ops on its ids (`POST /plugin/ops
  {"thread", "ops"}`) update it in place, and its actions come back as the same `POST /action`. Today's
  `/plugin/card` forms (choice, run card, secret, config) become card blocks. A card can also be pushed (`push`).
- **`menu-row`** (optional): a row under the avatar menu's **Plugins**. It opens `screen` as a sheet. A plugin with
  no persona (hello) lives only here.
- A plugin is attached to the personas its places name. The owner can move it to another persona in its Plugins
  row; the default comes from the manifest. Places never add a tab or touch another plugin's persona.

### The schema and `cooee plugin check` (Q4)

`docs/plugins/screens.schema.json` (to build) describes every block, op, place and action answer. `cooee plugin
check` GETs each place's screen, validates it, and lints it: ids unique, a destructive button with no confirm, a
`secret` sent with a value, images over 2 MB. The app still skips unknown blocks, so a newer plugin works on an
older app.

## 2. What drover's screens need that the blocks can't express

1. **Images** (the design card's PNGs): not in the brief's list, so it is **added** as the `images` block (Q2: in v1).
2. **A live clock** (`lapses in 11m`, `takes the recommendation in 12m`, row ages): **added** as `since` / `until`
   values, ticked by the client. Without them a stream would have to send an op every minute.
3. **The blocked chain** (`#686` held until `#456`) and **an issue holding its rounds**: **added** as row `indent` and a toned `subtitle`, which carries the rounds line. A Workflow-style unfolding row could come in v2.
4. **A lane's live chat** (tool lines, the transcript): **not a block.** A row's `tap: {"thread": id}` opens cooee's
   own chat, which is better than any copy of it.
5. **Reordering the queue** (drag to move): **not in v1.** The Queue sheet uses swipe actions (`Top`, `Drop`).
   A `reorder` action on a list could come in v2.
6. **Scene** (the horses acting their states): theme art, cooee's own. Rows get the theme's state figure through
   `figure.state`, but a whole-field Scene is not a plugin block.
7. **Terminal** and **charts you tap into** (the machines plugin's 24 h charts): the `web` escape hatch, or
   cooee's own views (Machines + Terminal sits in the avatar menu).

## Suggested build issues, after cooee#124 decides

1. **SwiftUI renderer:** the blocks, ops and `POST /action` (`List`, `Section`, `Menu`, `Toggle`,
   `confirmationDialog`, `.swipeActions`, `.sheet`), with this folder's fixtures as snapshot tests on iPhone and Mac.
2. **Places:** header buttons, chat cards (and `/plugin/card` moving to card blocks), the avatar menu's Plugins rows,
   and attaching a plugin to a persona.
3. **Schema and `cooee plugin check`.**
4. **drover on screens:** cooee-drover serves Queue, Events, Fleets and its cards in these shapes, attached to the
   Conductor.
5. **Docs:** `docs/plugins.md` gets the hello example and the block and place reference.

## Open questions for the owner

1. **Header buttons per persona.** **Recommend:** at most 2 per persona, with more going under one `⋯`, so the
   header stays readable on the iPhone. The alternative is no limit, with the app folding extras into `⋯`.
2. **Who may post chat cards.** **Recommend:** only into chats of the personas the plugin is attached to. The owner
   sees the attachment in its Plugins row. The alternative is any chat a plugin started.
3. **A card's push.** **Recommend:** a card with `"push": true` notifies, as approvals do today, and the rest stay
   quiet in the chat. The alternative is that every card notifies.
