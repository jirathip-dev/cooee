# Routine guards: how the callers read (#690)

Written first. Names are the ones [`shapes.md`](shapes.md) defines; the code is a sketch, not the diff. Read on
staging `4d1935d`.

## 1. What the owner sees

```toml
# <config>/routines.toml
per_day = 12                 # the default cap, set in Settings › routines (absent: 24)

[[routine]]
id = "r-1763600000000"
name = "CI watch"
when = "*/15 * * * *"
per_day = 48                 # this routine's own cap (absent: the default)
prompt = "Anything new on the build?"
```

Settings › routines (`GET /screens/settings/routines`, a new row in the avatar menu) is given, and shows:

```text
Default cap     12 runs a day                                                       [edit]
CI watch        every 15 minutes · cap 48 (its own)                                 [edit]
                today: ran 5 · skipped 31 idle, 0 capped
PR review       on pull_request in o/app · cap 12 (default)                         [edit]
                today: ran 12 · skipped 0 idle, 3 capped                            ← capped today
Build check     every 15 minutes · wake script · cap 12 (default)                   [edit]
                today: woke the agent 2 · skipped 0 capped
Disk report     every day 08:00 · post script · no cap: it runs no agent
Trip watch      every hour · ended 2026-11-19 18:00 by stop_routine · cap 12 (default)
                today: ran 0 · skipped 0 idle, 0 capped
```

The design lane draws it from block kinds the app already renders; the wire is in [`shapes.md`](shapes.md) §4.

## 2. A schedule's due time (`Runner::tick`)

```rust
// crates/cooee-server/src/routines.rs: the loop body, three lines changed
let ended = r.end(now);
let due = r.due(now, offset);
if let Some(due) = due {
    if r.paused.is_none() && self.admit_due(r, now).is_ok() {
        let outcome = self.run(r, &r.text(&Fields::Due(due, offset)), &r.key(due));
        self.count(r, &outcome, now);
        self.record(r, r.key(due), outcome, now);
    }
    r.last_due = due;
}
```

A paused or ended routine never reaches the guard. A held due time is tallied inside `admit_due` and moves
`last_due` like a paused one: no run, no tokens, no burst later. `count` counts only a prompt routine's
`Outcome::Ran`. Check, run, count, in that order: a crash before `count` meets the same verdict again and the hub
adopts the run by its key, so still never a second run and never a lost one. For a script routine `admit_due` is
`Ok` and `count` does nothing: its run costs no tokens, and its wake asks for itself (§4).

## 3. An event (`Runner::deliver`, `Runner::accept`)

```rust
match r.admit(now).and_then(|()| self.admit_event(r, now)) {
    Ok(()) => runs.push((r.clone(), r.text(&fields), key)),
    Err(why) => self.record(r, key, Outcome::Skipped { why }, now),   // paused or ended, 6 in the hour, capped
}
```

`admit_event` checks and counts in one step, before the worker starts, so a burst of deliveries can't pass the cap.
A delivery is new input by definition (a repeat is already `seen` and runs nothing), so an event is never idle. A
capped one is `skipped: capped` and `202 runs: 0`. For a script routine `admit_event` is `Ok`: the script runs.

`Runner::inbound` is untouched: `hear` (the silence clock) comes before `accept`, so every signed POST feeds the
clock whatever the guard then says. #697's alive POST returns between the two, so it reaches neither `admit` (the
hourly rate) nor `admit_event` (the cap and the tally).

## 4. A wake script (`Runner::script`)

```rust
// crates/cooee-server/src/routines/script.rs: one arm changed
match (s.mode, ran.ok) {
    (Mode::Wake, true) => Outcome::Skipped { why: Skip::Quiet },
    (Mode::Wake, false) => match self.admit_wake(r, now()) {
        Ok(()) => self.prompt(r, &ran.said, key),
        Err(why) => Outcome::Skipped { why },                         // capped: no agent runs
    },
    (Mode::Post, ok) => { /* as today: it never asks the guard */ }
}
```

The script itself ran either way. Its failing exit is the change, so a wake is never idle; it is checked and
counted in one step, like an event.

## 5. The guard itself

```rust
// crates/cooee-server/src/routines/guard.rs (new): the only routine code that opens the watermark
impl Runner {
    pub(super) fn admit_due(&self, r: &Routine, now: u64) -> Result<(), Held> {
        if r.turns() != Turns::Always {
            return Ok(());
        }
        let day = day(now, self.offset(now));
        let w = self.mark(r, |w| w.clone());
        let change = w.fresh_or_new_day(day).or_else(|| self.wrote(r, w.line).then_some(Change::Said));
        let verdict = w.check(day, self.cap(r), change).map(|_| ());
        if let Err(why) = verdict {
            self.mark(r, |w| w.held(day, why));
        }
        verdict
    }

    pub(super) fn admit_event(&self, r: &Routine, now: u64) -> Result<(), Skip> {
        if r.turns() != Turns::Always {
            return Ok(());
        }
        self.admit_now(r, Change::Event, now)
    }

    pub(super) fn admit_wake(&self, r: &Routine, now: u64) -> Result<(), Skip> {
        self.admit_now(r, Change::Woke, now)
    }

    fn admit_now(&self, r: &Routine, change: Change, now: u64) -> Result<(), Skip> {
        let (day, cap) = (day(now, self.offset(now)), self.cap(r));
        self.mark(r, |w| match w.check(day, cap, Some(change)) {
            Ok(_) => {
                w.ran(day, now);
                Ok(())
            }
            Err(why) => {
                w.held(day, why);
                Err(Skip::Capped)
            }
        })
    }

    fn wrote(&self, r: &Routine, line: u64) -> bool {
        let profile = match &r.place {
            Place::Chat { thread } => self.hub.thread(thread).map(|(t, _)| t.profile),
            Place::NewThread { who } => Ok(who.profile.clone()),
        };
        profile.and_then(|p| self.hub.news(&p, (line, false))).map_or(true, |news| news.is_some())
    }
}
```

