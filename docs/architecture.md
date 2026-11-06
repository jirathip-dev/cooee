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
| `cooee-web` | `web/` | the web UI in Topcoat (ADR 0001): pages, `chat.js`, `app.css`, the PWA manifest, the themes and schemes (`look.rs`, `look.js`, `themes/`, `schemes/`) | not the server or binary |
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
by id, and a torn last line from a crash is skipped. A pending approval is stored before the page hears of it.

## Chat (M1)

One thread is one ACP session on [shed](https://github.com/jirathip-dev/shed): `shed run --acp <agent> --name
cooee-<id> --label cooee=1 --label cooee.thread=<id> --cwd <folder>` (the CLI, as are `shed agents`; the socket has
no method for them). Everything else goes over shed's socket, one connection per request, each opening with the exact
`hello` of the pinned `shed-proto` (another version is refused with both versions named). Prompts, approvals and
cancels are framed `send`s, as `shed send|approve|cancel` do. One `events` stream (label `cooee=1`) feeds every thread;
after a restart cooee re-attaches each open thread to its terminal by name and replays from the last event it applied.

Each shed event maps to items through one exhaustive `match` (`cooee-core/src/chat.rs`). The page gets them over SSE
(`/threads/<id>/events`), each with id `<run>.<seq>`; a thread keeps its last 4,096 page events, so a page that
reconnects with `Last-Event-ID` gets exactly what it missed, and one from another run or too far behind gets `reset`
and reloads. Its first slice (`/threads/<id>/messages?around=&before=&after=`) carries the cursor it is current to,
taken under the thread's lock with the subscription, so nothing is lost or doubled.

## Pairing and push (M2)

Everything but `/pair` and the static files (manifest, icons, style, script, `sw.js`) needs a paired device: an
unpaired page load goes to `/pair`, any other read or write gets 401. `cooee pair` prints a six-digit code (and its
`/pair?code=` link as a QR code), good for 10 minutes and once; five wrong tries drop it. The browser that enters it
gets a random token as an `HttpOnly`, `SameSite=Strict` cookie. `<data>/devices.json` (0600) keeps only SHA-256
hashes. `cooee devices` lists the devices and `cooee devices rm <id>` unpairs one (its pushes stop too). On iPhone,
add cooee to the Home Screen first and pair there: the app has its own cookies.

Before that, every request's `Host` (and its `Origin`, when sent) must be localhost, the `bind` address or one of
`allowed_hosts`, or it gets 403, paired cookie or not. That blocks DNS rebinding. To reach cooee from the phone,
keep `bind` on 127.0.0.1, run `tailscale serve --bg 4747`, and put the machine's Tailscale name in `allowed_hosts`.

Web push (RFC 8291 aes128gcm, VAPID per RFC 8292, sent with `curl`): the VAPID key is made once and kept in the
macOS keychain (`security`, service `cooee`), else in `<data>/vapid.key` (0600). The thread list's 🔔 subscribes;
subscriptions sit in `<data>/push.json` (0600), each tied to its device. A push goes out when an agent asks for an
approval, a turn finishes while no page has the thread open, or an agent errors. Each one is a line about the thread:
`{title, body, tag: <thread>, url: /t/<thread>, item}`. The service worker (push only, no cache) shows it under the
thread's tag, so a newer push replaces the older one. It shows nothing while that thread is open in view. A tap opens
the thread, and reading the thread past `item` closes it.

## Sign-ins (M3)

Each provider is one row of the `providers` table in the config: its name, the ACP agent it signs in, its CLI's own
login command and a read-only status command. There is no provider-specific code beyond that table. The status
command signs a provider in when it exits 0; its output, on one line and redacted, is the status line. Settings'
sign-ins row opens the sign-ins card on the thread list: each provider's state, and `[sign in]` for a signed-out one. The new-thread
picker marks each agent `✓` or `signed out`, with a link to the sheet.

`[sign in]` runs the login command in a plain shed terminal (terminal mode, not ACP; over the socket: `run`, then
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

## Configuration

One TOML file, `$XDG_CONFIG_HOME/cooee/config.toml` (else `~/.config/cooee/config.toml`, or `cooee serve --config
<file>`). Every key is optional; an unknown key is an error that lists the known ones.

| Key | Default | Meaning |
|---|---|---|
| `port` | `4747` | The port `cooee serve` listens on (`0` picks a free one). |
| `bind` | `127.0.0.1` | The address it listens on. Anything but loopback needs `allowed_hosts`. |
| `allowed_hosts` | none | The names requests may use besides localhost and `bind`: the Tailscale name. Matched exactly, so `*` widens nothing. |
| `roots` | none | The folders a thread may work in: each root and the folders right inside it, nothing outside. |
| `shed` | `shed` | The `shed` binary. |
| `shed_dir` | shed's per-user server | shed's `--dir` (its socket `shed.sock` and state). |
| `data` | `$XDG_DATA_HOME/cooee`, else `~/.local/share/cooee` | Where threads are stored. |
| `providers` | Claude Code, Codex, Gemini | The sign-ins: `[[providers]]` with `name`, `agent` (its ACP agent id), `login` and `status` (commands, as arrays). Setting it replaces the defaults. |
| `theme` | `farm` | The theme a device shows until it picks its own ([themes.md](themes.md)); own themes and schemes live in `themes/` and `schemes/` beside the config file. |
| `plugins` | none | `[[plugins]]` entries: `name`, `label`, `url`, `module`, `token` ([plugins.md](plugins.md)). |
