# ADR: every routine gets the heartbeat's guards (#690)

## Summary

Decision: Every agent turn a routine starts passes one guard, the heartbeat's own. A schedule's prompt runs at a due time only when you wrote in one of its persona's threads since its last run, when it was just made, edited or resumed, or when it has not run yet that day; a GitHub event, a signed POST or a wake script's failing exit always counts as new.
Each routine also stops at a runs-a-day cap: 24 until you set a default in Settings › routines, or the routine's own number. A skipped turn costs no tokens and is counted (idle, capped) for Settings to show. The model is untouched: the chat's own.
Only agent turns are checked and counted. A script's own run costs no tokens: a post-only script is never skipped or capped, a wake script only when it would wake the agent. A paused or ended routine, a silence alert and an alive POST (#697) run no agent and count for nothing.
Slices: A the shared guard, with the heartbeat moved onto it; B plain routines behind it (schedules, events, wake scripts) with the default and the per-routine cap; C Settings › routines (the screen, its two edits, the menu row).
Open questions: none. You kept both picks (q2488): one run on a quiet day, and a default cap of 24.

Files: [`callers.md`](callers.md) (how the callers read, written first), [`shapes.md`](shapes.md) (the types and
files), [`boundaries.md`](boundaries.md) (who calls whom, the statecharts). Read on staging `4d1935d`, with #692
(silence), #689 (end time) and #691 (script routines) merged.

## Question

Where do "skip when nothing changed" and "at most N runs a day" sit so that every agent turn a routine starts (a
schedule's due time, a GitHub delivery, a signed POST, a wake script's failing exit) passes them, a run that costs
no tokens does not, the heartbeat's watermark is reused and not copied, the owner sets a default and a per-routine
cap, and Settings can say what was skipped today? And what does "changed" mean for a routine whose prompt is the
owner's own text?

## What the code says

1. The heartbeat's guards are forty lines inside `Runner::heartbeat` (`cooee-server/src/routines.rs`): news after
   the watermark's `line`, then `runs` in the last 24 hours against `per_day`. Nothing else can call them.
2. `history::since` already has two rules: #498's "anything was said" and #219's "the owner wrote" (routine prompts
   never count in either). `Hub::news(persona, (line, false))` is the second, ready to call.
3. A plain routine's run state is `routine-runs.json` (with #689's `ended` and #692's `heard` and `silent`); the
   heartbeat's and consolidation's is `watermarks.json`.
4. An event's dedupe (`seen`) already means a repeat runs nothing; its only rate guard is six an hour.
5. One function starts a plain routine's agent turn: `Runner::prompt` (#691). A prompt routine's run always reaches
   it. A script routine runs `sh` first: `post` never reaches it, `wake` only on a non-zero exit (exit 0 is
   `skipped: quiet`).
