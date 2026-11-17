//! Routines (#75, #261, #499): a schedule, a GitHub event or a signed POST that posts a prompt to an agent, made by the agent in chat.
//! Each one's settings are `<config>/routines.toml`, its run state and the deliveries seen `<data>/routine-runs.json`
//! (#359). Pure: the server's runner gives `now`, the Mac's UTC offset and each delivery.
mod cron;
mod github;
mod webhook;

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

pub use cron::{Schedule, local};
pub use github::{Delivery, Fields, Github, Hooks, Rejected, Skip};
pub use webhook::{Inbound, Webhook};

use crate::Thread;

/// How many routines there may be.
pub const MAX: usize = 50;

/// How many times an event may run one routine in an hour.
pub const RATE: usize = 6;

const RUNS_FORMAT: u32 = 1;

const HOUR: u64 = 3_600_000;

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
    /// A signed POST to `/hooks/in/<id>`.
    Webhook(Webhook),
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
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

/// `create_routine`'s arguments: `when`, `github` or `webhook`.
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
    /// The signed POST.
    pub webhook: Option<Webhook>,
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
    /// `BAD_SCHEDULE`, `BAD_GITHUB`, `BAD_WEBHOOK`, `BAD_TRIGGER`, `BAD_NAME`, `BAD_PROMPT`, `BAD_PLACE` or `TOO_MANY`, said for
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
        let trigger = match (ask.when, ask.github, ask.webhook) {
            (Some(w), None, None) => Trigger::Schedule(Schedule::parse(&w)?),
            (None, Some(g), None) => Trigger::Github(g.checked()?),
            (None, None, Some(h)) => Trigger::Webhook(h.checked()?),
            _ => return Err("BAD_TRIGGER: give one of when, github or webhook".to_owned()),
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
            Trigger::Webhook(h) => format!("on a POST signed with secret:{}", h.secret),
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
    /// the event's payload (`pull_request.title`), each at most 2,000 characters; then a POST's untrusted body.
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
        out + rest + &f.data()
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

/// The routines and the deliveries seen.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Book {
    /// The routines.
    pub routines: Vec<Routine>,
    /// Each delivery's guid seen, by the hook or the catch-up, and when (ms).
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

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    #[serde(default)]
    routine: Vec<Def>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Def {
    id: String,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    when: Option<Schedule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    github: Option<Github>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    webhook: Option<Webhook>,
    prompt: String,
    #[serde(default)]
    paused: bool,
    place: Place,
}

#[derive(Debug, Serialize, Deserialize)]
struct RunFile {
    format: u32,
    #[serde(default)]
    runs: BTreeMap<String, State>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    seen: Vec<(String, u64)>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct State {
    made_in: String,
    made_at: u64,
    last_due: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_run: Option<Run>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chat_gone: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    recent: Vec<u64>,
}

pub(crate) fn files(routines: &[Routine], seen: &[(String, u64)]) -> (String, Vec<u8>) {
    let mut settings = Settings::default();
    let mut runs = RunFile {
        format: RUNS_FORMAT,
        runs: BTreeMap::new(),
        seen: seen.to_vec(),
    };
    for r in routines {
        let (when, github, webhook) = match r.trigger.clone() {
            Trigger::Schedule(s) => (Some(s), None, None),
            Trigger::Github(g) => (None, Some(g), None),
            Trigger::Webhook(h) => (None, None, Some(h)),
        };
        let chat_gone = match &r.paused {
            Some(Paused::ChatGone { thread }) => Some(thread.clone()),
            _ => None,
        };
        settings.routine.push(Def {
            id: r.id.clone(),
            name: r.name.clone(),
            when,
            github,
            webhook,
            prompt: r.prompt.clone(),
            paused: r.paused == Some(Paused::Owner),
            place: r.place.clone(),
        });
        let state = State {
            made_in: r.made_in.clone(),
            made_at: r.made_at,
            last_due: r.last_due,
            last_run: r.last_run.clone(),
            chat_gone,
            recent: r.recent.clone(),
        };
        runs.runs.insert(r.id.clone(), state);
    }
    let toml = crate::toml_text(&serde_json::to_value(&settings).unwrap_or_default());
    (toml, serde_json::to_vec_pretty(&runs).unwrap_or_default())
}

fn read(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        read => read.map(Some),
    }
}

/// The routines' two files.
#[derive(Debug, Clone)]
pub struct Routines {
    settings: PathBuf,
    runs: PathBuf,
}

impl Routines {
    /// The routines of config folder `config`, their run state in data folder `data`.
    #[must_use]
    pub fn new(config: &Path, data: &Path) -> Self {
        Self {
            settings: config.join("routines.toml"),
            runs: data.join("routine-runs.json"),
        }
    }

    /// Every routine (none before the first), its settings joined with its run state, and the deliveries seen.
    ///
    /// # Errors
    /// When a file can't be read, isn't what it should be, or the run state is a newer format.
    pub fn load(&self) -> io::Result<Book> {
        let text = String::from_utf8(read(&self.settings)?.unwrap_or_default())
            .map_err(io::Error::other)?;
        let bad = |e: toml::de::Error| {
            io::Error::other(format!("{}: {}", self.settings.display(), e.message()))
        };
        let settings: Settings = toml::from_str(&text).map_err(bad)?;
        let runs: RunFile = match read(&self.runs)? {
            Some(bytes) => serde_json::from_slice(&bytes)?,
            None => RunFile {
                format: RUNS_FORMAT,
                runs: BTreeMap::new(),
                seen: Vec::new(),
            },
        };
        if runs.format > RUNS_FORMAT {
            return Err(io::Error::other(format!(
                "{} is format {}, newer than this cooee",
                self.runs.display(),
                runs.format
            )));
        }
        let mut routines = Vec::new();
        for d in settings.routine {
            let s = runs.runs.get(&d.id).cloned().unwrap_or_default();
            let gone = s.chat_gone.map(|thread| Paused::ChatGone { thread });
            let trigger = match (d.when, d.github, d.webhook) {
                (Some(w), None, None) => Trigger::Schedule(w),
                (None, Some(g), None) => Trigger::Github(g),
                (None, None, Some(h)) => Trigger::Webhook(h),
                _ => {
                    return Err(io::Error::other(format!(
                        "{}: routine {} needs when, github or webhook, one of them",
                        self.settings.display(),
                        d.id
                    )));
                }
            };
            routines.push(Routine {
                trigger,
                paused: if d.paused { Some(Paused::Owner) } else { gone },
                made_in: s.made_in,
                made_at: s.made_at,
                last_due: s.last_due,
                last_run: s.last_run,
                recent: s.recent,
                id: d.id,
                name: d.name,
                place: d.place,
                prompt: d.prompt,
            });
        }
        Ok(Book {
            routines,
            seen: runs.seen,
        })
    }

    /// Writes `book` (write-temp, fsync, rename), each file only when its part changed; `routines.toml` changes
    /// only in the keys that differ (#360), its comments and layout kept.
    ///
    /// # Errors
    /// When it can't be read or written.
    pub fn save(&self, book: &Book) -> io::Result<()> {
        let (toml, runs) = files(&book.routines, &book.seen);
        let text = String::from_utf8(read(&self.settings)?.unwrap_or_default())
            .map_err(io::Error::other)?;
        let was: Settings = toml::from_str(&text).map_err(io::Error::other)?;
        let was = crate::toml_text(&serde_json::to_value(&was).unwrap_or_default());
        let toml = crate::disk::patch(&text, &was, &toml)?;
        for (path, bytes) in [(&self.settings, toml.as_bytes()), (&self.runs, &runs[..])] {
            if read(path)?.as_deref() != Some(bytes) {
                crate::write_private(path, bytes)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;
    const MON: u64 = 1_731_283_200_000;

    fn ask(when: &str, place: &str) -> Ask {
        Ask {
            name: " Monday renewals ".to_owned(),
            when: Some(when.to_owned()),
            github: None,
            webhook: None,
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
        let dir = crate::scratch::owned(
            std::env::temp_dir().join(format!("cooee-routines-{}", std::process::id())),
        );
        let file = Routines::new(&dir, &dir);
        assert_eq!(file.load().unwrap(), Book::default());
        let mut r = Routine::make(&[], ask("0 9 * * 1", ""), &chat(), MON).unwrap();
        r.last_run = Some(Run {
            at: MON,
            key: r.key(MON),
            outcome: Outcome::Ran {
                thread: "t-1".to_owned(),
            },
        });
        let github = Ask {
            when: None,
            github: serde_json::from_value(serde_json::json!({
                "repo": "o/app", "events": ["push"], "match": { "ref": "refs/heads/main" } }))
            .unwrap(),
            ..ask("", "")
        };
        let mut g = Routine::make(&[], github, &chat(), MON + 1).unwrap();
        g.recent = vec![MON];
        let mut book = Book {
            routines: vec![r, g],
            ..Book::default()
        };
        assert!(book.see("g-1", MON) && !book.see("g-1", MON + 1));
        file.save(&book).unwrap();
        let text = std::fs::read_to_string(dir.join("routines.toml")).unwrap();
        assert!(text.contains("when = \"0 9 * * 1\""), "{text}");
        assert!(text.contains("[routine.github.match]"), "{text}");
        let runs = std::fs::read_to_string(dir.join("routine-runs.json")).unwrap();
        assert!(runs.contains("\"outcome\": \"ran\""), "{runs}");
        assert!(runs.contains("\"g-1\""), "{runs}");
        assert_eq!(file.load().unwrap(), book);
        std::fs::write(dir.join("routine-runs.json"), r#"{"format":9,"runs":{}}"#).unwrap();
        assert!(file.load().is_err());
        std::fs::remove_dir_all(&dir).unwrap();
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
        for at in 0..u64::try_from(RATE).unwrap() {
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
