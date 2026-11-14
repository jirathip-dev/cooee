//! Routines (#75, #261): a schedule or a GitHub event that posts a prompt to an agent, made by the agent in chat and
//! kept in `<data>/routines.json` with its run state and the deliveries seen. Pure: the server's runner gives `now`,
//! the Mac's UTC offset and each delivery.

mod cron;

use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
};

use hkdf::hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;

pub use cron::{Schedule, local};

use crate::Thread;

/// How many routines there may be.
pub const MAX: usize = 50;

/// How many times an event may run one routine in an hour.
pub const RATE: usize = 6;

/// An hour, in milliseconds.
const HOUR: u64 = 3_600_000;

/// How long a delivery's guid is kept (GitHub keeps its log three days).
const KEEP: u64 = 96 * HOUR;

/// The routine a thread or message came from, as the list and the chat show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mark {
    /// The routine's id.
    pub id: String,
    /// Its name.
    pub name: String,
}

/// A routine: when it runs, where it posts and what it says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Routine {
    /// `r-<stamp>`, never reused.
    pub id: String,
    /// 1 to 60 characters.
    pub name: String,
    /// When it runs.
    pub trigger: Trigger,
    /// Where each run posts.
    pub place: Place,
    /// What each run says: see [`Routine::text`].
    pub prompt: String,
    /// Why it doesn't run, if it doesn't.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paused: Option<Paused>,
    /// The thread whose agent made it.
    pub made_in: String,
    /// When, in Unix ms: its first due time is after this.
    pub made_at: u64,
    /// The newest due time already run (ms): the "once per due time" record.
    #[serde(default)]
    pub last_due: u64,
    /// Its last run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run: Option<Run>,
    /// When events ran it in the last hour (ms), for [`RATE`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent: Vec<u64>,
}

/// When a routine runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    /// A five-field cron in the Mac's local time.
    Schedule(Schedule),
    /// A GitHub webhook event.
    Github(Github),
}

/// The GitHub events a routine runs on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Github {
    /// `owner/name`, matched case-insensitively with the payload's `repository.full_name`.
    pub repo: String,
    /// `pull_request` (any action) or `pull_request.opened`; 1 to 8.
    pub events: Vec<String>,
    /// Payload paths (`workflow_run.conclusion`) and the text each must equal.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub r#match: BTreeMap<String, String>,
}

impl Github {
    /// Itself, when its repo and events are well formed.
    fn checked(self) -> Result<Self, String> {
        let word = |w: &str| !w.is_empty() && !w.contains(['/', ' ']);
        let repo = self
            .repo
            .split_once('/')
            .is_some_and(|(o, n)| word(o) && word(n));
        let events = (1..=8).contains(&self.events.len()) && self.events.iter().all(|e| word(e));
        if repo && events {
            Ok(self)
        } else {
            Err("BAD_GITHUB: repo is owner/name, events 1 to 8 like pull_request.opened".to_owned())
        }
    }
}

/// Where a routine's runs post.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "in", rename_all = "snake_case")]
pub enum Place {
    /// The chat that made it, as a message marked with the routine.
    Chat {
        /// That chat.
        thread: String,
    },
    /// Each run its own thread, started as the making chat's agent was.
    NewThread {
        /// That agent, as it was when the routine was made.
        who: Who,
    },
}

/// How a `new_thread` run starts: the making chat's agent, account, profile or persona, folder, model, effort and
/// machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Who {
    /// The agent.
    pub agent: String,
    /// The account.
    pub account: String,
    /// The profile's or persona's id (empty: Plain).
    pub profile: String,
    /// The folder (empty: a scratch folder).
    pub folder: PathBuf,
    /// The model.
    pub model: String,
    /// The effort.
    pub effort: String,
    /// The machine (empty: this host).
    pub machine: String,
}

/// Why a routine is paused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "why", rename_all = "snake_case")]
pub enum Paused {
    /// The owner paused it.
    Owner,
    /// Its chat was closed or deleted.
    ChatGone {
        /// That chat.
        thread: String,
    },
}

/// A routine's run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    /// When, in Unix ms.
    pub at: u64,
    /// Its key, the hub's `random_id`: a run again with it adopts the first.
    pub key: String,
    /// How it went.
    #[serde(flatten)]
    pub outcome: Outcome,
}

/// How a run went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
    /// It posted in `thread`.
    Ran {
        /// The thread.
        thread: String,
    },
    /// It didn't, and why.
    Failed {
        /// The hub's sentence.
        why: String,
    },
    /// An event didn't run it.
    Skipped {
        /// Why.
        why: Skip,
    },
}

