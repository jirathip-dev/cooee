# Architecture

cooee is one binary on the owner's machine (Mac or Linux). It serves a web app (an installable PWA) on 127.0.0.1,
and Tailscale carries it to the owner's phone. Agents run under [shed](https://github.com/jirathip-dev/shed), which
keeps them going while the phone is away; cooee talks to shed and pings the phone when an agent needs the owner.

```
  phone (PWA, Safari/Chrome)                 owner's machine
 ┌──────────────────────────┐   Tailscale   ┌──────────────────────────────────────────────────────────┐
 │ thread list, chat, login │◄─────────────►│ cooee (one binary)                                       │
 │ web push ◄───────────────┼───────────────┤  ├─ cooee         args + wiring                          │
 └──────────────────────────┘    (M2)       │  ├─ cooee-server  HTTP on 127.0.0.1, live events,        │
                                            │  │                pairing, push (Tokio, via Topcoat)     │
                                            │  ├─ cooee-web     the UI: Topcoat pages (server-rendered)│
                                            │  └─ cooee-core    config, threads, store (no sockets)    │
                                            │         │ ACP over shed's API (M1)                       │
                                            │         ▼                                                │
                                            │  shed ──► agents: Claude Code, Codex, Gemini, Pi, any ACP│
                                            └──────────────────────────────────────────────────────────┘
```

## Crates

