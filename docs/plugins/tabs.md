# How a tab runs

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
  `Last-Event-ID`. An event stream should send a comment now and then (cooee passes it on, so a closed tab is
  noticed); one silent for a minute is closed. A call has a total deadline, connect to last byte (30 s through the
  tab's proxy, 5 s for a screen, 10 s for an action or a card's answer; a stream's deadline ends once its head is in),
  and a backend has at most 8 calls in flight: one more fails at once with a 502 `busy`.