/// Why an event didn't run a routine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Skip {
    /// It is paused.
    Paused,
    /// It ran [`RATE`] times in the last hour.
    Rate,
}

/// `create_routine`'s arguments: `when` or `github`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ask {
    /// The name.
    pub name: String,
    /// The cron.
    #[serde(default)]
    pub when: Option<String>,
    /// The GitHub events.
    #[serde(default)]
    pub github: Option<Github>,
    /// The prompt.
    pub prompt: String,
    /// `chat` (default) or `new_thread`.
    #[serde(default)]
    pub place: String,
}

/// Settings › routines' edit: each field given is changed.
#[derive(Debug, Default, Deserialize)]
pub struct Edit {
    /// A new name.
    pub name: Option<String>,
    /// A new cron.
    pub when: Option<String>,
    /// A new prompt.
    pub prompt: Option<String>,
    /// Pause, or resume.
    pub paused: Option<bool>,
}

impl Routine {
    /// The routine `ask` says, made by thread `from`'s agent at `now`, among `list`.
    ///
    /// # Errors
    /// `BAD_SCHEDULE`, `BAD_GITHUB`, `BAD_TRIGGER`, `BAD_NAME`, `BAD_PROMPT`, `BAD_PLACE` or `TOO_MANY`, said for
    /// the agent.
    pub fn make(list: &[Self], ask: Ask, from: &Thread, now: u64) -> Result<Self, String> {
        if list.len() >= MAX {
            return Err(format!("TOO_MANY: there are {MAX} routines already"));
        }
        let place = match ask.place.as_str() {
            "" | "chat" => Place::Chat {
                thread: from.id.clone(),
            },
            "new_thread" => Place::NewThread {
                who: Who {
                    agent: from.agent.clone(),
                    account: from.account.clone(),
                    profile: from.profile.clone(),
                    folder: from.folder.clone().unwrap_or_default(),
                    model: from.model.clone(),
                    effort: from.effort.clone(),
                    machine: from.machine.clone(),
                },
            },
            p => return Err(format!("BAD_PLACE: {p:?} is neither chat nor new_thread")),
        };
        let trigger = match (ask.when, ask.github) {
            (Some(w), None) => Trigger::Schedule(Schedule::parse(&w)?),
            (None, Some(g)) => Trigger::Github(g.checked()?),
            _ => {
                return Err("BAD_TRIGGER: give when (a schedule) or github, one of them".to_owned());
            }
        };
        let mut stamp = now;
        while list.iter().any(|r| r.id == format!("r-{stamp}")) {
            stamp += 1;
        }
        let mut r = Self {
            id: format!("r-{stamp}"),
            name: String::new(),
            trigger,
            place,
            prompt: String::new(),
            paused: None,
            made_in: from.id.clone(),
            made_at: now,
            last_due: 0,
            last_run: None,
            recent: Vec::new(),
        };
        r.edit(
            Edit {
                name: Some(ask.name),
                prompt: Some(ask.prompt),
                ..Edit::default()
            },
            now,
        )?;
        Ok(r)
    }

    /// Changes what `e` gives; a new schedule runs from `now` on.
    ///
    /// # Errors
    /// As [`Routine::make`]; nothing is changed then.
    pub fn edit(&mut self, e: Edit, now: u64) -> Result<(), String> {
        let name = e.name.map(|n| n.trim().to_owned());
        if name
            .as_ref()
            .is_some_and(|n| n.is_empty() || n.chars().count() > 60)
        {
            return Err("BAD_NAME: a name is 1 to 60 characters".to_owned());
        }
        let prompt = e.prompt.map(|p| p.trim().to_owned());
        let unclosed = |p: &str| p.rsplit_once("{{").is_some_and(|(_, t)| !t.contains("}}"));
        if prompt.as_ref().is_some_and(|p| p.is_empty() || unclosed(p)) {
            return Err("BAD_PROMPT: a prompt is not empty and closes each {{".to_owned());
        }
        let trigger = e.when.map(|w| Schedule::parse(&w)).transpose()?;
        if let Some(t) = trigger
            .map(Trigger::Schedule)
            .filter(|t| *t != self.trigger)
        {
            self.trigger = t;
            self.last_due = now;
        }
        self.name = name.unwrap_or_else(|| self.name.clone());
        self.prompt = prompt.unwrap_or_else(|| self.prompt.clone());
        match e.paused {
            Some(true) => self.paused = Some(Paused::Owner),
            Some(false) => self.paused = None,
            None => {}
        }
        Ok(())
    }