| Crate | Path | Holds | May depend on |
|---|---|---|---|
| `cooee-core` | `crates/cooee-core` | config, threads, the store, the chat (shed events → items), paired devices, providers (status, redaction), the shed client; no HTTP | no workspace crate |
| `cooee-web` | `web/` | the web UI, HTML-first (ADR 0001, #49): server-rendered pages and sheets, strict TypeScript in `web/ts` stripped by `build.rs` (oxc; no Node to build) only where script is needed, `app.css`, the PWA manifest, the themes and schemes (`look.rs`, `shown.rs`, `themes/`, `schemes/`); its script calls only `docs/api.md` | not the server or binary |
| `cooee-machines` | `crates/cooee-machines` | the first-party plugin `machines` ([plugins.md](plugins.md)): machines' health from shed, its backend and tab module | core |
| `cooee-terminal` | `crates/cooee-terminal` | the first-party plugin `terminal` ([plugins.md](plugins.md)): the owner's shells through shed, its backend and tab module | core |
| `cooee-server` | `crates/cooee-server` | HTTP, the threads' JSON API and SSE (M1), the Host/pairing gate, push (M2), logins (M3) | core, web |
| `cooee` | `crates/cooee` | the binary: arguments and wiring only | all |

`scripts/dep-direction.sh` enforces the last column in CI.

## Storage

One typed JSON file per thread, `<data>/threads/<id>.json`: `{"format": 1, "thread": {…}}`. Every write is
write-temp, fsync, rename, then fsync of the folder, under that thread's lock, so a crash leaves the old file or the new
one, never half of either. A file from a newer format is refused, not guessed at. `crates/cooee-core/tests/upgrade_compat.rs`
loads every earlier release's files (`tests/fixtures/upgrade/<version>/`) and fails if any field would be lost.

A thread's rows (items: your messages, replies, tool calls, approvals, plans) are JSON lines beside it,
`<data>/threads/<id>.jsonl`. Each change appends the item's whole new version as one fsynced line; reading folds them
by id, and a torn last line from a crash is skipped. A pending approval is stored before the page hears of it. Files sent
with a message sit in `<data>/files/<thread>/` (the item keeps their names; the agent gets their paths under the text),
and a thread's queued messages and its archived mark are in its JSON file.

## Chat (M1)

One thread is one ACP session on [shed](https://github.com/jirathip-dev/shed): `shed run --acp <agent> --name
cooee-<id> --label cooee=1 --label cooee.thread=<id> --cwd <folder>` (the CLI, as are `shed agents`; the socket has
no method for them). Everything else goes over shed's socket, one connection per request, each opening with the exact
`hello` of the pinned `shed-proto` (another version is refused with both versions named). Prompts, approvals and
cancels are framed `send`s, as `shed send|approve|cancel` do. One `events` stream (every terminal; cooee's carry `cooee=1`) feeds every thread;
after a restart cooee re-attaches each open thread to its terminal by name and replays from the last event it applied.

A new thread is stored and listed before shed starts its agent, so an ask that comes during the start is shown, and its
`random_id` is kept: a retried start (after a timeout or a crash) adopts that thread and never starts a second
terminal; a start whose CLI failed while the terminal came up anyway is kept too. Each socket call gives up after 10 s
(a `wait` gets its own timeout on top), so one stuck call never holds its thread; a message or answer to a terminal
that is gone gets "Not delivered" (410). A working turn with no shed event for 30 minutes shows as needs-you, with a
push, until its next event. Page loads run no command: the installed agents and the sign-ins' status are checked at
start and every 30 s (the sign-ins also after a login, sign-out or new default), and pages read that.

Each shed event maps to items through one exhaustive `match` (`cooee-core/src/chat.rs`). The page gets them over SSE
(`/threads/<id>/events`), each with id `<run>.<seq>`; a thread keeps its last 4,096 page events, so a page that
reconnects with `Last-Event-ID` gets exactly what it missed, and one from another run or too far behind gets `reset`
and reloads. Its first slice (`/threads/<id>/messages?around=&before=&after=`) carries the cursor it is current to,
taken under the thread's lock with the subscription, so nothing is lost or doubled.

## Workflow (#33)

`GET /threads` lists threads from two sources. **Chats** are the sessions the owner starts, listed by agent in the
sidebar (#86: each agent's oldest open chat is its ongoing one, the rest its side threads). **Workflow** is any ACP
session another tool started in shed, known only by its labels: `source` (the tool),
`group` (its repo), `item` (`#issue`), `role`, `round` and an optional `url`; one carrying a `cooee…` label is
cooee's own. cooee's one `events` stream follows every terminal: a new session's first event lists it (and at
start, `shed ls` lists the ones already running, replayed from shed's kept events), stored like a thread as
`w-<terminal>` so its rows outlive a restart. drover's Workflow tab shows them, never the agents sidebar; a session
goes once shed no longer lists it (checked at start and every 30 s). It opens as a normal chat with its role, round and a link to its `url`; the tool's prompts
show as messages, the reply box and [stop] reach its terminal, and only its tool closes it. Its turns end without a
push (approvals and errors still push).

herdsman's lanes (its impl, review and fix agents) are what fill this list: each runs as a shed ACP session labelled by
its source (the tool), `group` its repo, `item` its issue, with its `role` and `round`. So a lane is opened as a chat in
cooee core, never in a cooee-drover tab; those show the queue, asks and fleets ([Plugins](#plugins-m4)).

## Pairing and push (M2)

Everything but `/pair` (and `/pair/request`, `/pair/claim`, `/pair/cancel`, `/passkey/challenge`, `/passkey/signin`)
and the static files (manifest, icons, style, scripts, `sw.js`, `POST /look`, the themes' files) needs a paired device:
an unpaired page load goes to `/pair`, any other read or write gets 401. `/pair` is all an unpaired browser sees (#47,
q528; docs/design/pair): the theme's figure, three steps, six code boxes and the other ways, no thread titles or
messages. `cooee pair` (or Settings › devices › pair a device) shows a six-digit code and its `/pair?code=` link as a
QR code, good for 10 minutes and once; a new one drops the open one. The QR link fills the boxes and pairs at once.
Five wrong codes a minute from one address (the forwarded one behind `tailscale serve`) close the boxes for the rest
of the minute, and five misses drop the code. The browser that enters it gets a random token as an `HttpOnly`,
`SameSite=Strict` cookie. A `devices.json` that can't be read gets 503, never taken for "not paired".
`<data>/devices.json` (0600) keeps only SHA-256 hashes of codes and tokens, and the passkeys' public keys. The other
way round, a new device asks to pair (`/pair/request`), shows a two-word check word and polls `/pair/claim`; every
paired device gets a push and a card, and approving needs a passkey with Face ID (`/pair/answer`); a decline tells the
new device who said no. Requests last 5 minutes, at most 4 at once. Passkeys are WebAuthn, ES256 on the platform
authenticator, offered after pairing: the server checks the challenge (once, 5 minutes), the origin, the relying
party's hash, user verification and the signature. A device that lost its cookie signs back in with its passkey
(`/passkey/signin`) as a new device. `cooee devices` lists the devices and `cooee devices rm <id>` unpairs one (its
pushes stop too); Settings → devices does both from the phone and lists and removes passkeys. On iPhone, add cooee to
the Home Screen first and pair there: the app has its own cookies (the page says so in Safari).

Before that, every request's `Host` (and its `Origin`, when sent) must be localhost, the `bind` address or one of
`allowed_hosts`, or it gets 403, paired cookie or not. That blocks DNS rebinding. To reach cooee from the phone,
keep `bind` on 127.0.0.1, run `tailscale serve --bg 4747`, and put the machine's Tailscale name in `allowed_hosts`.

Web push (RFC 8291 aes128gcm, VAPID per RFC 8292, sent with `curl`): the VAPID key is made once and kept in the
macOS keychain (`security`, service `cooee`; a call that takes over 5 s, as an unseen prompt after a rebuild would,
is an error), else in `<data>/vapid.key` (0600). The thread list's 🔔 subscribes;
subscriptions sit in `<data>/push.json` (0600), each tied to its device. A push goes out when an agent asks for an
approval, a turn finishes while no page has the thread open, or an agent errors. Each one is a line about the thread:
`{title, body, tag: <thread>, url: /t/<thread>, item}`. The service worker (push, and a network-first copy of the pages and static files for offline) shows it under the
thread's tag, so a newer push replaces the older one. It shows nothing while that thread is open in view. A tap opens
the thread, and reading the thread past `item` closes it.

## Sign-ins (M3) and accounts

Each provider is one row of the `providers` table in the config: its name, the ACP agent it signs in, its CLI's own
login, logout and read-only status commands, and the variable that points its CLI at a config folder (`home`:
`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `GEMINI_CLI_HOME`). There is no provider-specific code beyond that table. The
status command signs an account in when it exits 0; its output, on one line and redacted, is the status line (a
JSON status, as `claude auth status` prints, becomes a plain line, its email or org and any `usage` shown apart).

A provider's accounts are the system default login (`~/.claude` and so on, the first row) and the ones added in
cooee: each a config folder `<data>/accounts/<provider>/<name>/` that the CLI gets through its variable, so the CLI
keeps its own login there or in its keychain. cooee never reads or stores a token. `.cooee.json` beside the folders
keeps only the provider's default account and when each login last succeeded. Settings' sign-ins row opens the
sign-ins card on the thread list: per provider, each account's state (signed in, signed out, needs sign-in again),
who it is, its last sign-in and usage, with `[sign in]`, `[sign out]` and `[make default]`; and `[+ account]`, which
runs the login in a new folder (`.new-<name>`) that is listed only once the login exits 0 and the status says signed
in, else removed. `[sign out]` (after a confirm) runs the logout for that folder, then removes it (the system
default's is never removed). A provider whose CLI is missing says "not installed on this Mac" with its install line.
The new-thread picker marks each agent by its default account, `✓` or `signed out` with a link to the sheet, and has
an account picker, the default preselected; the thread's agent starts as `shed run --acp <agent> … -- env
<variable>=<folder> <the agent's adapter>` (the system default: no `env`), and the thread's header names the account.

`[sign in]` runs the login command (for an added account, `env <variable>=<folder> <login>`) in a plain shed terminal (terminal mode, not ACP; over the socket: `run`, then
`resize` to 500 columns so a long link stays on one line), named `cooee-login-<provider>-<stamp>` and labelled
`cooee-login=1`. The page reads its visible screen every second (`GET /logins/<provider>`), its links tappable; a
code typed under it goes in with `send` then `keys enter`. A `wait` on the terminal ends when the command exits or
after 10 minutes; then the terminal is killed and removed, `[cancel]` does the same at once, and cooee removes any
`cooee-login` terminals a previous run left. The screen is only ever read on demand: never stored, logged or kept
in memory. Before it goes to the page, anything that looks like a token is replaced by `[redacted]`: a run of 20 or
more token characters with both a letter and a digit, and in a link the values of `token`, `secret`, `password` and
`key` parameters. The tokens themselves stay with the CLI.

## Plugins (M4)

A plugin is a tab beside Chat: a backend of its own and a UI module it serves ([plugins.md](plugins.md)). The module
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

**cooee-drover** ([jirathip-dev/cooee-drover](https://github.com/jirathip-dev/cooee-drover), in the index) is the
plugin that adds the fleet tabs: Queue, Asks and Fleets. Data flows one way:

```text
cooee → cooee-drover → herdsman /api/v1
```

cooee proxies the tab (`/p/cooee-drover/api/…`); cooee-drover passes each call through to herdsman's `/api/v1` and
reimplements nothing; herdsman is the engine: queue, drive, lanes, reviews, gates and merge. herdsman is to be
renamed drover ([herdsman#456](https://github.com/jirathip-dev/herdsman/issues/456)). The lanes themselves are not in
these tabs: they are shed sessions, shown as chats under Workflow (above).

## Configuration

One TOML file per instance, `$XDG_CONFIG_HOME/cooee/<instance>/config.toml` (else `~/.config/cooee/<instance>/…`, or
`--config <file>`). Every key is optional; an unknown key is an error that lists the known ones.

**Instances.** `--instance <name>` (default `cooee`) picks one: its own config, port, data folder (threads, devices,
push key) and pairing, so two instances never share state; both use the one shed server. Plugins are per instance, so
`cooee-drover` adds its tabs without touching `cooee`. `cooee install --instance <name> --port <p>` writes
`~/Library/LaunchAgents/dev.cooee.<name>.plist` (KeepAlive, log in the data folder) and the config's `port` if it has
no config yet, then prints the `launchctl bootstrap`, `tailscale serve` and (if shed isn't running) shed install lines
for the owner to run. `cooee doctor` checks the service, port, shed, push key and tailnet URL.

| Key | Default | Meaning |
|---|---|---|
| `port` | `4747` | The port `cooee serve` listens on (`0` picks a free one). |
| `bind` | `127.0.0.1` | The address it listens on. Anything but loopback needs `allowed_hosts`. |
| `allowed_hosts` | none | The names requests may use besides localhost and `bind`: the Tailscale name. Matched exactly, so `*` widens nothing. |
| `roots` | none | The folders a thread may work in: each root and the folders right inside it, nothing outside. |
| `shed` | `shed` | The `shed` binary. |
| `shed_dir` | shed's per-user server | shed's `--dir` (its socket `shed.sock` and state). |
| `data` | `$XDG_DATA_HOME/cooee/<instance>`, else `~/.local/share/cooee/<instance>` | Where threads, devices and push subscriptions are stored. |
| `providers` | Claude Code, Codex, Gemini | The sign-ins: `[[providers]]` with `name`, `agent` (its ACP agent id), `login`, `status` and `logout` (commands, as arrays), `install`, `home` (the CLI's config-folder variable; without it only the system default account) and `settings` (the variable its ACP adapter reads an enforced settings file from, for agent profiles' guards; default none). `models` and `efforts` are the picks a thread offers; `model` and `effort` the words that start its agent on one (`{}` is the pick; `NAME=value` goes in its environment, the rest after its command): Claude `ANTHROPIC_MODEL={}`, Codex `-c model_reasoning_effort={}`. A pick applies when the agent starts (shed can't change a running session's model yet). Setting it replaces the defaults. |
| `theme` | `farm` (when installed, else `terminal`) | The theme a device shows until it picks its own ([themes.md](themes.md)); installed theme packs and own schemes live in `themes/` and `schemes/` beside the config file, themes being built in `themes-dev/`. |
| `plugins` | none | `[[plugins]]` entries: `name`, `label`, `url`, `module`, `token` ([plugins.md](plugins.md)). |
| `plugin_index` | the built-in `plugins/index.toml` | The plugin index `cooee plugin add <name>` and Settings › Plugins list ([plugins.md](plugins.md)). |
| `[perf] slow_ms` | `250` | A request taking this many ms or more is a `warn` line in `<data>/perf.log` and on stderr (#53). |

`cooee serve` times every request (method, route pattern, status), every shed call (method, socket or CLI) and each
prompt's first reply chunk (thread, agent) into `<data>/perf.log`, one JSON line each, never a prompt or a token;
`cooee perf [--minutes 60]` prints p50/p95/max of each as a table.

## Agent profiles (#40)

An agent profile is a named recipe for a thread's agent, picked in the new-thread form and fixed for the thread's
life (a new thread changes it). **Plain**, the default, is the agent with the owner's own setup and nothing added. The
owner's profiles are `<config>/profiles/*.toml`, one per file; a plugin brings its own as `[[profiles]]` in its
`cooee-plugin.toml` ([plugins.md](plugins.md)), shown with the plugin's name, listed while it is installed and on.

```toml
name = "coder"                  # its id: lowercase letters, digits and - (a plugin's: <plugin>/<name>)
label = "Coder"                 # the picker and the thread's header
agent = "claude-acp"            # the ACP agent
account = "work"                # optional: its provider's account (else the default)
folder = "~/Projects/app"       # optional: the folder (a root or inside one), instead of the one picked
instructions = "coder.md"       # optional: sent ahead of the first message (the chat shows only the message)
settings = "settings.json"      # optional: an agent settings file, its hooks the profile's guard
env = { MODE = "review", API_KEY = "secret:API_KEY" }   # optional: plain values ({thread}: the thread's id), secrets as secret:<NAME>
policy = "ask"                  # optional: shed's approval policy, ask, allow or deny
mcp = "mcp.json"                # optional: refused for now (shed starts ACP sessions without MCP servers)
avatar = "https://x/me.png"     # optional: the sidebar's avatar, an image URL or a theme sprite id (none: initials)
```

Paths are relative to the profile's file (a plugin's: its checkout); `~/` is the home folder. A secret is never in a
profile: an env value that looks like a token is refused, and `secret:<NAME>` reads `<data>/secrets/<NAME>` at start.
At spawn the agent's adapter runs as `env <account folder> <settings variable>=<file> K=V… <adapter>`; secrets go
through a 0600 file that the adapter's shell reads and removes (`sh -c '. file; rm file; exec adapter'`), so no
secret is on a command line, in shed's state or in cooee's files or log. **Guarded profiles:** `settings` reaches the
agent only through its provider's `settings` variable, the one its ACP adapter reads an extra settings file from
and enforces, hooks included; an agent with none (Codex's and Gemini's) can't start the profile: "this profile's guard
can't run with <agent>". Claude Code's is `CLAUDE_CONFIG_DIR` (herdsman#565: claude-acp enforces the user settings
there), so a Claude profile's `settings` is a config folder (its `settings.json` holds the hooks), in place of the
account's. The form preselects each folder's last-used profile.

## The app shell (#21, herdsman#553 rows 1–14)

Offline, the Home Screen app opens on the service worker's last copy of a page and the chat page on its newest slice
(kept in `localStorage`), with "as of Xm ago" in the loading strip. A thread's sync dot is green while its event
stream is open and amber while it reconnects. Every action answers with a short toast. ↻, a pull down from the top, the
app coming back after 30 s or the network returning reload the page's data. Settings has an index and deep links
(`#settings/<section>`): sign-ins, look, alerts (truthful: on only when the server keeps this browser's subscription for
this device; `POST /push/test` alerts this device only, `POST /push/unsubscribe` turns it off), devices, and report
mode (`POST /report` to `<data>/reports.jsonl`, listed by `cooee reports`). Figures are `sprite`, `ascii` or `none` (#35); a theme
with moments plays them on the thread bar.