6. Ended is a kind of pause (`Paused::Ended`, #689). `tick` ends a routine before it looks at its due time, and
   `Routine::admit` before an event; a paused due time only moves `last_due`.
7. The silence clock (#692) is fed by `Runner::hear` in `inbound`, before `accept`; its alert and its "back" line
   are a push and a chat note, never a run. Today every signed POST then runs the routine (#697 is that bug).
8. A `chat` run is a message in the chat, so it runs on the chat's model; a `new_thread` run starts as `who`, the
   model the chat had. Only the built-ins go through `Cheapest`. Requirement 3 holds today; it needs a test, no code.
9. There is no Settings › routines screen today: the avatar menu has no such row, and routines show only on a
   persona's panel. Settings screens are data (`GET /screens/settings/*`); `POST /screens/action` serves persona
   screens only.
10. Size: `cooee-server/src/routines.rs` is 791 of 800 lines, so nothing can be added before something leaves;
    `cooee-core/src/routines.rs` is 594. The repo is at 33,762 of 36,000.

## Options

| | Choice | For | Against |
|---|---|---|---|
| Changed A | #498's rule: any new line in the persona's threads | The heartbeat's, word for word | A routine's reply is news to the next routine: two routines, or one and the heartbeat, wake each other all night |
| Changed **B** | **#219's rule: the owner wrote in one of the persona's threads** | No routine can wake another; `Hub::news` unchanged | A prompt routine that looks outward is not woken by the outside (a wake script or a webhook is) |
| Changed C | Also watch the folder or a URL | Sees the outside | New watermark kinds; blind on a remote machine; the issue says reuse |
| Changed D | A per-routine "always run" switch | Simple | An option that turns the guard off; the cap override already says "let this one run more" |
| Quiet days B1 | Strict: nothing written, no run, however long | The heartbeat's | "Mondays 09:00" in a persona you didn't write to that week never runs |
| Quiet days **B2** | **A new local day is a change: the first due time of a day runs** (the owner's pick, q2488) | Daily and weekly routines keep working; a 15-minute one costs one run on a silent day, not 96 | One run a day that strict would save |
| Scripts A | Guard every run, a script's too | One rule for all | A script run costs no tokens; a capped or idle monitor goes blind for nothing |
| Scripts **B** | **Guard the agent turn: `post` never, `wake` at the wake** (the owner's rule, q2488) | The guard is about tokens | A script row's tally counts wakes, not runs |
| A wake's change A | Idle when the script says what it said at the last wake | A red build wakes the agent once | A new watermark field; output with a time in it never matches |
| A wake's change **B** | **The failing exit is the change: never idle, only capped** | The same as an event; no new state | A script that fails all day wakes the agent up to its cap |
| State A | In `routine-runs.json` beside `last_due` | One write a tick; gone with the routine | A second home for a line and a count; the heartbeat could not share the guard |
| State **B** | **`watermarks.json`, key `routine-<id>`** | The issue's "reuse the watermark"; one guard type for both | Two files a run (ordered so a crash neither loses nor repeats a run) |
| Day A | Any 24 hours (the heartbeat today) | No behaviour change | "The rest of the day" and "skipped today" need a time per skip |
| Day **B** | **The Mac's local calendar day** | The issue's words; three counters | The heartbeat's `per_day` becomes "a local day" |
| Default's home | `config.toml` / **`routines.toml` top-level `per_day`** | The server already writes `routines.toml`, comments kept | — |
| Who sets a cap | Agents too / **the owner only, in Settings** | A cap is a spend limit | An agent can't make its own routine run more |

## Decision

**Changed B with B2, Scripts B, a wake never idle, State B, Day B, the default in `routines.toml`, the owner sets
caps.**

1. **One guard, in front of the agent turn.** `cooee-core/src/routines/guard.rs` holds `Cap`, `Change`, `Held`,
   `Tally` and three methods on `Watermark`: `check` (idle before capped, counts nothing), `ran`, `held`. The
   heartbeat and plain routines both call them; the heartbeat's inline copy is deleted. `Routine::turns` says when a
   routine's run is an agent turn: always (a prompt), on a wake (a `wake` script), never (a `post` script).
2. **The rule for a schedule's prompt** (the PR body states it): a due time runs when the routine is fresh (not run
   since it was made, its prompt or schedule edited, or it was resumed), or the owner wrote in a thread of its
   persona after its last run, or it has not run yet this local day. Else it is skipped as idle. "Its persona" is
   the chat's agent profile (`chat`) or `who.profile` (`new_thread`); with none, the Plain chats. Routine prompts,
   replies, script output and silence notes never count.
3. **Events and wakes are never idle.** A delivery, a signed POST or a wake script's non-zero exit is the new input;
   a repeated event is already `seen`. Only the cap applies, after pause and the hourly rate.
4. **The cap** is agent turns started in a local day: the routine's own `per_day`, else the default, else 24. 1 to
   144. Past it a due time, an event or a wake is skipped as capped; what the owner wrote waits for the next run.
   A `post` script has no cap, and setting one on it is refused.
5. **Scripts.** A script's own run is never checked or counted, at a due time or on an event (an event's is still
   under the hourly rate, as #691 left it). `post`: never idle-skipped, never capped, no tally. `wake`: exit 0 is
   `skipped: quiet` as today; a non-zero exit asks the guard before `Runner::prompt`, and at the cap it is
   `skipped: capped` and no agent runs.
6. **Counting.** A schedule's prompt checks, runs, then counts, so a crash re-runs into the same key and never
   loses or doubles a run; a failed start is not counted. An event and a wake check and count in one step, so a
   burst can't pass the cap (a crash can count a wake twice, never run it twice).
7. **State** is `watermarks.json` under `routine-<id>`: `line` (the last run's time) and `today` (day, ran, idle,
   capped). A held due time never touches `last_run`; a held event or wake is recorded `skipped: capped`, as `rate`
   and `quiet` are. An edit of prompt or schedule, or a resume, resets the line; a delete drops the key.
8. **Paused and ended routines** stop before the guard: not checked, not tallied, nothing caught up later. An end
   time or `stop_routine` changes nothing in the watermark; the key stays while the routine stays listed, and
   Settings shows its row as ended with the day's tally. A resume makes it fresh but keeps the day's count, so the
   cap still holds that day.
9. **The silence clock** stays in `routine-runs.json`; the guard never reads or writes `heard` or `silent`, and
   `hear` never opens the watermark. Every signed POST feeds the clock before the guard, so a POST skipped at the
   rate or the cap still says the sender is alive. The silence alert and its "back" line run no agent: not checked,
   not counted. #697's alive POST returns after `hear` and before `accept`: no run, no hourly slot, no tally.
10. **Settings.** `per_day` at the top of `routines.toml` is the default, `per_day` in a `[[routine]]` its own. A new
    screen, `GET /screens/settings/routines`, lists the default and each routine with its cap and today's tally; its
    two actions set the default and a routine's cap. `create_routine` and Chief's `routine` tool can't set one.
11. **The model.** No change and no `Cheapest` for plain routines; one test pins it.
12. **The built-ins.** The heartbeat keeps its own source rule (#498) and its own `per_day` (default 4), now counted
    per local day by the shared guard. Consolidation keeps #219's skip and gets no cap: it is once per due time on
    the cheapest model.

## Consequences

- A fast open-ended prompt routine in a silent persona costs one run a day, not one per due time. A daily or weekly
  one runs as before.
- A prompt that polls the outside on a schedule ("is the build green yet?") runs only while you are writing to its
  persona, plus once a day. The outside's own way in is a wake script (checks for free at each due time, wakes the
  agent only on a failing exit), a signed POST or a GitHub event. `create_routine`'s description gains one sentence
  so agents pick the wake script and don't promise otherwise.
- A wake script that fails all day wakes the agent up to its cap, then is `skipped: capped` until midnight.
- A capped event is dropped, not kept for tomorrow (the same as today's hourly rate), and now shows as the row's
  last run, `skipped: capped`. Raise that routine's cap.
- **Order with #697.** Until #697 merges, a webhook fed an alive ping every 5 minutes passes the hourly rate 6 times
  an hour and reaches a cap of 24 in four hours; its real POSTs are then skipped until midnight. Merge #697 first, or
  give that routine a cap of 144.
- On upgrade, each routine is fresh: its next due time runs. The heartbeat's count starts over once, and its limit
  reads "runs a local day" where it read "in any 24 hours".
- No push and no chat line for a skip: Settings is where it shows.
- Size: about `src/ net +370` (core guard 90, core fields 30, server guard 120, the screen 110, wiring 20; the
  heartbeat shrinks), of 2,238 lines of room. No Swift is expected: the screen is data, the menu row is JSON.
- The design lane, next, draws Settings › routines from [`shapes.md`](shapes.md) §4; the data and the two actions
  are fixed here, the look is theirs.

## Hot files

Measured on staging `4d1935d`.

| File | Lines | PRs in 14 days | What this design does |
|---|---|---|---|
| `cooee-server/src/routines.rs` | 791 of 800 | 19 | Shrinks first, to about 640: `consolidate`, `heartbeat`, `as_persona`, `beat` (165 lines) move to `routines/builtin.rs` in the PR's first commit; then `tick`, `deliver`, `accept`, `edit`, `delete` gain one call each |
| `cooee-server/src/screens/persona.rs` | 365 | 15 | Three lines: `action` hands other screens on |
| `cooee-core/src/agents.rs` | 745 of 800 | 13 | No new line: `Heartbeat.per_day`'s type, and its `PER_DAY` constant becomes a `Cap` |
| `cooee-core/src/history.rs` | 522 | 11 | One field (`runs` becomes `today`); the methods live in `routines/guard.rs` |
| `cooee-core/src/routines.rs` | 594 | 11 | The `per_day` fields, `turns` and `mod guard;` |
| `cooee/tests/routines.rs` | 1,134 | 10 | Untouched: the guards are their own area, `tests/routine_guards.rs` |
| `cooee-server/src/screens/mod.rs` | 41 | 7 | Two lines. It is a hand-kept route list; generating it like `routes/mod.rs` is its own PR, not this one |
| `docs/routines.md` | 92 | 7 | One sentence (the heartbeat's day) and a link; the guards' text is a new `docs/routine-guards.md` |

The files #689, #691 and #692 added under `routines/` are small and already per area: `script.rs` (146) changes
three lines at its wake, `webhook.rs` and `end.rs` none. No file is converted here, so nothing is frozen or
generated. One ban keeps the boundary.

## Slices

- **A**: server `routines/builtin.rs` (the move, first); core `routines/guard.rs`; `Watermark.today`; the heartbeat
  on the guard.
- **B**: `per_day` in `Routine`, `Book`, `Edit` and `routines.toml`; `Routine::turns`; `Skip::Capped`; server
  `routines/guard.rs`; `tick`, `deliver`, `accept`, `script`, `edit`, `delete`; the tool description;
  `tests/routine_guards.rs`; `docs/routine-guards.md`.
- **C**: `screens/routines.rs`, the action hand-off, the menu row and its value, after the design is approved.

## Decided by the owner (q2488)

1. **Once a day when nothing changed:** yes, the first due time of a local day runs.
2. **The first default cap:** 24 a day.
3. **Scripts:** only a wake that prompts the agent is checked and counted; a post-only run never is.
4. **An alive POST** (#697) neither runs the agent nor counts toward the cap or the hourly rate.

## `.drover/arch.toml` (the impl lane adds this in the PR)

```toml
[[ban]]
pattern = "history::watermark"
paths = ["crates/cooee-server/src/**"]
except = ["crates/cooee-server/src/routines/guard.rs", "crates/cooee-server/src/routines/builtin.rs", "crates/cooee-server/src/hub/heartbeat.rs", "crates/cooee-server/src/hub/mcp.rs"]
reason = "#690: a routine's line, cap and tally change only through the guard: use Runner::admit_due, admit_event, admit_wake, count"
```
