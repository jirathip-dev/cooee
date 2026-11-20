# Routine guards: how the callers read (#690)

Written first. Names are the ones [`shapes.md`](shapes.md) defines; the code is a sketch, not the diff.

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
Default cap            12 runs a day                                  [edit]
CI watch               every 15 minutes · cap 48 (its own)            [edit]
                       today: ran 5 · skipped 31 idle, 0 capped
Monday renewals        Mondays 09:00 · cap 12 (default)               [edit]
                       today: ran 1 · skipped 0 idle, 0 capped
PR review              on pull_request in o/app · cap 12 (default)    [edit]
                       today: ran 12 · skipped 0 idle, 3 capped       ← capped today
```

The design lane draws it from block kinds the app already renders; the wire is in [`shapes.md`](shapes.md) §4.

## 2. A schedule's due time (`Runner::tick`)

```rust
// crates/cooee-server/src/routines.rs: the loop body, three lines changed
let Some(due) = r.due(now, offset) else { continue };
if r.paused.is_none() && self.admit_due(r, r.cap(default), now).is_ok() {
    let outcome = self.run(r, &r.text(&Fields::Due(due, offset)), &r.key(due));
    self.count(r, &outcome, now);
    self.record(r, r.key(due), outcome, now);
}
r.last_due = due;
```

A held due time is tallied inside `admit_due` and moves `last_due` like a paused one: no run, no tokens, no burst
later. `count` counts only `Outcome::Ran`. Check, run, count, in that order: a crash before `count` meets the same
verdict again and the hub adopts the run by its key, so still never a second run and never a lost one.

## 3. An event (`Runner::deliver`, `Runner::accept`)

```rust
match r.admit(now) {
    Err(why) => self.record(r, key, Outcome::Skipped { why }, now),      // paused, or 6 in the hour: as today
    Ok(()) if self.admit_event(r, r.cap(default), now).is_err() => {}    // capped: tallied, 202 `runs: 0`
    Ok(()) => runs.push((r.clone(), r.text(&fields), key)),
}
```

`admit_event` checks and counts in one step, before the worker starts, so a burst of deliveries can't pass the cap.
A delivery is new input by definition (a repeat is already `seen` and runs nothing), so an event is never idle.

## 4. The guard itself

```rust
// crates/cooee-server/src/routines/guard.rs (new): the only routine code that opens the watermark
impl Runner {
    pub(super) fn admit_due(&self, r: &Routine, cap: Cap, now: u64) -> Result<Change, Held> {
        let day = day(now, self.offset(now));
        let w = self.mark(r, |w| w.clone());
        let change = w.fresh_or_new_day(day).or_else(|| self.wrote(r, w.line).then_some(Change::Said));
        let verdict = w.check(day, cap, change);
        if let Err(why) = verdict {
            self.mark(r, |w| w.held(day, why));
        }
        verdict
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
The same file holds `count` (a schedule's `Ran`: `w.ran(day, now)`), `admit_event` (`check` and `ran` in one
closure), `fresh` (`Runner::edit` calls it when the prompt or the schedule changed: the line back to 0) and
`forget` (`Runner::delete`: the key is dropped).

## 5. The heartbeat, on the same guard

```rust
// crates/cooee-server/src/routines/builtin.rs (moved from routines.rs): inside its watermark closure
let news = self.hub.news(&p.id, (w.line, true))?;
match w.check(day, cap, news.as_ref().map(|_| Change::Said)) {
    Err(why) => { w.held(day, why); Ok(None) }
    Ok(_) => { /* as today: w.ran(day, newest), w.last = Some(Beat { … }), the prompt with the news */ }
}
```

Its own source rule stays (#498: replies and routine results count, no "once a day"); only the counting is shared.

## 6. Settings' two edits

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

`POST /routines/<id>` takes the same `per_day` field (it is `Edit`). `create_routine` and Chief's `routine` tool take
none: an agent never sets its own cap.

## 7. The tests, by AC line (new file `crates/cooee/tests/routine_guards.rs`, through the real server)

| AC 2 | Test |
|---|---|
| no change means no run | A chat routine's first due time runs (fresh). The next one the same day, nothing written: no new message, the screen says `idle 1`. |
| a change means a run | The owner writes in that chat; the next due time runs. Another routine's reply alone does not wake it. |
| the cap holds, default | Default set to 2 through the screen action; the owner writes before each due time; the third that day does not run, the screen says `capped 1`. The tally's day set back one (as `heartbeat.rs` sets a due back): it runs again. |
| the cap holds, override | The same with `per_day = 1` on the routine under a default of 2. A webhook routine with `per_day = 1`: the second signed POST of the day is `202 runs: 0`. |
| a run's model is the chat's | A chat on model `m2`: the routine's turn runs on `m2`; a `new_thread` run's thread has the model the chat had, never the provider's cheapest. |
| (open question 1) | With nothing written, the first due time of the next local day runs once. |

Core unit tests sit in `routines/guard.rs`: `check`'s order (idle before capped), the day rollover, `Cap::parse`.
`heartbeat.rs` keeps its tests; where they read `runs` from the watermark they read `today.ran`.
