# Plugins

## Today

- **Plugins:** git repos added from a curated index, supervised backends, tabs and screens as data (#51, #205, [`plugins/index.toml`](../../plugins/index.toml)).
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
these tabs: they are shed sessions, shown as chats under Workflow ([workflow.md](../workflow.md)).