    /// What the list and the chat show it as.
    #[must_use]
    pub fn mark(&self) -> Mark {
        Mark {
            id: self.id.clone(),
            name: self.name.clone(),
        }
    }

    /// Its trigger in words.
    #[must_use]
    pub fn when(&self) -> String {
        match &self.trigger {
            Trigger::Schedule(s) => s.describe(),
            Trigger::Github(g) => format!("on {} in {}", g.events.join(", "), g.repo),
        }
    }

    /// The due time to run at `now` (ms, in UTC offset `offset_min`): the newest one past both its last run and its
    /// making, so missed ones run once, late.
    #[must_use]
    pub fn due(&self, now: u64, offset_min: i32) -> Option<u64> {
        let Trigger::Schedule(s) = &self.trigger else {
            return None;
        };
        s.latest(now, offset_min)
            .filter(|due| *due > self.last_due.max(self.made_at))
    }

    /// The key of its run for a due time or a delivery's guid: the hub's `random_id`, so a run again adopts the first.
    #[must_use]
    pub fn key(&self, at: impl std::fmt::Display) -> String {
        format!("routine-{}-{at}", self.id)
    }

    /// The prompt, each `{{path}}` filled: `due` (local `YYYY-MM-DD HH:MM`), `event`, `repo`, or a dotted path into
    /// the event's payload (`pull_request.title`); each value at most 2,000 characters, a missing one empty.
    #[must_use]
    pub fn text(&self, f: &Fields) -> String {
        let (mut out, mut rest) = (String::new(), self.prompt.as_str());
        while let Some((before, after)) = rest.split_once("{{") {
            let Some((path, tail)) = after.split_once("}}") else {
                break;
            };
            let value = f.get(path.trim());
            out.push_str(before);
            out.extend(value.chars().take(2000));
            if value.chars().nth(2000).is_some() {
                out.push('…');
            }
            rest = tail;
        }
        out + rest
    }

    /// Whether delivery `d` is one of its events: its repo, one of its events and every `match`.
    #[must_use]
    pub fn matches(&self, d: &Delivery) -> bool {
        let Trigger::Github(g) = &self.trigger else {
            return false;
        };
        let event = d.event();
        g.repo.eq_ignore_ascii_case(d.repo())
            && g.events.iter().any(|e| *e == d.event || *e == event)
            && g.r#match.iter().all(|(path, v)| d.at(path) == *v)
    }

    /// Whether an event may run it at `now`: not paused and under [`RATE`] in the hour; a run is counted.
    ///
    /// # Errors
    /// Why not.
    pub fn admit(&mut self, now: u64) -> Result<(), Skip> {
        if self.paused.is_some() {
            return Err(Skip::Paused);
        }
        self.recent.retain(|t| now.saturating_sub(*t) < HOUR);
        if self.recent.len() >= RATE {
            return Err(Skip::Rate);
        }
        self.recent.push(now);
        Ok(())
    }
}

/// What a prompt's `{{…}}` read: a schedule's due time (ms) in a UTC offset (minutes), or an event.
pub enum Fields<'a> {
    /// A due time.
    Due(u64, i32),
    /// A delivery.
    Event(&'a Delivery),
}

impl Fields<'_> {
    fn get(&self, path: &str) -> String {
        match (self, path) {
            (Self::Due(due, offset), "due") => local(*due, *offset),
            (Self::Event(d), "event") => d.event(),
            (Self::Event(d), "repo") => d.repo().to_owned(),
            (Self::Event(d), path) => d.at(path),
            (Self::Due(..), _) => String::new(),
        }
    }
}

/// One GitHub webhook delivery, proven: [`Delivery::signed`] checked its signature, [`Delivery::fetched`] read it
/// from GitHub's own delivery log with the owner's token.
#[derive(Debug, Clone)]
pub struct Delivery {
    guid: String,
    event: String,
    payload: Value,
}

/// Why a webhook delivery was refused.
#[derive(Debug, thiserror::Error)]
pub enum Rejected {
    /// No `X-GitHub-Event` or `X-GitHub-Delivery`.
    #[error("no X-GitHub-Event or X-GitHub-Delivery")]
    NoHeaders,
    /// `X-Hub-Signature-256` isn't the body's HMAC.
    #[error("the signature doesn't check out")]
    BadSignature,
    /// The body isn't JSON.
    #[error("the body isn't JSON")]
    NotJson,
}

