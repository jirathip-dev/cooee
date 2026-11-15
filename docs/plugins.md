# Plugins

## Today

- **Plugins:** git repos added from a curated index, supervised backends, tabs and screens as data (#51, #205, [`plugins/index.toml`](../plugins/index.toml)).
- **Fleet screens:** the app draws herdsman's fleet settings and pushes the screens it taps (#211).
- **First-party plugins:** `machines` (per-machine health) and `terminal` (your shells, behind Face ID) (#50, #91).
- **Run cards:** herdsman's commands shown with their sha, approved or denied from the phone (#114).

## How it works

A plugin adds a tab beside Chat. It is a backend of its own (on this machine or on the tailnet) that serves a UI
module, an ES module that cooee runs in the tab, or native screens (#125). herdsman's Queue, Events and Fleets screens
come this way, through herdsman's own `drover` plugin (below).

A plugin is a tab beside Chat: a backend of its own and a UI module it serves. The module
runs in a sandboxed frame (an opaque origin with a strict Content-Security-Policy) and reaches only its own backend,
through the page around it and cooee's proxy (`/p/<name>/api/…`), which adds the plugin's token and drops the device's
cookie. The backend posts pushes and choice cards back with that token; the owner's answer to a card goes to it.

Plugins are installed like lazy.nvim's (#37): a git repo with a `cooee-plugin.toml`, added from the built-in index
(`plugins/index.toml`) or by URL with `cooee plugin add` or Settings › Plugins. Each commit builds in its own folder
and goes live only after its backend served its first tab's module; `plugins.lock` pins it and keeps the build before
to roll back to. `cooee serve` runs every enabled backend as its child (`cooee-server/src/manage.rs`): restarts with
backoff, 5 crashes in 10 minutes turn it off with an alert. [improve] puts an agent on a branch of the plugin's
checkout; approving builds and swaps it in the same way.

**The passkey gate (#28).** The `terminal` tab runs commands on the Mac, so a paired cookie is not enough: the device
needs its passkey (WebAuthn, Face ID on iPhone), registered on its first use of the tab. The ceremony runs in cooee's
page (`/passkey/options`, then `/passkey/register` or `/passkey/unlock`), never in the sandboxed frame; cooee checks it
(`cooee-server/src/passkey.rs`: ES256 only, this host's origin and relying party, user verified) and keeps the
credential in the device's entry in `devices.json`. An unlock lives in memory and lasts while the tab is used: each
proxied call counts, and 10 minutes without one locks it (403 `locked`; an open stream ends at its next event) until
Face ID again. The proxy names the device to the backend (`Cooee-Device`), which refuses any call without it. Every
unlock and every shell started goes to `<data>/terminal.log` with the time and device; keystrokes never do.

**drover** (in the index) is herdsman's own plugin: `cooee-plugin.toml` at the root of
[jirathip-dev/herdsman](https://github.com/jirathip-dev/herdsman) ([herdsman#814](https://github.com/jirathip-dev/herdsman/issues/814)),
whose `herdsman serve` answers the screens itself: Queue and Events chips on the Conductor persona's header strip, its
asks as chat cards and a Fleets row in the menu. Data flows one way:

```text
cooee → herdsman serve (/screens/…, /action)
```

Its manifest's `attach` (#226) points at the `herdsman serve` already running on the Mac (`127.0.0.1:7777`), so cooee
uses it instead of starting a second one, and runs `run` only when nothing answers there. Its `attach_token`
(`~/.config/herdsman/cooee-token`) is the file that serve keeps its cooee token in; cooee reads it at `add`. herdsman is the engine:
queue, drive, lanes, reviews, gates and merge; the archived bridge plugin it replaces is gone from the index. The lanes themselves are not in
these tabs: they are shed sessions, shown as chats under Workflow ([workflow.md](workflow.md)).

### Configuration

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

### Installing plugins

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
([profiles.md](profiles.md)), their paths in its checkout. The add and update confirm lists
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
- `cooee plugin check <answer.json|cooee-plugin.toml>...` (#138) validates a plugin's answers (a screen or any block, a
  list of them, stream events, places) against `docs/plugins/screens.schema.json`, the JSON Schema of #125's blocks, and
  names each field that breaks it (`$.blocks[0].rows[0].badge.tone: "red" is not one of …`). An unknown block type
  passes. A `.toml` file is checked as a manifest, by the rules `cooee plugin add` applies.

A manifest's `attach = "http://127.0.0.1:<port>"` (#226) names a backend that may be running already, like herdsman's
own `herdsman serve` on :7777: while that port answers, `add` checks it instead of starting `run`, and `cooee serve`
uses it and starts nothing (`plugin list` says `attached`); when it doesn't, `run` starts on that port. Its
`attach_token` (`~/` or absolute; q1290) names the 0600 file the running backend keeps its token in, created by the
backend: `add` reads it and calls the backend with that token, which `run` then gets as `COOEE_PLUGIN_TOKEN` too.

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

### How a tab runs

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

### From the backend to cooee

The backend authenticates with the same token (`Authorization: Bearer <token>`); any other call is refused (401).

- `POST /plugin/push` `{"text": "…", "path": "lane/3", "tag": "lane"}`: a push to every paired device, titled with
  the tab's label; a tap opens `/p/<name>#lane/3` (`path` is optional). A newer push of the same `tag` (optional,
  default `plugin-<name>`) replaces the older one. It answers with the notification sent.
- `POST /plugin/dot` `{"on": true}`: a dot on the plugin's tab (`false` clears it), until cooee restarts.
- `POST /plugin/card` `{"thread": "<id>", "text": "Ship it?", "options": ["yes", "no"]}`: a choice card in a
  thread of a persona the plugin is attached to (docs/personas.md; any other thread's is refused, 400 with why), pushed like an approval, with an "other" box for a free answer. The owner's tap goes back to the backend as
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

### Screens

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

### Example

`docs/plugins/hello/` is a plugin in about 30 lines, used by the tests (`crates/cooee/tests/plugins.rs`; its manifest by
`crates/cooee/tests/plugin_manager.rs`). Run it with
`PORT=4800 COOEE=http://127.0.0.1:4747 TOKEN=a-long-random-secret node docs/plugins/hello/server.mjs` and the entry
above.

The backend, `server.mjs`:

```js
// cooee's example plugin backend (docs/plugins.md): `PORT=4800 COOEE=http://127.0.0.1:4747 TOKEN=… node server.mjs`.
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
  // Only cooee calls this backend, and it sends the plugin's token.
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
    // A run card's [approve] carries its sha; herdsman would run it and answer the exit code and output tail.
    if (body.option === "approve" && body.value) return json(res, 200, { line: `ran: exit 0\n\`\`\`\nat ${body.value.slice(0, 12)}\n\`\`\`` });
    return json(res, 200, "ok");
  }
  json(res, 404, "no such path");
}).listen(Number(PORT), "127.0.0.1");
```

The module, `tab.js`:

```js
// The example plugin's tab: runs in cooee's sandboxed frame with the `cooee` API (docs/plugins.md).
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

## API

### Settings › Plugins

| Call | Body | Answer |
|---|---|---|
| `GET /plugins` | | `{installed: {name, version, commit, listed, state, enabled, first_party, improving?, previous?, tabs, places, permissions}[], index: {name, description}[], jobs: {name: text}, issues}` |
| `GET /plugins/log?name=N` | | the plugin's log, text |
| `POST /plugins/plan` | `{source}` or `{name, at}` | `{commit, text}`: what would run, to confirm |
| `POST /plugins/add`, `/plugins/update` | the plan's body and its `commit` (a confirm holds for that commit only) | a JSON string (`"installing"`, …) |
| `POST /plugins/enable` | `{name, on}` | a JSON string |
| `POST /plugins/improve` | `{name, text}` | `{thread}` |
| `POST /plugins/approve`, `/plugins/rollback`, `/plugins/remove` | `{name}` | a JSON string |

### Plugin backends

`GET` or `POST /p/{name}/api/{path}` reaches the plugin's own backend through cooee's proxy, with its token, never the
cookie. An event stream comes back as SSE messages of `{event, data}`. The `terminal` plugin's calls need the device
unlocked with its passkey: else 403 `locked` (header `Cooee-Locked: 1`).

### Plugin screens (#125)

A plugin whose manifest has `[[places]]` serves its screens as blocks ([Screens](#screens)). A device
reaches them through cooee, with the plugin's token, never the cookie:

| Call | Body | Answer |
|---|---|---|
| `GET /p/{plugin}/screens/{path}` | | the backend's `GET /screens/{path}`: a screen (JSON), or its stream (SSE, each event `{seq, ops}` as sent, its id kept; `Last-Event-ID` goes along) |
| `POST /p/{plugin}/action` | `{screen, block, action, values}` | the backend's answer to `POST /action` (`by`: the device), as it sent it; 403 for an action not in its `[permissions] actions` (never sent); 404 for no installed plugin with places; 502 when the backend can't be reached |
