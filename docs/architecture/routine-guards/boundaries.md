# Routine guards: module boundaries and statecharts (#690)

## Who calls whom

```text
 phone ─► GET /screens/settings/routines, POST /screens/action ─► screens/routines.rs (new) ─┐
 phone ─► POST /routines/<id>  (per_day) ─► routes/routines.rs ───────────────────────────────┤
                                                                                              ▼
 cooee-server routines.rs        tick · deliver · accept · edit · delete     (asks the guard; never opens the watermark)
        │                              │
        │                              ▼
        │            routines/guard.rs (new)    admit_due · admit_event · count · guards · set_default
        │                              │  Hub::news (#219's, unchanged)   history::watermark
        ▼                              ▼
 routines/builtin.rs (moved)     cooee-core routines/guard.rs (new)   Cap · Change · Held · Tally · Watermark::check/ran/held
   consolidate · heartbeat · beat          │
        └── the same core guard ───────────┤
                                           ▼
                                 <data>/watermarks.json      <config>/routines.toml (per_day, through Routines::save)
```

| Module | May call | Never |
|---|---|---|
| core `routines::guard` | `history::Watermark`'s fields, serde | the clock, files, the hub: it is given `now`, the day and the change |
| core `routines` | `guard::Cap` for the two `per_day` keys | `watermarks.json` |
| server `routines/guard.rs` | core guard, `history::watermark`, `Hub::news`, `Hub::thread`, `Runner::offset`, `Runner::change` | starting a run; `last_run` |
| server `routines.rs` | `Runner::admit_due`, `admit_event`, `count`, and on an edit or delete `fresh`, `forget` | `history::watermark` (banned, see the ADR's last section) |
| server `routines/builtin.rs` | core guard inside its own watermark closure, `as_persona` | plain routines' keys |
| server `screens/routines.rs` | `routines::on(cx, …)` with `guards`, `set_default`, `edit` | core files, the watermark |
| `screens/persona.rs` `action` | hands any screen that is not a persona's to `screens/routines.rs` | — |
| `mcp/own.rs`, `chief/tools/routine.rs` | as today; `create_routine`'s description gains one sentence on the guards | setting `per_day` |

Lock order: the runner's book lock, then the watermark file's. Nothing takes them the other way round.
A watermark that can't be read holds the run and notes why: a routine never runs unguarded.

## Statechart: a schedule's due time

```text
 tick: due > max(last_due, made_at) ?
   paused ───────────────────────────────────────────────────────────────────► last_due = due
   change?   Fresh    not run since made or edited (line = 0)
             Said     the owner wrote in a thread of its persona after its last run
             NewDay   no run yet this local day                    (open question 1)
     none ─────────────► Held(Idle):   idle + 1 ─────────────────────────────► last_due = due
   ran today ≥ cap ────► Held(Capped): capped + 1, the line stays ───────────► last_due = due
   else run(key) ─► Ran:    ran + 1, line = now ─► record ───────────────────► last_due = due
                 ─► Failed: nothing counted, the line stays ─► record (pauses on a gone chat, as today)
```

"Its persona": the chat's agent profile for a `chat` routine, `who.profile` for a `new_thread` one; with none
(Plain), the Plain chats. Routine prompts and replies never count, so two routines can't wake each other.
An edit of the prompt or the schedule sets the line to 0 (`Fresh`); a rename, a pause or a new cap doesn't.

## Statechart: a routine's day

```text
   new local day (the tally starts at zero)
        │
        ▼
   Open (ran < cap) ── due or event, changed: run, ran + 1 ──► Open, or Capped once ran = cap
        │  ▲   └────── due, nothing changed: idle + 1, no run ──► Open
        │  │
        │  └── the owner raises the cap in Settings, or a new local day
        ▼
   Capped ── due or event: capped + 1, no run; what the owner wrote waits for the next run
```

## Statechart: an event (GitHub delivery, signed POST, the Test button)

```text
 seen before ─► 200, nothing            paused ─► Skipped(Paused)           6 in the hour ─► Skipped(Rate)
 ran today ≥ cap ─► capped + 1, counted in `runs: 0`, last_run untouched
 else ran + 1 at once ─► 202 ─► the worker runs it
```
