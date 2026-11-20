# Routine guards (#690)

## Summary

Decision: Every agent turn a routine starts passes one guard, the heartbeat's own. A schedule's prompt runs at a due time only when you wrote in one of its persona's threads since its last run, when it was just made, edited or resumed, or when it has not run yet that day; a GitHub event, a signed POST or a wake script's failing exit always counts as new.
Each routine also stops at a runs-a-day cap: 24 until you set a default in Settings › routines, or the routine's own number. A skipped turn costs no tokens and is counted (idle, capped) for Settings to show. The model is untouched: the chat's own.
Only agent turns are checked and counted. A script's own run costs no tokens: a post-only script is never skipped or capped, a wake script only when it would wake the agent. A paused or ended routine, a silence alert and an alive POST (#697) run no agent and count for nothing.
Slices: A the shared guard, with the heartbeat moved onto it; B plain routines behind it (schedules, events, wake scripts) with the default and the per-routine cap; C Settings › routines (the screen, its two edits, the menu row).
Open questions: none. You kept both picks (q2488): one run on a quiet day, and a default cap of 24.

## Read in this order

1. [`adr.md`](adr.md): the question, what the code says, the options, the twelve decisions, the consequences, the
   hot files, the slices, the `.drover/arch.toml` lines.
2. [`callers.md`](callers.md): what the owner sees, the tick, an event, a wake script, the guard, the heartbeat on
   it, Settings' two edits, the tests by AC line. Written first.
3. [`shapes.md`](shapes.md): the guard's types, what changes in `Watermark`, `Routine`, `Book`, `Edit`, `Skip` and
   `Heartbeat`, the two files, Settings › routines on the wire.
4. [`boundaries.md`](boundaries.md): which module may call which, and four statecharts.

## Where the issue's points are answered

| The issue asks | ADR decision |
|---|---|
| Skip when nothing changed, on every routine, reusing #498's and #219's watermark | 1, 2, 3, 7 |
| What "changed" means for a plain prompt routine | 2 |
| A runs-a-day cap, a default in Settings, a per-routine override | 4, 10 |
| A capped routine skips the rest of the day; Settings shows the cap and today's skips | 4, 7, 10 |
| A run uses the chat's own model | 11 |

## Where the revise note (q2488) is answered

| The owner asks | ADR decision |
|---|---|
| Rebase on staging `4d1935d` (#692, #689, #691) | "What the code says" 3, 5, 6, 7, 10 |
| Script routines: only a wake that prompts the agent is checked and counted | 1, 5, 6 |
| Ended routines and the guard's state | 8 |
| The silence clock (`heard`) and the guard | 9 |
| An alive POST (#697) runs nothing and counts for nothing | 9, and the order in Consequences |
| The hot-file sizes | "Hot files" |
| Keep both picks | 2, 4 |
