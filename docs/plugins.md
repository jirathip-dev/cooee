# Plugins

A plugin adds a tab beside Chat. It is a backend of its own (on this machine or on the tailnet) that serves a UI
module, an ES module that cooee runs in the tab. drover's board moves into cooee this way, in stages.

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

- `POST /plugin/push` `{"text": "…", "path": "lane/3"}`: a push to every paired device, titled with the tab's label;
  a tap opens `/p/<name>#lane/3` (`path` is optional). It answers with the notification sent.
- `POST /plugin/card` `{"thread": "<id>", "text": "Ship it?", "options": ["yes", "no"]}`: a choice card in a
  thread, pushed like an approval. The owner's tap goes back to the backend as `POST /cooee/choice`
  `{"thread", "item", "option"}` (with the token); a 2xx marks the card answered, anything else lets it be tapped
  again.

## Example

`docs/plugins/hello/` is a plugin in about 30 lines, used by the tests (`web/e2e/plugins.spec.mjs`). Run it with
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
