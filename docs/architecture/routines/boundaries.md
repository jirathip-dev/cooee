# Module boundaries and statecharts (#75)

## Who calls whom

```text
 GitHub ──HTTPS──► Tailscale Funnel :8443 /hooks/github ──► 127.0.0.1:4748 (hooks listener: that one route)
                                                                   │
 agent ─MCP─► cooee mcp serve ─► POST /mcp/agent/routine ──┐        │
 phone ─────► /routines (Gate: paired device) ─────────────┤        │
                                                           ▼        ▼
                                cooee-server  routines.rs  (Runner: tick, deliver, catch_up, the routes)
                                   │  hub.send_from / hub.create_later / hub item for `.rt`
                                   │  lib.rs recipe()  (unchanged)
                                   │  curl (catch-up: GitHub REST, headers on stdin as the MCP proxy does)
                                   ▼
                                cooee-core  routines.rs, routines/cron.rs  (types, store, Template, Delivery, matching)
                                   │  secrets::read (GITHUB_WEBHOOK_SECRET, GITHUB_TOKEN)
                                   ▼
                                <data>/routines.json
 cooee-web: Settings › routines page, the `.rt` line, the ⟳ row mark (renders server data; no logic)
 cooee (binary): `[hooks]` config passed to serve; nothing else
```

| Module | May call | Never |
|---|---|---|
| `cooee-core::routines` | `secrets`, `store` types, `serde_json`, `hkdf::hmac`, `sha2` | sockets, processes, the clock (it is given `now` and the offset) |
| `cooee-server::routines` (`Runner`) | core `routines`, `Hub`'s public API, `recipe()`, `curl` (catch-up), `date +%z` (hourly) | `Live`, the thread store's files, shed directly |
| hooks listener | `Runner::deliver` only | the Gate, devices, any other route |
| `cooee-server::mcp` (`create_routine`) | `POST /mcp/agent/routine` with the thread's token | the store |
| `cooee-web` | — (renders `Routine` rows and items) | the server |

`scripts/dep-direction.sh` already enforces the crate direction; nothing new crosses it.

**The public edge is a separate listener, by design.** The main listener's Gate is unchanged: it gets no new bypass,
and `/hooks/*` is not routed there at all. Funnel points at `[hooks] bind`, which serves only `POST /hooks/github`;
a wrong Funnel path or a widened Funnel can reach nothing else. Funnel needs its own port (443's handlers become
public together), so the owner runs: `tailscale funnel --bg --https=8443 --set-path /hooks/github
http://127.0.0.1:4748/hooks/github`. `cooee doctor` prints that line when `[hooks]` is set and the port isn't
answering.

**Only GitHub.** q718 narrowed webhooks to one path; CI is GitHub Actions' events (`workflow_run`, `check_suite`).
Sentry and other senders are a later issue: a second constructor on `Delivery` and a second path.

## Statechart: a routine

```text
                 create_routine (valid; made_in a plain chat)
                              │  `.rt` line in made_in
                              ▼
   ┌──────────────────►  Active  ──── due time passes / delivery matches ───►  run(key) ──┐
   │  [resume]              │  ▲                                                          │
   │                        │  └──────────── Ran | Skipped(Rate) | Failed(other) ◄────────┘
   │                 [pause]│                                     │ Failed: chat closed or gone
   │                        ▼                                     ▼
   └────────────────  Paused(Owner)                     Paused(ChatGone)  (one push says so)
                            │                                     │
                            └──────── [delete] (confirm) ─────────┴──► gone (its threads stay)
```

- Paused: a due time passing still moves `last_due` (no burst on resume); a delivery is `Skipped(Paused)`.
- Edits (name, `when`, prompt) keep the state; a new `when` sets `last_due = now` so it doesn't fire at once.

## Statechart: a schedule's due time (AC 2a: once per due time, not again after a restart)

```text
 tick(now):  due = schedule.latest(now, offset)
             due ≤ max(last_due, made_at)  ──► nothing
             else                          ──► run(RunKey::Due(id, due))  ──► store.ran(id, due, outcome)
                                                     │ crash here: last_due unchanged
                                                     ▼
                                          restart: tick re-runs the same key; the hub adopts it (random_id)
```

Several missed due times (the Mac asleep) run once, for the newest; the run's `{{due}}` says which. The offset is the
Mac's (`date +%z`, read at start and hourly); a DST change moves later due times with the wall clock.

## Statechart: a delivery (AC 2b, 2c)

```text
 POST /hooks/github ─► secret unreadable ─► 503 (GitHub marks it failed; catch-up takes it later)
                    ─► Delivery::signed ─✗─► 413 | 400 | 401 (nothing stored)
                    ─✓─► guid in seen ─► 200 "seen"
                         event ping    ─► 200 "pong" (marked seen)
                         else          ─► mark seen, 202 {runs: n}; each matching routine:
                                          paused ─► Skipped(Paused); 6 runs this hour ─► Skipped(Rate);
                                          else run(RunKey::Delivery(id, guid)) on a worker (answer in < 1 s)

 catch_up (every 15 min) ─► per repo a routine names: GET hooks (the one whose config.url = [hooks] url)
                         ─► GET its deliveries (24 h) ─► each guid not in seen ─► GET it ─► Delivery::fetched ─► as above
```
