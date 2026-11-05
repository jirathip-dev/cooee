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
| `cooee-core` | `crates/cooee-core` | config, threads, the store, the shed client (M1); no sockets or HTTP | no workspace crate |
| `cooee-web` | `web/` | the web UI in Topcoat (ADR 0001): pages, the PWA manifest | not the server or binary |
| `cooee-server` | `crates/cooee-server` | HTTP on 127.0.0.1, live events, pairing, push | core, web |
| `cooee` | `crates/cooee` | the binary: arguments and wiring only | all |

`scripts/dep-direction.sh` enforces the last column in CI.

## Storage

One typed JSON file per thread, `<data>/threads/<id>.json`: `{"format": 1, "thread": {…}}`. Every write is
write-temp, fsync, rename, then fsync of the folder, under that thread's lock, so a crash leaves the old file or the new
one, never half of either. A file from a newer format is refused, not guessed at. `crates/cooee-core/tests/upgrade_compat.rs`
loads every earlier release's files (`tests/fixtures/upgrade/<version>/`) and fails if any field would be lost.

## Configuration

One TOML file, `$XDG_CONFIG_HOME/cooee/config.toml` (else `~/.config/cooee/config.toml`, or `cooee serve --config
<file>`). Every key is optional; an unknown key is an error that lists the known ones.

| Key | Default | Meaning |
|---|---|---|
| `port` | `4747` | The port `cooee serve` listens on, on 127.0.0.1 (`0` picks a free one). |
