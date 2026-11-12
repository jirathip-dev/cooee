//! Routines (#75): a schedule that posts a prompt to an agent, made by the agent in chat and kept in
//! `<data>/routines.json` with its run state. Pure: the server's runner gives `now` and the Mac's UTC offset.

mod cron;

use std::{
    io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

pub use cron::{Schedule, local};

use crate::Thread;

/// How many routines there may be.
pub const MAX: usize = 50;

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
    /// What each run says; `{{due}}` is the due time, local `YYYY-MM-DD HH:MM`.
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
}

/// When a routine runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    /// A five-field cron in the Mac's local time.
    Schedule(Schedule),
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
}

/// `create_routine`'s arguments.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ask {
    /// The name.
    pub name: String,
    /// The cron.
    pub when: String,
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
    /// `BAD_SCHEDULE`, `BAD_NAME`, `BAD_PROMPT`, `BAD_PLACE` or `TOO_MANY`, said for the agent.
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
        let mut stamp = now;
        while list.iter().any(|r| r.id == format!("r-{stamp}")) {
            stamp += 1;
        }
        let mut r = Self {
            id: format!("r-{stamp}"),
            name: String::new(),
            trigger: Trigger::Schedule(Schedule::parse(&ask.when)?),
            place,
            prompt: String::new(),
            paused: None,
            made_in: from.id.clone(),
            made_at: now,
            last_due: 0,
            last_run: None,
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
        if prompt.as_ref().is_some_and(String::is_empty) {
            return Err("BAD_PROMPT: a prompt can't be empty".to_owned());
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

    /// Its schedule in words.
    #[must_use]
    pub fn when(&self) -> String {
        let Trigger::Schedule(s) = &self.trigger;
        s.describe()
    }

    /// The due time to run at `now` (ms, in UTC offset `offset_min`): the newest one past both its last run and its
    /// making, so missed ones run once, late.
    #[must_use]
    pub fn due(&self, now: u64, offset_min: i32) -> Option<u64> {
        let Trigger::Schedule(s) = &self.trigger;
        s.latest(now, offset_min)
            .filter(|due| *due > self.last_due.max(self.made_at))
    }

    /// The run of due time `due`'s key: the hub's `random_id`, so a run again adopts the first.
    #[must_use]
    pub fn key(&self, due: u64) -> String {
        format!("routine-{}-{due}", self.id)
    }

    /// The prompt for due time `due`.
    #[must_use]
    pub fn text(&self, due: u64, offset_min: i32) -> String {
        self.prompt.replace("{{due}}", &local(due, offset_min))
    }
}

/// `<data>/routines.json`.
#[derive(Debug, Serialize, Deserialize)]
struct File {
    format: u32,
    routines: Vec<Routine>,
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

    /// Every routine (none before the first).
    ///
    /// # Errors
    /// When the file can't be read, isn't a routines file, or is a newer format.
    pub fn load(&self) -> io::Result<Vec<Routine>> {
        let bytes = match std::fs::read(&self.path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            read => read?,
        };
        let f: File = serde_json::from_slice(&bytes)?;
        if f.format > crate::FORMAT {
            return Err(io::Error::other(format!(
                "{} is format {}, newer than this cooee",
                self.path.display(),
                f.format
            )));
        }
        Ok(f.routines)
    }

    /// Writes `routines` whole (write-temp, fsync, rename).
    ///
    /// # Errors
    /// When it can't be written.
    pub fn save(&self, routines: &[Routine]) -> io::Result<()> {
        let f = File {
            format: crate::FORMAT,
            routines: routines.to_vec(),
        };
        crate::write_private(&self.path, &serde_json::to_vec_pretty(&f)?)
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
            when: when.to_owned(),
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
        assert_eq!(r.text(nine, 0), "Renewals due 2024-11-11 09:00?");
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
        assert_eq!(file.load().unwrap(), Vec::new());
        let mut r = Routine::make(&[], ask("0 9 * * 1", ""), &chat(), MON).unwrap();
        r.last_run = Some(Run {
            at: MON,
            key: r.key(MON),
            outcome: Outcome::Ran {
                thread: "t-1".to_owned(),
            },
        });
        file.save(std::slice::from_ref(&r)).unwrap();
        let text = std::fs::read_to_string(dir.join("routines.json")).unwrap();
        assert!(text.contains("\"schedule\": \"0 9 * * 1\""), "{text}");
        assert!(text.contains("\"outcome\": \"ran\""), "{text}");
        assert_eq!(file.load().unwrap(), vec![r]);
        std::fs::write(dir.join("routines.json"), r#"{"format":9,"routines":[]}"#).unwrap();
        assert!(file.load().is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
