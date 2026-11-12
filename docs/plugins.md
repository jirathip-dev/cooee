# Plugins

A plugin adds a tab beside Chat. It is a backend of its own (on this machine or on the tailnet) that serves a UI
module, an ES module that cooee runs in the tab. herdsman's Queue, Asks and Fleets tabs come this way, served by herdsman itself
as Drover ([architecture.md](architecture.md#plugins-m4)).

## Configuration

One `[[plugins]]` entry per plugin in `config.toml`:

```toml
[[plugins]]
name = "hello"                    # its URL: /p/hello (lowercase letters, digits and -)
label = "Hello"                   # the tab's label
url = "http://127.0.0.1:4800"     # its backend: plain http, on this machine or the tailnet
module = "/tab.js"                # its UI module, a path on the backend
token = "a-long-random-secret"    # shared with the backend; at least 16 characters
```

`cooee serve` refuses to start when an entry breaks a rule (no token, two plugins of one name, …).

## Installing plugins

A plugin can also be installed, like a lazy.nvim plugin: a git repo with `cooee-plugin.toml` at its root.

```toml
name = "hello"                                      # its folder and lock entry: lowercase letters, digits and -
version = "0.1.0"
build = "node --check server.mjs"                   # run with sh -c in the checkout
run = "TOKEN=$COOEE_PLUGIN_TOKEN exec node server.mjs"  # the backend: gets PORT, COOEE_PLUGIN_TOKEN and COOEE
needs = ["node"]                                    # optional: tools it needs on PATH

[[tabs]]                                            # one or more; each becomes a [[plugins]] entry
name = "hello"
label = "Hello"
module = "/tab.js"
```

A plugin may also bring agent profiles, `[[profiles]]` with the fields of an owner's profile
([architecture.md](architecture.md#agent-profiles-40)), their paths in its checkout. The add and update confirm lists
them; they are offered in the new-thread form while the plugin is installed and on.
Its backend may serve them instead (one per fleet, say): `GET /cooee/profiles` (with the token, read as the sidebar
loads and as a thread starts) answers `{"profiles": [...]}` in the same format, each with optional `hidden = true` and
a `why` line (not shown nor started, e.g. while its guard can't be proven), an `avatar` (a `/path` on the backend,
an `https://` or `data:image/` URL, or a sprite id of the theme's; none: the initials) and `threads`, the cooee
thread ids it lists as that agent's (its side threads, the newest its ongoing chat). Each is an agent in the sidebar,
under the plugin's name, named by its `label`. That list replaces the manifest's: a profile it doesn't list isn't
offered; a 404 offers the manifest's, any other answer (or a backend not up yet) none. When a thread starts as one of
the plugin's profiles, its backend gets `POST /cooee/thread-started` `{"thread", "profile"}` (the profile's name), so
it can attach the thread.

- `cooee plugin add <name | git-url> [--rev R] [--yes]` clones it into `<data>/plugins/<name>/repo`, shows what it will
  run (the URL, the commit, the exact build and run commands, and "not reviewed" when it isn't in the index) and asks
  to go ahead. Then it builds the commit in `builds/<commit>`, runs the backend on a spare port until it serves its
  first tab's module (the health check), writes the tabs' `[[plugins]]` entries with a fresh token to
  `<data>/plugins/plugins.toml` (read with the config's own) and records it in `<data>/plugins.lock` (name, URL,
  commit, port; JSON).
- `cooee plugin update [<name>] [--yes]` fetches the index's pinned commit (for a plugin added by URL, the latest on its
  default branch), confirms, builds it in a new folder and switches only once the build and the health check pass;
  otherwise the old one keeps running. The build before stays, to roll back to.
- `cooee plugin remove <name> [--purge]` stops it and removes its entries; its folder stays unless `--purge`.
- `cooee plugin list`: name, version, commit, source (index or url) and state.
- `cooee plugin check <answer.json>...` (#138) validates a plugin's answers (a screen or any block, a list of them,
  stream events, places) against `docs/plugins/screens.schema.json`, the JSON Schema of #125's blocks, and names each
  field that breaks it (`$.blocks[0].rows[0].badge.tone: "red" is not one of …`). An unknown block type passes.

`cooee serve` runs each enabled plugin's backend as its child (no launchd job per plugin) on its port, with its output
in `<data>/plugins/<name>/plugin.log` (its builds' too). A backend that exits restarts after 1 s, then 2, 4 and 8; the
5th crash within 10 minutes turns the plugin off, with an alert. Settings › Plugins on the phone does the same as the
CLI: each plugin's on/off, [update], [logs], [roll back] and [remove], and [+ add] with the index or a URL. Adding and
updating show the same confirm, which holds only for the commit it showed.

**The index** is `plugins/index.toml` in cooee's repo (`[[plugin]]`: `name`, `url`, pinned `commit`, `description`,
and `first_party`), built in; `plugin_index` in the config points at another. Adding or moving an entry is a normal
reviewed PR.

**[improve]** opens a thread with the first agent in shed (its provider's default account) in the plugin's checkout,
on a `cooee-improve` branch. [approve] commits the branch, builds it and swaps it in (the tab's frame reloads) with
the build before kept for [roll back]; a failed build leaves the old one running. A first-party plugin says "improve
via an issue" instead: it changes through cooee's own issue → review flow.

**Safety.** Every tab runs in its sandboxed frame with its plugin's own token, and reaches only its own backend; no
plugin can read cooee's data folder through the API. Its backend runs as the owner on the Mac, so adding, updating and
improving are owner-only: the CLI on the Mac, or a paired device (a plugin's token is refused), after a confirm.

## How a tab runs

- `/p/<name>` is the tab: cooee's bar with the tabs, and a frame. The frame's page, `/p/<name>/frame`, is the
  module (fetched from the backend) after cooee's small plugin API, in one inline script.
- **Isolation:** the frame is sandboxed (`sandbox="allow-scripts"` and a `sandbox` Content-Security-Policy, so its
  origin is opaque even when opened on its own). It has no cookies, no storage of cooee's, no access to the page
  around it or to other plugins, and its policy allows only that one script: no connections, no other scripts. A
  module is one self-contained file (no imports).
- **The API**, the global `cooee` in the module:
  - `await cooee.fetch(path, {method, type, body})` → `{status, ok, type, text}`: `GET` or `POST` `path` on this
    plugin's backend. The page around the frame calls `/p/<name>/api<path>` with the device's cookie, and cooee calls
    the backend with `Authorization: Bearer <token>`. The backend never sees the device's cookie, and a path that
    would leave `/p/<name>/api/` is refused.
  - `cooee.events(path, ({event, data}) => …)`: the backend's event stream at `path`, through the same proxy. It
    returns a function that closes it.
  - The theme: cooee's CSS variables (`--bg`, `--fg`, `--card`, `--edge`, `--mute`, `--link`, `--g`, `--a`, `--r`,
    …) are set on the frame's root, so `background: var(--card)` matches the app.
  - The tab's URL hash (`/p/<name>#lane/3`) is the frame's `location.hash`.
  - `await cooee.unlock()`: shows cooee's own "Unlock with Face ID" button over the tab and resolves once the device
    is unlocked with its passkey (the `terminal` tab's gate; a stream that ends for good, such as a locked tab's, sends
    `closed`).
- cooee calls the backend over HTTP/1.0, one connection per call, and passes on only the content type, `Accept` and
  `Last-Event-ID`. An event stream should send a comment now and then so a closed tab is noticed.

## From the backend to cooee

The backend authenticates with the same token (`Authorization: Bearer <token>`); any other call is refused (401).

- `POST /plugin/push` `{"text": "…", "path": "lane/3", "tag": "lane"}`: a push to every paired device, titled with
  the tab's label; a tap opens `/p/<name>#lane/3` (`path` is optional). A newer push of the same `tag` (optional,
  default `plugin-<name>`) replaces the older one. It answers with the notification sent.
- `POST /plugin/dot` `{"on": true}`: a dot on the plugin's tab (`false` clears it), until cooee restarts.
- `POST /plugin/card` `{"thread": "<id>", "text": "Ship it?", "options": ["yes", "no"]}`: a choice card in a
  thread of a persona the plugin is attached to (docs/api.md; any other thread's is refused, 400 with why), pushed like an approval, with an "other" box for a free answer. The owner's tap goes back to the backend as
  `POST /cooee/choice` `{"thread", "item", "option", "by"}` (with the token; `by` is the device's label); a 2xx
  marks the card answered (its ✓ line says by whom and when, or the backend's `{"line": "…"}`), a 409 closes it with
  its `line` (else `stale`), anything else lets it be tapped again. A conductor's cards (herdsman#553 rows 30–33):
  - `"recommend": "yes"` shows that option first, marked recommended.
  - `"answered": "yes", "by": "conductor-t6"`: the backend answered it itself (no push); the owner may [change] it
    for 30 minutes, and the change comes as a choice.
  - `"form": "config", "detail": "<diff>"`: a config change's diff with [apply] [discard]; while it waits, the
    thread's bar and its row in the list say `<text> waiting`.
  - `"form": "secret", "detail": "NAME"`: a masked field with [save] [dismiss]; the choice carries the `value`,
    which cooee never keeps or logs. The line reads `secret NAME set ✓`.
  - `"form": "thread"`: the other open threads as choices (the option is the thread's id).
  - `"form": "run-card", "detail": "<command>", "sha": "<its sha>"`: a herdsman run card (herdsman#538): the command,
    its sha's first 12 characters and [approve] [deny]. [approve] carries the card's `sha` as the `value`, sent
    once; the backend's line (`… exit N`, then the output tail in a fence) shows under the card, its 409 (expired,
    changed) closes it.
- `POST /plugin/handoff` `{"from": "<id>", "to": "<id>", "text": "…"}`: `from` shows a `continued in <to>` chip,
  `to` the text with a `from <from>` chip; `POST /plugin/line` `{"thread", "from", "text"}`: a line flowing back
  into `thread` with a `from` chip. A chip opens that thread (row 35).

## First-party plugin: machines

`crates/cooee-machines`, built into cooee: the health of the machines shed runs on. A `[[plugins]]` entry named
`machines` turns it on for that instance, and `cooee serve` runs its backend itself at the entry's `url`:

```toml
[[plugins]]
name = "machines"
label = "Machines"
url = "http://127.0.0.1:4801"     # a free port; cooee listens there for the plugin
module = "/tab.js"
token = "another-long-random-secret"
```

Once a minute it asks shed (never SSH itself): `shed machine ls`, then `shed machine stats` for this machine and
`shed --on <m> machine stats` for each added one (shed answers offline within 5 s). Each card shows online or offline
with when it was last seen, load per CPU, memory pressure, swap, free disk and shed's terminals by state; each number
has a 24 h sparkline, and a tap opens its chart. Only the samples newer than the last kept one are asked for. A
machine going offline, under 10 GB of free disk or at 90 % memory pressure or more sends one push (tag `machine`, a
tap opens `#<machine>`) when its set of problems changes, and the tab shows a dot while any machine has one.

## First-party plugin: terminal

`crates/cooee-terminal`, built into cooee: a real shell on the Mac from the phone, for the owner-only commands a chat
hands over (setup, keychain, `launchctl` …), instead of an SSH app. `cooee setup` adds its entry to each instance on
the instance's port + 1000 with a fresh token:

```toml
[[plugins]]
name = "terminal"
label = "Terminal"
url = "http://127.0.0.1:5747"
module = "/tab.js"
token = "a-third-long-random-secret"
```

- **Shells:** [+ shell] starts `$SHELL -l` in `~`, as the owner, in a shed terminal labelled `cooee.owner=1`, so it
  outlives the page and cooee. The list has the owner's shells, then the agents' terminals (every other shed
  terminal but sign-ins), which can be watched but never typed into (403). [close] ends a shell's whole process tree.
- **View:** shed keeps the screen (`screen.watch`); the backend streams its updates (rows of coloured runs) as
  `screen` events, and the tab draws them as a monospace grid in the theme's colours. Pinch zooms the font, and the
  shell is resized to fit; [earlier lines] shows the scrollback. No terminal emulator runs on the phone.
- **Input:** the keyboard's text goes in as text (Thai too), in order; the key bar at the tab's foot, on the keyboard
  while it is open: Esc, Ctrl (sticky for the next letter), Tab, ↑ ↓ ← →, ^C, paste and ↑ history (the lines typed
  here, to type again).
- **Gate:** the device's passkey (Face ID) opens the tab, registered on first use; 10 minutes without a touch or key
  lock it (blurred, no input) until Face ID again, and the shells keep running ([architecture.md](architecture.md#plugins-m4)).
- **Insert text:** `POST /insert` `{"text"}` (through the tab's proxy) types into the shell used last, else the
  newest, else a new one, and never presses Enter: trailing line ends go, a `\` continuation joins its lines and any
  other line end becomes `; `. It answers `{"terminal"}`. A chat's code block has **[▸ terminal]** beside [copy] when this plugin is on
  (#29): the owner's tap opens the Terminal tab, which asks for Face ID first when locked, then inserts the block's
  text and opens that shell (the tab's `#name`).

Its other calls: `GET /list`, `POST /shell`, `GET /watch?t=<name>` (the stream), `POST /input` `{terminal, text}`,
`/keys` `{terminal, keys}` (shed's key names), `/resize` `{terminal, cols, rows}`, `/close` `{terminal}`,
`/scrollback` `{terminal}` and `/seen` (use, which keeps the unlock alive).

## Screens

A plugin can instead (or as well) answer its screens as JSON made of cooee's blocks, which the native app draws: no
frontend code (the spec: [design/screens-as-data](design/screens-as-data/README.md)). Its manifest says where they go
and what it may do:

```toml
[[places]]
type = "header-strip"             # a chip under a persona's chat header; or "menu-row" (the avatar menu), "chat-cards"
id = "queue"
persona = "conductor"             # the persona it attaches to by default
label = "Queue"
screen = "/screens/queue"         # a path on its backend, under /screens/

[permissions]                     # shown when it's attached to a persona; cooee refuses the rest
places = ["header-strip"]
actions = ["drive", "drop"]       # what its screens may POST
destructive = ["drop"]            # always confirmed first by the app
push = false
network = []
```

cooee answers `GET /p/<plugin>/screens/...` from the backend's `GET /screens/...` (a screen, or its event stream of
`{seq, ops}`) and passes `POST /p/<plugin>/action` to its `POST /action` with `{screen, block, action, values, by}`.
An action not in `actions` gets a 403 and never reaches the backend. A plugin with screens only has no tab.
`docs/plugins/hello-py/` is a whole one: `hello.py`, a live screen in about 30 lines of Python, no dependencies.

## Example

`docs/plugins/hello/` is a plugin in about 30 lines, used by the tests (`web/e2e/plugins.spec.mjs`; its manifest by
`web/e2e/plugin-manager.spec.mjs`). Run it with
`PORT=4800 COOEE=http://127.0.0.1:4747 TOKEN=a-long-random-secret node docs/plugins/hello/server.mjs` and the entry
above.

The backend, `server.mjs`:

```js
import fs from "node:fs";
import http from "node:http";

const { PORT = "4800", COOEE = "http://127.0.0.1:4747", TOKEN } = process.env;
const streams = new Set();
const tell = (event, data) => { for (const s of streams) s.write(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`); };
const json = (res, status, v) => res.writeHead(status, { "content-type": "application/json" }).end(JSON.stringify(v));
const toCooee = async (path, body) => (await fetch(COOEE + path, {
  method: "POST", headers: { authorization: `Bearer ${TOKEN}`, "content-type": "application/json" }, body: JSON.stringify(body),
})).status;
const read = async (req) => { let b = ""; for await (const c of req) b += c; return b ? JSON.parse(b) : {}; };

http.createServer(async (req, res) => {
  if (req.headers.authorization !== `Bearer ${TOKEN}`) return json(res, 401, "no token");
  if (req.url === "/tab.js") return res.writeHead(200, { "content-type": "text/javascript" }).end(fs.readFileSync(new URL("tab.js", import.meta.url)));
  if (req.url === "/whoami") return json(res, 200, { token: "ok", cookie: req.headers.cookie ?? null });
  if (req.url === "/events") {
    res.writeHead(200, { "content-type": "text/event-stream" }).write(": open\n\n");
    streams.add(res);
    req.on("close", () => streams.delete(res));
    return tell("hello", "the stream is open");
  }
  const body = await read(req);
  if (req.url === "/notify") return json(res, 200, await toCooee("/plugin/push", { text: "hello from the plugin", path: "hi" }));
  if (req.url === "/ask") return json(res, 200, await toCooee("/plugin/card", { thread: body.thread, text: "Ship it?", options: ["yes", "no"] }));
  if (req.url === "/cooee/choice") {
    tell("choice", body.option);
    if (body.option === "approve" && body.value) return json(res, 200, { line: `ran: exit 0\n\`\`\`\nat ${body.value.slice(0, 12)}\n\`\`\`` });
    return json(res, 200, "ok");
  }
  json(res, 404, "no such path");
}).listen(Number(PORT), "127.0.0.1");
```

The module, `tab.js`:

```js
const log = document.body.appendChild(document.createElement("ul"));
const line = (text) => { log.appendChild(document.createElement("li")).textContent = text; };
const me = await cooee.fetch("/whoami");
line(`whoami ${me.status}: ${me.text}`);
cooee.events("/events", (e) => line(`${e.event}: ${e.data}`));
for (const path of ["/notify", "/ask"]) {
  const b = document.body.appendChild(document.createElement("button"));
  b.textContent = path.slice(1);
  b.onclick = async () => {
    const r = await cooee.fetch(path, { method: "POST", type: "application/json", body: JSON.stringify({ thread: location.hash.slice(1) }) });
    line(`${path.slice(1)} ${r.status}: ${r.text}`);
  };
}
```