impl Rejected {
    /// Its HTTP status.
    #[must_use]
    pub const fn status(&self) -> u16 {
        match self {
            Self::BadSignature => 401,
            Self::NoHeaders | Self::NotJson => 400,
        }
    }
}

impl Delivery {
    /// The delivery `body` with headers `X-GitHub-Event`, `X-GitHub-Delivery` and `X-Hub-Signature-256`, its
    /// HMAC-SHA256 under `secret` checked in constant time before the body is read.
    ///
    /// # Errors
    /// A missing header, a wrong signature, or a body that isn't JSON.
    pub fn signed(
        (event, guid, signature): (&str, &str, &str),
        body: &[u8],
        secret: &[u8],
    ) -> Result<Self, Rejected> {
        if event.is_empty() || guid.is_empty() {
            return Err(Rejected::NoHeaders);
        }
        let sig = signature.strip_prefix("sha256=").and_then(unhex);
        let mut mac = Hmac::<Sha256>::new_from_slice(secret).map_err(|_| Rejected::BadSignature)?;
        mac.update(body);
        mac.verify_slice(&sig.ok_or(Rejected::BadSignature)?)
            .map_err(|_| Rejected::BadSignature)?;
        let payload = serde_json::from_slice(body).map_err(|_| Rejected::NotJson)?;
        Ok(Self::fetched(guid.to_owned(), event.to_owned(), payload))
    }

    /// A delivery read from GitHub's delivery log over TLS with the owner's token.
    #[must_use]
    pub const fn fetched(guid: String, event: String, payload: Value) -> Self {
        Self {
            guid,
            event,
            payload,
        }
    }

    /// Its `X-GitHub-Delivery`.
    #[must_use]
    pub fn guid(&self) -> &str {
        &self.guid
    }

    /// Its event, with the payload's action when it has one: `pull_request.opened`, `push`.
    #[must_use]
    pub fn event(&self) -> String {
        match self.payload["action"].as_str() {
            Some(a) => format!("{}.{a}", self.event),
            None => self.event.clone(),
        }
    }

    /// The payload's `repository.full_name`.
    #[must_use]
    pub fn repo(&self) -> &str {
        self.payload["repository"]["full_name"]
            .as_str()
            .unwrap_or_default()
    }

    /// The payload's value at dotted `path` as text: a string as it is, others as JSON, a missing one empty.
    fn at(&self, path: &str) -> String {
        match self
            .payload
            .pointer(&format!("/{}", path.replace('.', "/")))
        {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
        }
    }
}

/// Hex `h` as bytes.
fn unhex(h: &str) -> Option<Vec<u8>> {
    let pairs = h.as_bytes().chunks(2);
    pairs
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).ok()?, 16).ok())
        .collect()
}

/// `<data>/routines.json`: the routines and the deliveries seen.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Book {
    #[serde(default)]
    format: u32,
    /// The routines.
    pub routines: Vec<Routine>,
    /// Each delivery's guid seen, by the hook or the catch-up, and when (ms).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seen: Vec<(String, u64)>,
}

impl Book {
    /// Records delivery `guid` seen at `now`, forgetting those older than four days: whether it is new.
    pub fn see(&mut self, guid: &str, now: u64) -> bool {
        self.seen.retain(|(_, at)| now.saturating_sub(*at) < KEEP);
        let new = !self.seen.iter().any(|(g, _)| g == guid);
        if new {
            self.seen.push((guid.to_owned(), now));
        }
        new
    }
}

/// The routines' file.
#[derive(Debug, Clone)]
pub struct Routines {
    path: PathBuf,
}

impl Routines {
    /// The routines in data folder `data`.
    #[must_use]
    pub fn new(data: &Path) -> Self {
        Self {
            path: data.join("routines.json"),
        }
    }

