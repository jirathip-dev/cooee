# ADR: every routine gets the heartbeat's guards (#690)

## Summary

Decision: Every routine run now passes one guard, the heartbeat's own. A schedule routine runs at a due time only when you wrote in one of its persona's threads since its last run, when it was just made or edited, or when it has not run yet that day; a GitHub event or a signed POST always counts as new.
Every routine also stops at a runs-a-day cap: 24 until you set a default in Settings › routines, or the routine's own number. A skipped run costs no tokens and is counted (idle, capped) for Settings to show. The model is untouched: the chat's own.
Slices: A the shared guard, with the heartbeat moved onto it; B plain routines behind it (schedules, GitHub, signed POSTs) with the default and the per-routine cap; C Settings › routines (the screen, its two edits, the menu row).
Open questions: (1) A routine with nothing new still runs once a day, at its first due time, so a weekly or a morning routine in a quiet persona is not lost (my pick). Or strict: no run until you write?
(2) The cap before you set one: 24 a day, one an hour (my pick), or the heartbeat's 4?

Files: [`callers.md`](callers.md) (how the callers read, written first), [`shapes.md`](shapes.md) (the types and
files), [`boundaries.md`](boundaries.md) (who calls whom, the statecharts).

## Question

Where do "skip when nothing changed" and "at most N runs a day" sit so that every routine run (a schedule's due
time, a GitHub delivery, a signed POST) passes them, the heartbeat's watermark is reused and not copied, the owner
sets a default and a per-routine cap, and Settings can say what was skipped today? And what does "changed" mean for
a routine whose prompt is the owner's own text?

## What the code says

1. The heartbeat's guards are forty lines inside `Runner::heartbeat` (`cooee-server/src/routines.rs`): news after
   the watermark's `line`, then `runs` in the last 24 hours against `per_day`. Nothing else can call them.
2. `history::since` already has two rules: #498's "anything was said" and #219's "the owner wrote" (routine prompts
   never count in either). `Hub::news(persona, (line, false))` is the second, ready to call.
3. A plain routine's run state is `routine-runs.json`; the heartbeat's and consolidation's is `watermarks.json`.
4. An event's dedupe (`seen`) already means a repeat runs nothing; its only rate guard is six an hour.
5. A `chat` run is a message in the chat, so it runs on the chat's model; a `new_thread` run starts as `who`, the
   model the chat had. Only the built-ins go through `Cheapest`. Requirement 3 holds today; it needs a test, no code.
6. There is no Settings › routines screen today: the avatar menu has no such row, and routines show only on a
   persona's panel. Settings screens are data (`GET /screens/settings/*`); `POST /screens/action` serves persona
   screens only.
7. Size: `cooee-server/src/routines.rs` is 768 of 800 lines, `cooee-core/src/routines.rs` 759. Headroom 33,355 of
   36,000.

## Options

| | Choice | For | Against |
|---|---|---|---|
| Changed A | #498's rule: any new line in the persona's threads | The heartbeat's, word for word | A routine's reply is news to the next routine: two routines, or one and the heartbeat, wake each other all night |
| Changed **B** | **#219's rule: the owner wrote in one of the persona's threads** | No routine can wake another; `Hub::news` unchanged | A routine that looks outward (mail, a build) is not woken by the outside |
| Changed C | Also watch the folder or a URL | Sees the outside | New watermark kinds; blind on a remote machine; the issue says reuse |
| Changed D | A per-routine "always run" switch | Simple | An option that turns the guard off; the cap override already says "let this one run more" |
| Quiet days B1 | Strict: nothing written, no run, however long | The heartbeat's | "Mondays 09:00" in a persona you didn't write to that week never runs |
| Quiet days **B2** | **A new local day is a change: the first due time of a day runs** | Daily and weekly routines keep working; a 15-minute one costs one run on a silent day, not 96 | One run a day that strict would save |
| State A | In `routine-runs.json` beside `last_due` | One write a tick; gone with the routine | A second home for a line and a count; the heartbeat could not share the guard |
| State **B** | **`watermarks.json`, key `routine-<id>`** | The issue's "reuse the watermark"; one guard type for both | Two files a run (ordered so a crash neither loses nor repeats a run) |
| Day A | Any 24 hours (the heartbeat today) | No behaviour change | "The rest of the day" and "skipped today" need a time per skip |
| Day **B** | **The Mac's local calendar day** | The issue's words; three counters | The heartbeat's `per_day` becomes "a local day" |
| Default's home | `config.toml` / **`routines.toml` top-level `per_day`** | The server already writes `routines.toml`, comments kept | — |
| Who sets a cap | Agents too / **the owner only, in Settings** | A cap is a spend limit | An agent can't make its own routine run more |

## Decision

**Changed B with B2, State B, Day B, the default in `routines.toml`, the owner sets caps.**

1. **One guard.** `cooee-core/src/routines/guard.rs` holds `Cap`, `Change`, `Held`, `Tally` and three methods on
   `Watermark`: `check` (idle before capped, counts nothing), `ran`, `held`. The heartbeat and plain routines both
   call them. The heartbeat's inline copy is deleted.
