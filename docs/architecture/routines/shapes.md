# The data shapes (#75)

All in `crates/cooee-core/src/routines.rs` (types, store, rendering) and `routines/cron.rs` (the schedule) unless
named. Pure: no sockets, no processes; the server's runner does those.

## The routine

```rust
/// A routine: when it runs, where it posts and what it says. Made by an agent in chat (`create_routine`), edited in
/// Settings › routines; never in config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Routine {
    pub id: String,                 // `r-<stamp>`: a thread-id-safe stamp, never reused
    pub name: String,               // 1..=60 chars, the `.rt` line and the list row
    pub trigger: Trigger,
    pub place: Place,
    pub prompt: Template,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paused: Option<Paused>,
    pub made_in: String,            // the thread whose agent made it (its `.rt` line is there)
    pub made_at: u64,               // ms; a schedule's first due time is after this
    #[serde(default, skip_serializing_if = "is_zero")]
    pub last_due: u64,              // the newest due time already run (ms): the "once per due time" record
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run: Option<Run>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    Schedule(Schedule),             // stored as its cron text; parsed at the edge (load, the tool, the form)
    Github(GithubTrigger),
}

/// Five-field cron in the Mac's local time: minute hour day-of-month month day-of-week; `*`, `n`, `a-b`, `*/s`,
/// `a-b/s` and comma lists; day-of-week 0–7 (0 and 7 Sunday); day-of-month and day-of-week both set: either matches.
/// No names, no seconds. Refused when two due times in the next 7 days are under 15 minutes apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Schedule { text: String, fields: [BitSet64; 5] }

impl Schedule {
    pub fn parse(text: &str) -> Result<Self, ScheduleError>;
    /// The newest due time at or before `now` (ms), in UTC offset `offset_min`; None before any.
    pub fn latest(&self, now: u64, offset_min: i32) -> Option<u64>;
    pub fn next(&self, after: u64, offset_min: i32) -> Option<u64>;
    pub fn describe(&self) -> String;  // "Mondays 09:00", "every day 18:30", else the cron text
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GithubTrigger {
    pub repo: String,               // `owner/name`, matched case-insensitively with `repository.full_name`
    pub events: Vec<String>,        // `pull_request` (any action) or `pull_request.opened`; 1..=8
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub r#match: BTreeMap<String, String>, // payload path → the value it must equal as text ("failure", "true")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "in", rename_all = "snake_case")]
pub enum Place {
    /// Post into the chat that made it, as a message marked ⟳ (the design's default).
    Chat { thread: String },
    /// Each run its own thread, started as `who` (a snapshot of the making chat's agent, taken at creation).
    New { who: Who },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Who {
    pub agent: String, pub account: String, pub profile: String, // a persona's or profile's id, as POST /threads
    pub folder: PathBuf, pub model: String, pub effort: String, pub machine: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "why", rename_all = "snake_case")]
pub enum Paused {
    Owner,                          // [pause] in Settings
    ChatGone { thread: String },    // a `Place::Chat` run found its chat closed or deleted
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run { pub at: u64, pub key: String, pub outcome: Outcome }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
    Ran { thread: String },
    Skipped { why: Skip },          // paused, or over the rate
    Failed { why: String },         // the hub's sentence
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Skip { Paused, Rate }      // Rate: an event routine's 7th run inside an hour
```

## The prompt template