    /// Every routine and delivery seen (none before the first).
    ///
    /// # Errors
    /// When the file can't be read, isn't a routines file, or is a newer format.
    pub fn load(&self) -> io::Result<Book> {
        let bytes = match std::fs::read(&self.path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Book::default()),
            read => read?,
        };
        let f: Book = serde_json::from_slice(&bytes)?;
        if f.format > crate::FORMAT {
            return Err(io::Error::other(format!(
                "{} is format {}, newer than this cooee",
                self.path.display(),
                f.format
            )));
        }
        Ok(f)
    }

    /// Writes `book` whole (write-temp, fsync, rename).
    ///
    /// # Errors
    /// When it can't be written.
    pub fn save(&self, book: &mut Book) -> io::Result<()> {
        book.format = crate::FORMAT;
        crate::write_private(&self.path, &serde_json::to_vec_pretty(book)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;
    /// 2024-11-11 00:00 UTC, a Monday.
    const MON: u64 = 1_731_283_200_000;

    fn ask(when: &str, place: &str) -> Ask {
        Ask {
            name: " Monday renewals ".to_owned(),
            when: Some(when.to_owned()),
            github: None,
            prompt: "Renewals due {{due}}?".to_owned(),
            place: place.to_owned(),
        }
    }

    fn chat() -> Thread {
        Thread {
            id: "t-1".to_owned(),
            agent: "fake-acp".to_owned(),
            model: "m".to_owned(),
            ..Thread::default()
        }
    }

    #[test]
    fn a_routine_is_due_once_per_due_time_after_it_was_made() {
        let made = MON - 3 * 1440 * MIN;
        let mut r = Routine::make(&[], ask("0 9 * * 1", ""), &chat(), made).unwrap();
        assert_eq!(
            (r.name.as_str(), r.when().as_str(), &r.place),
            (
                "Monday renewals",
                "Mondays 09:00",
                &Place::Chat {
                    thread: "t-1".to_owned()
                }
            )
        );
        let nine = MON + 9 * 60 * MIN;
        assert_eq!(r.due(nine - MIN, 0), None);
        assert_eq!(r.due(nine + 5 * MIN, 0), Some(nine));
        assert_eq!(
            r.text(&Fields::Due(nine, 0)),
            "Renewals due 2024-11-11 09:00?"
        );
        r.last_due = nine;
        assert_eq!(r.due(nine + 5 * MIN, 0), None);
        // Asleep a fortnight: the two missed Mondays run once, as the newest.
        assert_eq!(
            r.due(nine + 14 * 1440 * MIN, 0),
            Some(nine + 14 * 1440 * MIN)
        );
        assert_eq!(r.key(nine), format!("routine-{}-{nine}", r.id));
    }

    #[test]
    fn make_and_edit_refuse_what_is_wrong_and_change_nothing() {
        let list = vec![Routine::make(&[], ask("0 9 * * 1", ""), &chat(), MON).unwrap()];
        let other = Routine::make(&list, ask("0 9 * * 1", "new_thread"), &chat(), MON).unwrap();
        assert_ne!(other.id, list[0].id);
        let Place::NewThread { who } = &other.place else {
            panic!("{:?}", other.place)
        };
        assert_eq!((who.agent.as_str(), who.model.as_str()), ("fake-acp", "m"));
        for (a, why) in [
            (ask("*/5 * * * *", ""), "BAD_SCHEDULE"),
            (ask("0 9 * * 1", "somewhere"), "BAD_PLACE"),
            (
                Ask {
                    name: " ".to_owned(),
                    ..ask("0 9 * * 1", "")
                },
                "BAD_NAME",
            ),
        ] {
            let e = Routine::make(&list, a, &chat(), MON).unwrap_err();
            assert!(e.starts_with(why), "{e}");
        }
        let full = vec![list[0].clone(); MAX];
        let e = Routine::make(&full, ask("0 9 * * 1", ""), &chat(), MON).unwrap_err();
        assert!(e.starts_with("TOO_MANY"), "{e}");
        let mut r = list[0].clone();
        let bad = Edit {
            name: Some("x".repeat(61)),
            ..Edit::default()
        };
        assert!(r.edit(bad, MON + 1).is_err());
        assert_eq!(r, list[0]);
        let e = Edit {
            when: Some("30 18 * * *".to_owned()),
            paused: Some(true),
            ..Edit::default()
        };
        r.edit(e, MON + 1).unwrap();
        assert_eq!(
            (r.when().as_str(), r.last_due),
            ("every day 18:30", MON + 1)
        );
        assert_eq!(r.paused, Some(Paused::Owner));
    }

    #[test]
    fn the_file_round_trips_and_refuses_a_newer_format() {
        let dir = std::env::temp_dir().join(format!("cooee-routines-{}", std::process::id()));
        let file = Routines::new(&dir);
        assert_eq!(file.load().unwrap().routines, Vec::new());
        let mut r = Routine::make(&[], ask("0 9 * * 1", ""), &chat(), MON).unwrap();
        r.last_run = Some(Run {
            at: MON,
            key: r.key(MON),
            outcome: Outcome::Ran {
                thread: "t-1".to_owned(),
            },
        });
        let mut book = Book {
            routines: vec![r.clone()],
            ..Book::default()
        };
        assert!(book.see("g-1", MON) && !book.see("g-1", MON + 1));
        file.save(&mut book).unwrap();
        let text = std::fs::read_to_string(dir.join("routines.json")).unwrap();
        assert!(text.contains("\"schedule\": \"0 9 * * 1\""), "{text}");
        assert!(text.contains("\"outcome\": \"ran\""), "{text}");
        let back = file.load().unwrap();
        assert_eq!(
            (back.routines, back.seen),
            (vec![r], vec![("g-1".to_owned(), MON)])
        );
        std::fs::write(dir.join("routines.json"), r#"{"format":9,"routines":[]}"#).unwrap();
        assert!(file.load().is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_signature_checks_against_githubs_documented_test_vector() {
        let (secret, body) = (b"It's a Secret to Everybody", b"Hello, World!");
        let sig = "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17";
        let headers = |sig| ("ping", "g-1", sig);
        let e = Delivery::signed(headers(sig), body, secret).unwrap_err();
        assert!(matches!(e, Rejected::NotJson), "the signature passed: {e}");
        for bad in [&sig.replace("757", "758"), "sha256=75", "", &sig[7..]] {
            let e = Delivery::signed(headers(bad), body, secret).unwrap_err();
            assert_eq!(e.status(), 401, "{bad}");
        }
        let e = Delivery::signed(("", "g-1", sig), body, secret).unwrap_err();
        assert_eq!(e.status(), 400);
    }

    #[test]
    fn an_event_matches_its_repo_events_and_match_fills_the_prompt_and_is_rate_capped() {
        let github: Github = serde_json::from_value(serde_json::json!({
            "repo": "Owner/App", "events": ["pull_request.opened", "push"],
            "match": { "pull_request.draft": "false" } }))
        .unwrap();
        let ask = Ask {
            when: None,
            github: Some(github),
            prompt: "{{event}} on {{repo}}: {{pull_request.title}} #{{ pull_request.number }}{{nope}} {{x".to_owned(),
            ..ask("", "")
        };
        let e = Routine::make(&[], ask.clone(), &chat(), MON).unwrap_err();
        assert!(e.starts_with("BAD_PROMPT"), "{e}");
        let ask = Ask {
            prompt: ask.prompt.replace(" {{x", ""),
            ..ask
        };
        let mut r = Routine::make(&[], ask.clone(), &chat(), MON).unwrap();
        assert_eq!(
            (r.when().as_str(), r.due(MON * 2, 0)),
            ("on pull_request.opened, push in Owner/App", None)
        );
        let pr = |action: &str, draft: bool, title: &str| {
            let payload = serde_json::json!({ "action": action, "repository": { "full_name": "owner/app" },
                "pull_request": { "title": title, "number": 7, "draft": draft } });
            Delivery::fetched("g".to_owned(), "pull_request".to_owned(), payload)
        };
        let d = pr("opened", false, "Fix login");
        assert!(r.matches(&d));
        assert!(!r.matches(&pr("closed", false, "x")) && !r.matches(&pr("opened", true, "x")));
        assert_eq!(
            r.text(&Fields::Event(&d)),
            "pull_request.opened on owner/app: Fix login #7"
        );
        let long = r.text(&Fields::Event(&pr("opened", false, &"t".repeat(3000))));
        assert_eq!(long.chars().filter(|c| *c == 't').count(), 2000 + 1);
        assert!(long.contains("t…"), "{long}");
        for at in 0..RATE as u64 {
            assert_eq!(r.admit(MON + at), Ok(()));
        }
        assert_eq!(r.admit(MON + 10), Err(Skip::Rate));
        assert_eq!(r.admit(MON + HOUR), Ok(()), "an hour on, it runs again");
        r.paused = Some(Paused::Owner);
        assert_eq!(r.admit(MON + 2 * HOUR), Err(Skip::Paused));
        for (bad, why) in [(Some("x"), "BAD_GITHUB"), (None, "BAD_TRIGGER")] {
            let mut a = ask.clone();
            if let Some(repo) = bad {
                a.github.as_mut().unwrap().repo = repo.to_owned();
            } else {
                a.when = Some("0 9 * * 1".to_owned());
            }
            let e = Routine::make(&[], a, &chat(), MON).unwrap_err();
            assert!(e.starts_with(why), "{e}");
        }
    }
}