2. **The rule for a schedule routine** (the PR body states it): a due time runs when the routine is fresh (not run
   since it was made or its prompt or schedule edited), or the owner wrote in a thread of its persona after its last
   run, or it has not run yet this local day. Else it is skipped as idle. "Its persona" is the chat's agent profile
   (`chat`) or `who.profile` (`new_thread`); with none, the Plain chats. Routine prompts and replies never count.
3. **Events are never idle.** A delivery or a signed POST is the new input; a repeat is already `seen`. The cap
   applies, after pause and the hourly rate.
4. **The cap** is runs started in a local day: the routine's own `per_day`, else the default, else 24. 1 to 144.
   Past it a due time or an event is skipped as capped; what the owner wrote waits for the next run.
5. **Counting.** A schedule checks, runs, then counts, so a crash re-runs into the same key and never loses or
   doubles a run. An event checks and counts in one step, so a burst can't pass the cap. A failed start of a
   schedule run is not counted.
6. **State** is `watermarks.json` under `routine-<id>`: `line` (the last run's time) and `today` (day, ran, idle,
   capped). A held run never touches `last_run`. An edit of prompt or schedule resets the line; a delete drops the key.
7. **Settings.** `per_day` at the top of `routines.toml` is the default, `per_day` in a `[[routine]]` its own. A new
   screen, `GET /screens/settings/routines`, lists the default and each routine with its cap and today's tally; its
   two actions set the default and a routine's cap. `create_routine` and Chief's `routine` tool can't set one.
8. **The model.** No change and no `Cheapest` for plain routines; one test pins it.
9. **The built-ins.** The heartbeat keeps its own source rule (#498) and its own `per_day` (default 4), now counted
   per local day by the shared guard. Consolidation keeps #219's skip and gets no cap: it is once per due time on
   the cheapest model.

## Consequences

- A fast open-ended routine in a silent persona costs one run a day, not one per due time. A daily or weekly one
  runs as before.
- A routine that polls the outside on a schedule ("is the build green yet?") runs only while you are writing to its
  persona, plus once a day. The outside's own way in is a signed POST or a GitHub event. `create_routine`'s
  description gains one sentence so agents don't promise otherwise.
- On upgrade, each routine is fresh: its next due time runs. The heartbeat's count starts over once, and its limit
  reads "runs a local day" where it read "in any 24 hours".
- A capped event is dropped, not kept for tomorrow (the same as today's hourly rate). Raise that routine's cap.
- No push and no chat line for a skip: Settings is where it shows.
- Size: about `src/ net +350` (core guard 90, core fields 25, server guard 110, the screen 110, wiring 15; the
  heartbeat shrinks). No Swift is expected: the screen is data, the menu row is JSON.
- The design lane, next, draws Settings › routines from [`shapes.md`](shapes.md) §4; the data and the two actions
  are fixed here, the look is theirs.

## Hot files

| File | PRs in 14 days | What this design does |
|---|---|---|
| `cooee-server/src/routines.rs` | 17 | Shrinks: `consolidate`, `heartbeat`, `as_persona`, `beat` move to `routines/builtin.rs`; the guard is `routines/guard.rs`; `tick`, `deliver`, `accept`, `edit`, `delete` gain one call each |
| `cooee-server/src/screens/persona.rs` | 14 | Three lines: `action` hands other screens on |
| `cooee-core/src/history.rs` | 11 | One field (`runs` becomes `today`); the methods live in `routines/guard.rs` |
| `cooee-core/src/routines.rs` | 9 | The `per_day` fields and `mod guard;` |
| `cooee/tests/routines.rs` | 8 | Untouched: the guards are their own area, `tests/routine_guards.rs` |
| `cooee-server/src/screens/mod.rs` | 7 | Two lines. It is a hand-kept route list; generating it like `routes/mod.rs` is its own PR, not this one |
| `docs/routines.md` | 6 | One sentence (the heartbeat's day) and a link; the guards' text is a new `docs/routine-guards.md` |

No file is converted here, so nothing is frozen or generated. One ban keeps the boundary.

## Slices

- **A**: core `routines/guard.rs`; `Watermark.today`; server `routines/builtin.rs` with the heartbeat on the guard.
- **B**: `per_day` in `Routine`, `Book`, `Edit` and `routines.toml`; server `routines/guard.rs`; `tick`, `deliver`,
  `accept`, `edit`, `delete`; the tool description; `tests/routine_guards.rs`; `docs/routine-guards.md`.
- **C**: `screens/routines.rs`, the action hand-off, the menu row and its value, after the design is approved.

## Open questions for the owner

1. **Once a day when nothing changed.** Default: yes, the first due time of a local day runs. Strict (the
   heartbeat's) means a weekly or morning routine in a persona you didn't write to is skipped until you do.
2. **The first default cap.** Default: 24 a day, so an hourly routine you already have is not capped on upgrade.
   The heartbeat's own number is 4.

## `.drover/arch.toml` (the impl lane adds this in the PR)

```toml
[[ban]]
pattern = "history::watermark"
paths = ["crates/cooee-server/src/**"]
except = ["crates/cooee-server/src/routines/guard.rs", "crates/cooee-server/src/routines/builtin.rs", "crates/cooee-server/src/hub/heartbeat.rs", "crates/cooee-server/src/hub/mcp.rs"]
reason = "#690: a routine's line, cap and tally change only through the guard: use Runner::admit_due, admit_event, count"
```