```rust
/// `{{path}}` placeholders: a dotted path into the event's payload (`pull_request.title`, `commits.0.message`), or
/// one of `due` (a schedule's due time, local `YYYY-MM-DD HH:MM`), `event` (`pull_request.opened`), `repo`.
/// A string, number or bool goes in as text; an object or array as compact JSON; each value at most 2,000 chars
/// (cut with `…`); a missing path as nothing. Parsed at the edge: an unclosed `{{` is refused.
#[serde(try_from = "String", into = "String")]
pub struct Template { text: String, parts: Vec<Part> }

pub enum Fields<'a> { Due(u64, i32), Event { event: &'a str, payload: &'a serde_json::Value } }

impl Template { pub fn render(&self, f: &Fields) -> String; }
```

## A delivery: proven before it is built

```rust
/// One GitHub webhook delivery. Its constructors are the only proofs: `signed` checks HMAC-SHA256 of the raw body
/// (`X-Hub-Signature-256: sha256=<hex>`, constant-time, `hkdf::hmac`, no new crate); `fetched` is one the catch-up
/// read from GitHub's own delivery log over TLS with the owner's token. No other way to make one.
pub struct Delivery { guid: String, event: String, at: u64, payload: serde_json::Value }

impl Delivery {
    pub fn signed(h: &Headers, body: &[u8], secret: &[u8]) -> Result<Self, Rejected>;
    pub fn fetched(guid: String, event: String, at: u64, payload: serde_json::Value) -> Self; // github.rs only
    /// `pull_request.opened` (with `payload.action`) or `push`.
    pub fn event(&self) -> String;
}

pub enum Rejected { TooBig, NoHeaders, BadSignature, NotJson }   // 413, 400, 401, 400

/// The routines a delivery runs (each not paused, under its rate): `repo` and one `events` entry and every `match`.
pub fn matching<'a>(routines: &'a [Routine], d: &Delivery) -> impl Iterator<Item = &'a Routine>;
```

## On disk: `<data>/routines.json` (0600)

```json
{ "format": 1,
  "routines": [ { "id": "r-1731301200000", "name": "Monday renewals",
                  "trigger": { "schedule": "0 9 * * 1" },
                  "place": { "in": "chat", "thread": "t-1731300000000" },
                  "prompt": "Which client renewals are due this month? Draft a short note for each.",
                  "made_in": "t-1731300000000", "made_at": 1731301200000,
                  "last_due": 1731895200000,
                  "last_run": { "at": 1731895230000, "key": "routine-r-1731301200000-1731895200000",
                                "outcome": "ran", "thread": "t-1731300000000" } } ],
  "seen": [ ["4f1c…-guid", 1731895000000] ] }
```

`seen` is the deliveries already handled (by the hook or the catch-up), kept 4 days (GitHub keeps its log 3). One
file, one lock, written write-temp → fsync → rename → fsync folder, like a thread's. A newer `format` is refused.

## Run keys: the idempotency

```rust
pub enum RunKey { Due(String, u64), Delivery(String, String) }   // (routine, due ms) | (routine, delivery guid)
impl RunKey { pub fn random_id(&self) -> String; }                // `routine-<id>-<due>` | `routine-<id>-<guid>`
```

The key is the hub's `random_id`: `create_later` adopts a thread already started with it and `send` stores a
message once per it, both across restarts. So a run is idempotent without a second ledger.

## Changes to existing types

```rust
// cooee-core store.rs Thread: the routine whose run started it (Place::New)
#[serde(default, skip_serializing_if = "Option::is_none")]
pub routine: Option<Mark>,
// cooee-core chat.rs Body::You: the routine that sent it (Place::Chat); a new Body::Routine is the `.rt` line
#[serde(default, skip_serializing_if = "Option::is_none")]
routine: Option<Mark>,
Routine { routine: String, name: String, when: String, place: String },   // "Mondays 09:00", "in this chat"
// chat.rs Summary / API Row: `routine: string` ("" none)
pub struct Mark { pub id: String, pub name: String }
// cooee-server hub.rs Recipe: `routine: Option<Mark>`; cooee-core config.rs: `hooks: Option<Hooks>`
pub struct Hooks { pub bind: SocketAddr, pub url: String } // `[hooks] bind = "127.0.0.1:4748"`, `url` = the Funnel URL
```

All additions are `serde(default)`: every earlier thread file loads unchanged (`tests/upgrade_compat.rs` keeps
passing with no new fixture).
