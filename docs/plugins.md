# Plugins

A plugin adds a tab beside Chat. It is a backend of its own (on this machine or on the tailnet) that serves a UI
module, an ES module that cooee runs in the tab. herdsman's Queue, Asks and Fleets tabs come this way, through the
[cooee-drover](https://github.com/jirathip-dev/cooee-drover) plugin ([architecture.md](architecture.md#plugins-m4)).

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
- cooee calls the backend over HTTP/1.0, one connection per call, and passes on only the content type, `Accept` and
  `Last-Event-ID`. An event stream should send a comment now and then so a closed tab is noticed.

## From the backend to cooee

The backend authenticates with the same token (`Authorization: Bearer <token>`); any other call is refused (401).

- `POST /plugin/push` `{"text": "…", "path": "lane/3", "tag": "lane"}`: a push to every paired device, titled with
  the tab's label; a tap opens `/p/<name>#lane/3` (`path` is optional). A newer push of the same `tag` (optional,
  default `plugin-<name>`) replaces the older one. It answers with the notification sent.
- `POST /plugin/dot` `{"on": true}`: a dot on the plugin's tab (`false` clears it), until cooee restarts.
- `POST /plugin/card` `{"thread": "<id>", "text": "Ship it?", "options": ["yes", "no"]}`: a choice card in a
  thread, pushed like an approval, with an "other" box for a free answer. The owner's tap goes back to the backend as
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

## Example

`docs/plugins/hello/` is a plugin in about 30 lines, used by the tests (`web/e2e/plugins.spec.mjs`; its manifest by
`web/e2e/plugin-manager.spec.mjs`). Run it with
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
  if (req.url === "/cooee/choice") { tell("choice", body.option); return json(res, 200, "ok"); }
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
