# Routine guards: module boundaries and statecharts (#690)

## Who calls whom

```text
 phone ─► GET /screens/settings/routines, POST /screens/action ─► screens/routines.rs (new) ─┐
 phone ─► POST /routines/<id>  (per_day) ─► routes/routines.rs ───────────────────────────────┤
                                                                                              ▼
 cooee-server routines.rs        tick · deliver · accept · edit · delete     (asks the guard; never opens the watermark)
        │    routines/script.rs  script: its wake arm only                   (the same)
        │    routines/webhook.rs inbound · hear        routines/end.rs stop  (never ask the guard)
        │                              │
        │                              ▼
        │            routines/guard.rs (new)    admit_due · admit_event · admit_wake · count · fresh · forget
        │                              │        guards · set_default · cap
        │                              │  Hub::news (#219's, unchanged)   history::watermark
        ▼                              ▼
 routines/builtin.rs (moved)     cooee-core routines/guard.rs (new)   Cap · Turns · Change · Held · Tally · Watermark::check/ran/held
   consolidate · heartbeat · beat          │
        └── the same core guard ───────────┤
                                           ▼
                                 <data>/watermarks.json      <config>/routines.toml (per_day, through Routines::save)
```

| Module | May call | Never |
|---|---|---|
| core `routines::guard` | `history::Watermark`'s fields, serde | the clock, files, the hub: it is given `now`, the day and the change |
| core `routines` | `guard::Cap` for the two `per_day` keys, `guard::Turns` for `Routine::turns` | `watermarks.json` |
| server `routines/guard.rs` | core guard, `history::watermark`, `Hub::news`, `Hub::thread`, `Runner::offset`, `Runner::change`, the file's default cap | starting a run; `last_run`; `heard`, `silent`, `ends_at`, `paused` |
| server `routines.rs` | `Runner::admit_due`, `admit_event`, `count`, and on an edit or delete `fresh`, `forget` | `history::watermark` (banned, see the ADR's last section) |
| server `routines/script.rs` | `Runner::admit_wake` before `Runner::prompt`, in its wake arm | the guard on a `post` run or before the script runs |
| server `routines/webhook.rs`, `routines/end.rs` | as today | the guard: `hear`, the silence alert, `stop` and an end time tally nothing |
| server `routines/builtin.rs` | core guard inside its own watermark closure, `as_persona` | plain routines' keys |
| server `screens/routines.rs` | `routines::on(cx, …)` with `guards`, `set_default`, `edit` | core files, the watermark |
| `screens/persona.rs` `action` | hands any screen that is not a persona's to `screens/routines.rs` | — |
| `mcp/own.rs`, `chief/tools/routine.rs` | as today; `create_routine`'s description gains one sentence on the guards | setting `per_day` |

Lock order: the runner's book lock, then the watermark file's. Nothing takes them the other way round.
A watermark that can't be read holds the turn and notes why: an agent turn never starts unguarded.

## Statechart: a schedule's due time

```text
 tick: end time passed ─► Ended (a pause)                       due > max(last_due, made_at) ?
   paused or ended ──────────────────────────────────────────────────────────► last_due = due   (no guard, no tally)
   a script ─► it runs ─► post: its output posts (no guard, no tally) ───────► last_due = due
                       ─► wake: exit 0 quiet; else "a wake" below ───────────► last_due = due
   a prompt: change?
             Fresh    not run since made, edited (prompt or schedule) or resumed (line = 0)
             Said     the owner wrote in a thread of its persona after its last run
             NewDay   no run yet this local day
     none ─────────────► Held(Idle):   idle + 1 ─────────────────────────────► last_due = due
   ran today ≥ cap ────► Held(Capped): capped + 1, the line stays ───────────► last_due = due
   else run(key) ─► Ran:    ran + 1, line = now ─► record ───────────────────► last_due = due
                 ─► Failed: nothing counted, the line stays ─► record (pauses on a gone chat, as today)
```

"Its persona": the chat's agent profile for a `chat` routine, `who.profile` for a `new_thread` one; with none
(Plain), the Plain chats. Routine prompts and replies, script output and silence notes never count, so two routines
can't wake each other. An edit of the prompt or the schedule, or a resume, sets the line to 0 (`Fresh`) and keeps
the day's tally; a rename, a pause, an end or a new cap changes nothing in the watermark.

## Statechart: a routine's day

```text
   new local day (the tally starts at zero)
        │
        ▼
   Open (ran < cap) ── due, event or wake, changed: an agent turn, ran + 1 ──► Open, or Capped once ran = cap
        │  ▲   └────── due, nothing changed: idle + 1, no run ──► Open
        │  │
        │  └── the owner raises the cap in Settings, or a new local day
        ▼
   Capped ── due, event or wake: capped + 1, no agent turn; what the owner wrote waits for the next run

   Paused or ended, from either: nothing is checked or tallied. Resumed: back to Open or Capped by the same
   day's count, and fresh. A `post` script has no such day: it always runs.
```

## Statechart: an event (GitHub delivery, signed POST, the Test button)

```text
 signed POST ─► hear: the silence clock moves (always, before anything below)
             ─► an alive POST (#697) ─► 2xx, nothing else: no hourly slot, no tally
 seen before ─► 200, nothing            paused or ended ─► Skipped(Paused)           6 in the hour ─► Skipped(Rate)
 a script ─► 202 ─► the worker runs it ─► post: its output posts ─► wake: exit 0 quiet; else "a wake" below
 a prompt: ran today ≥ cap ─► capped + 1, Skipped(Capped), 202 `runs: 0`
           else ran + 1 at once ─► 202 ─► the worker runs it
```

## Statechart: a wake (a `wake` script exited non-zero, at a due time or on an event)

```text
 ran today ≥ cap ─► capped + 1, Skipped(Capped), no agent turn
 else ran + 1 at once, line = now ─► Runner::prompt: the script's output is the agent's prompt
```
