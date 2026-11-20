# Routine guards (#690)

## Summary

Decision: Every routine run now passes one guard, the heartbeat's own. A schedule routine runs at a due time only when you wrote in one of its persona's threads since its last run, when it was just made or edited, or when it has not run yet that day; a GitHub event or a signed POST always counts as new.
Every routine also stops at a runs-a-day cap: 24 until you set a default in Settings › routines, or the routine's own number. A skipped run costs no tokens and is counted (idle, capped) for Settings to show. The model is untouched: the chat's own.
Slices: A the shared guard, with the heartbeat moved onto it; B plain routines behind it (schedules, GitHub, signed POSTs) with the default and the per-routine cap; C Settings › routines (the screen, its two edits, the menu row).
Open questions: (1) A routine with nothing new still runs once a day, at its first due time, so a weekly or a morning routine in a quiet persona is not lost (my pick). Or strict: no run until you write?
(2) The cap before you set one: 24 a day, one an hour (my pick), or the heartbeat's 4?

## Read in this order

1. [`adr.md`](adr.md): the question, what the code says, the options, the nine decisions, the consequences, the hot
   files, the slices, the `.drover/arch.toml` lines.
2. [`callers.md`](callers.md): what the owner sees, the tick, an event, the guard, the heartbeat on it, Settings'
   two edits, the tests by AC line. Written first.
3. [`shapes.md`](shapes.md): the guard's types, what changes in `Watermark`, `Routine`, `Book`, `Edit` and
   `Heartbeat`, the two files, Settings › routines on the wire.
4. [`boundaries.md`](boundaries.md): which module may call which, and three statecharts.

## Where the issue's points are answered

| The issue asks | ADR decision |
|---|---|
| Skip when nothing changed, on every routine, reusing #498's and #219's watermark | 1, 2, 3, 6 |
| What "changed" means for a plain prompt routine | 2 (open question 1) |
| A runs-a-day cap, a default in Settings, a per-routine override | 4, 7 (open question 2) |
| A capped routine skips the rest of the day; Settings shows the cap and today's skips | 4, 6, 7 |
| A run uses the chat's own model | 8 |
