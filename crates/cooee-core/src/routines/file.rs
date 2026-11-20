use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::{Book, Ended, Github, Paused, Place, Routine, Run, Schedule, Script, Trigger, Webhook};

const RUNS_FORMAT: u32 = 1;

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
    #[serde(default)]
    prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    script: Option<Script>,
    #[serde(default)]
    paused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ends_at: Option<u64>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    ended: Option<Ended>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    recent: Vec<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    heard: Option<u64>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    silent: bool,
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
        let (chat_gone, ended) = match &r.paused {
            Some(Paused::ChatGone { thread }) => (Some(thread.clone()), None),
            Some(Paused::Ended(e)) => (None, Some(*e)),
            _ => (None, None),
        };
        settings.routine.push(Def {
            id: r.id.clone(),
            name: r.name.clone(),
            when,
            github,
            webhook,
            prompt: r.prompt.clone(),
            script: r.script.clone(),
            paused: r.paused == Some(Paused::Owner),
            ends_at: r.ends_at,
            place: r.place.clone(),
        });
        let state = State {
            made_in: r.made_in.clone(),
            made_at: r.made_at,
            last_due: r.last_due,
            last_run: r.last_run.clone(),
            chat_gone,
            ended,
            recent: r.recent.clone(),
            heard: r.heard,
            silent: r.silent,
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
            let gone = s
                .ended
                .map(Paused::Ended)
                .or_else(|| s.chat_gone.map(|thread| Paused::ChatGone { thread }));
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
                ends_at: d.ends_at,
                made_in: s.made_in,
                made_at: s.made_at,
                last_due: s.last_due,
                last_run: s.last_run,
                recent: s.recent,
                heard: s.heard,
                silent: s.silent,
                id: d.id,
                name: d.name,
                place: d.place,
                prompt: d.prompt,
                script: d.script,
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