`Hub::news(profile, (line, false))` is #219's call, unchanged: "did the owner write in one of this persona's
threads after `line`". A chat that is gone counts as changed, so the run fails and pauses the routine as today.
`self.cap(r)` is the routine's own `per_day`, else `routines.toml`'s, else 24; the guard reads it, so `script.rs`,
which holds no book, asks the same way. The same file holds `count` (a schedule prompt's `Ran`: `w.ran(day, now)`),
`fresh` (`Runner::edit` calls it when the prompt or the schedule changed or the routine was resumed: the line back
to 0, the day's tally kept) and `forget` (`Runner::delete`: the key is dropped). `Runner::stop` and an end time
call none of them.

## 6. The heartbeat, on the same guard

```rust
// crates/cooee-server/src/routines/builtin.rs (moved from routines.rs): inside its watermark closure
let news = self.hub.news(&p.id, (w.line, true))?;
match w.check(day, cap, news.as_ref().map(|_| Change::Said)) {
    Err(why) => { w.held(day, why); Ok(None) }
    Ok(_) => { /* as today: w.ran(day, newest), w.last = Some(Beat { … }), the prompt with the news */ }
}
```

Its own source rule stays (#498: replies and routine results count, no "once a day"); only the counting is shared.

## 7. Settings' two edits

```text
POST /screens/action  {"screen": "/screens/settings/routines", "action": "default", "values": {"per_day": "12"}}
POST /screens/action  {"screen": "/screens/settings/routines", "action": "cap", "values": {"routine": "r-1", "per_day": "48"}}
POST /screens/action  {… "action": "cap", "values": {"routine": "r-1", "per_day": ""}}     # back to the default
```

```rust
// crates/cooee-server/src/screens/routines.rs (new)
match a.action.as_str() {
    "default" => routines::on(cx, move |r| r.set_default(&per_day)).await?,
    "cap" => routines::on(cx, move |r| r.edit(&id, Edit { per_day: Some(per_day), ..Edit::default() }, now())).await?,
    other => return Err(refused(format!("no action {other}"))),
};
screen(cx).await        // each answers the screen again; 0, 145 or "x" is `BAD_CAP: a cap is 1 to 144 runs a day`
```

`POST /routines/<id>` takes the same `per_day` field (it is `Edit`). A cap on a `post` script is refused
(`BAD_CAP: a script that only posts runs no agent`). `create_routine` and Chief's `routine` tool take none: an
agent never sets its own cap.

## 8. The tests, by AC line (new file `crates/cooee/tests/routine_guards.rs`, through the real server)

| AC 2 | Test |
|---|---|
| no change means no run | A chat routine's first due time runs (fresh). The next one the same day, nothing written: no new message, the screen says `idle 1`. |
| a change means a run | The owner writes in that chat; the next due time runs. Another routine's reply alone does not wake it. With nothing written, the first due time of the next local day runs once. |
| the cap holds, default | Default set to 2 through the screen action; the owner writes before each due time; the third that day does not run, the screen says `capped 1`. The tally's day set back one (as `heartbeat.rs` sets a due back): it runs again. |
| the cap holds, override | The same with `per_day = 1` on the routine under a default of 2. A webhook routine with `per_day = 1` and a silence rule: the second signed POST of the day is `202 runs: 0` and `skipped: capped`, and its `heard` still moved. |
| a run's model is the chat's | A chat on model `m2`: the routine's turn runs on `m2`; a `new_thread` run's thread has the model the chat had, never the provider's cheapest. |
| scripts (q2488) | Under a default of 1 and nothing written: a `post` script posts at two due times the same day, its tally zero. A `wake` script that fails wakes the agent at the first and is `skipped: capped` at the second; one that exits 0 counts nothing. |
| ended (#689) | An ended routine's due time tallies nothing; resumed, its next due time runs with nothing written. |

Core unit tests sit in `routines/guard.rs`: `check`'s order (idle before capped), the day rollover, `Cap::parse`.
`heartbeat.rs` keeps its tests; where they read `runs` from the watermark they read `today.ran`. #697's own tests
cover the alive POST; if it is on staging when impl starts, its "12 alive POSTs" test also reads the tally as zero.
