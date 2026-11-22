# Architecture

cooee is one binary on the owner's machine (Mac or Linux). It serves the native app's JSON API (`apple/`, iOS and macOS;
the client since #189) on 127.0.0.1, and Tailscale carries it to the owner's phone. Agents run under [shed](https://github.com/jirathip-dev/shed), which
keeps them going while the phone is away; cooee talks to shed and pings the phone when an agent needs the owner.

```
  phone (native app)                         owner's machine
 ┌──────────────────────────┐   Tailscale   ┌──────────────────────────────────────────────────────────┐
 │ thread list, chat, login │◄─────────────►│ cooee (one binary)                                       │
 │ APNs push ◄──────────────┼───────────────┤  ├─ cooee         args + wiring                          │
 └──────────────────────────┘    (M2)       │  ├─ cooee-server  HTTP on 127.0.0.1, live events,        │
                                            │  │                pairing, push (Tokio, via Topcoat)     │
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
| `cooee-machines` | `crates/cooee-machines` | the first-party plugin `machines` ([plugins/](plugins/README.md)): machines' health from shed, its backend and tab module | core |
| `cooee-terminal` | `crates/cooee-terminal` | the first-party plugin `terminal` ([plugins/](plugins/README.md)): the owner's shells through shed, its backend and tab module | core |
| `cooee-server` | `crates/cooee-server` | HTTP, the threads' JSON API and SSE (M1), the Host/pairing gate, push (M2), logins (M3), `GET /machines` (the machines plugin's data, #148) | core, machines |
| `cooee` | `crates/cooee` | the binary: arguments and wiring only | all |

core/arch's `[[layer]]` entries (`.drover/arch/gatekeeper.toml`) enforce the last column in CI.

Each area has its own doc beside this one, `docs/<area>.md`: what it does today, how it works and its API.

## API conventions

Everything the native app (`apple/Sources/CooeeAPI`) and the plugins call; cooee serves no pages since #189. Every
call needs a paired device's `cooee` cookie (pairing excepted), and a
request must name one of the server's own hosts (`Host`, and `Origin` when sent).

A failure is a status ≥ 400. When the body is cooee's own sentence, the answer carries `Cooee-Error: 1` and a
`text/plain` body to show as it is; any other failure is worded by the app. `Item` is one row of a thread:

```ts
type Item = { id: number; at: number; seq?: number } & (
  | { kind: "you"; text: string; random_id: string; files?: string[] }
  | { kind: "agent"; text: string; stop_reason: string }
  | { kind: "tool"; call: string; title: string; status: string; ended?: number }
  | { kind: "approval"; tool: string; options: { id: string; name: string; kind: string }[]; tapped?: string; chosen?: string; plugin?: string;
      by?: string; answered?: number; form?: "config" | "secret" | "thread"; detail?: string; line?: string; until?: number;
      block: Card }  // #138: the card block the native app draws (blocks' schema/blocks.schema.json, #621)
  | { kind: "link"; text: string; thread: string; title: string; out?: boolean }
  | { kind: "ask"; persona: string; name: string; text: string; thread: string;  // #177: an ask_persona card
      outcome: "asking" | "replied" | "not_allowed" | "depth" | "timeout" | "offline"; secs: number; note?: string }
  | { kind: "handoff"; text: string; outcome: "waiting" | "handed_back"; note?: string }  // #164: a handoff ask
  | { kind: "plan"; entries: { content: string; status: string; priority?: string }[] }
  | { kind: "note"; text: string }
  | { kind: "ended"; text: string }
);
type Turn = { text: string; state: "working" | "blocked" | "done" | "error" | "ended" | ""; note?: string };
type Queued = { text: string; random_id: string; files?: string[] };
type Row = { id; title; agent; folder; state; last; at; unread: boolean; unseen: number; archived: boolean };
type File = { name: string; data: string }; // data: the bytes in base64
```
