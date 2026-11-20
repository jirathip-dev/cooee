# Routine guards: the data shapes (#690)

## 1. The guard (new: `crates/cooee-core/src/routines/guard.rs`)

Pure: it is given `now`, the local day and whether the owner wrote. No clock, no files, no hub.

```rust
/// At most this many runs of one routine a local day: 1 to 144 (six an hour, the most an event routine can run).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct Cap(u32);

impl Cap {
    /// The cap until the owner sets a default.
    pub const DEFAULT: Self = Self(24);
    /// `BAD_CAP: a cap is 1 to 144 runs a day` for anything else.
    pub fn parse(text: &str) -> Result<Self, String>;
}

/// What changed since a routine's last run: why this one may go ahead.
pub enum Change {
    /// An event came: a GitHub delivery or a signed POST.
    Event,
    /// It has not run since it was made, or since its prompt or schedule was edited.
    Fresh,
    /// It has not run yet this local day.
    NewDay,
    /// The owner wrote after its last run (the heartbeat: anything was said).
    Said,
}

/// Why the guard held a run back.
pub enum Held { Idle, Capped }

/// What a routine did on one local day.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tally {
    /// The local day, in days since 1970.
    pub day: u64,
    /// Runs started.
    pub ran: u32,
    /// Due times skipped because nothing changed.
    pub idle: u32,
    /// Due times and events skipped at the cap.
    pub capped: u32,
}

/// The local day of `now` (ms) in UTC offset `offset_min`.
pub fn day(now: u64, offset_min: i32) -> u64;

impl Watermark {
    /// `day`'s tally: zeros once the day has changed.
    pub fn today(&self, day: u64) -> Tally;
    /// `Fresh` or `NewDay` when so; else none, and the caller asks whether something was said after `line`.
    pub fn fresh_or_new_day(&self, day: u64) -> Option<Change>;
    /// Whether a run may go ahead: `Idle` without a change, `Capped` at `cap` runs today. Counts nothing.
    pub fn check(&self, day: u64, cap: Cap, change: Option<Change>) -> Result<Change, Held>;
    /// Counts a run and moves the line to `line`.
    pub fn ran(&mut self, day: u64, line: u64);
    /// Counts a held run; the line stays, so the news waits.
    pub fn held(&mut self, day: u64, why: Held);
}
```

## 2. What changes in existing types

```rust
// cooee-core history.rs
pub struct Watermark {
    pub line: u64,              // routines: the time of the last run (0: fresh); the heartbeat: the newest line read
    pub due: u64,               // unused by `routine-<id>` keys: their once-per-due record stays `Routine.last_due`
    pub today: Tally,           // replaces `runs: Vec<u64>`; an old file's `runs` is ignored (the count starts over once)
    pub last: Option<Beat>,
}
// `history::watermark` drops a key whose value is back to the default (a deleted routine leaves nothing).

// cooee-core routines.rs
pub struct Routine { …, pub per_day: Option<Cap> }       // its own cap; in routines.toml, not in the run state
pub struct Book    { …, pub per_day: Option<Cap> }       // the default; routines.toml's top-level `per_day`
pub struct Edit    { …, pub per_day: Option<String> }    // a number sets it, "" clears it
impl Routine { pub fn cap(&self, default: Option<Cap>) -> Cap }   // own, else default, else Cap::DEFAULT

// cooee-core agents.rs
pub struct Heartbeat { …, pub per_day: Option<Cap> }     // was Option<usize>; the same TOML; now runs a local day
```

`Ask` (`create_routine`) and Chief's `routine` tool gain nothing. `Skip`, `Outcome` and `last_run` are untouched: a
held run is tallied, never written over the last real run.

## 3. The files

```json
// <data>/watermarks.json: one more kind of key, the same shape as its neighbours
{
  "consolidate-travel": { "line": 1763600000000, "due": 1763607600000 },
  "heartbeat-travel":   { "line": 1763600000000, "due": 1763611200000, "today": { "day": 20412, "ran": 2, "idle": 5 }, "last": { … } },
  "routine-r-1763600000000": { "line": 1763611200000, "today": { "day": 20412, "ran": 5, "idle": 31, "capped": 0 } }
}
```

`<config>/routines.toml` gains the two `per_day` keys ([callers §1](callers.md)); `routine-runs.json` is unchanged.
No migration: a routine with no watermark is `Fresh` and runs at its next due time.

## 4. Settings › routines on the wire

`GET /screens/settings/routines` answers a core screen (`id: routines`) built in the server from:

```rust
// cooee-server routines/guard.rs
pub struct Guards { pub per_day: Cap, pub rows: Vec<Guarded> }
pub struct Guarded { pub routine: Routine, pub cap: Cap, pub today: Tally }   // `routine.per_day` says own or default
impl Runner { pub fn guards(&self, now: u64) -> Result<Guards, HubError>; pub fn set_default(&self, per_day: &str) -> Result<Cap, HubError>; }
```

Each row must carry: name, `when()`, the cap and whether it is its own, today's `ran`, `idle` and `capped`, and a
marked state when `capped > 0` or `ran` reached the cap. Its two actions are in [callers §6](callers.md).
`GET /routines.json` gains only `per_day` (it serialises `Routine`). The menu's bundled `menu.json` gains one row,
`{"id": "routines", "title": "Routines", "open": "/screens/settings/routines"}`.
