use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File},
    io::{self, Write},
    os::unix::fs::FileExt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, PoisonError},
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// The on-disk format version this build writes. A file with a newer one is refused, not guessed at.
pub const FORMAT: u32 = 1;

/// A conversation with one agent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Thread {
    /// The thread's id: 1 to 64 of `A-Z a-z 0-9 - _` (it names the file).
    pub id: String,
    /// The title shown in the thread list.
    pub title: String,
    /// The messages, oldest first (0.1.0; a chat's rows are its items, `<id>.jsonl`).
    pub messages: Vec<Message>,
    /// The ACP agent (`shed agents` id).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub agent: String,
    /// The working folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<PathBuf>,
    /// The provider account its agent runs with, by name (empty: the system default, or no provider's agent; before
    /// #48 its label, `System default` for that one).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub account: String,
    /// Its agent profile's id (empty: Plain), fixed for its life.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub profile: String,
    /// The shed terminal's name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub terminal: String,
    /// The model its agent runs on (#553 row 34; empty: the agent's default).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
    /// The effort, as `model`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub effort: String,
    /// The machine it runs on (#148): its persona's, or the one picked at its start; empty: this host.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub machine: String,
    /// The newest item id the owner has seen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read: Option<u64>,
    /// The thread was closed: its terminal is killed and removed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub closed: bool,
    /// The page's id for the start that made it, so a retried start adopts this thread instead of starting another.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub random_id: String,
    /// Archived: listed apart, under the thread list's archived threads, until restored.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub archived: bool,
    /// The owner's messages queued during a turn, sent in order as each turn ends.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub queue: Vec<Queued>,
    /// Where a session a tool started came from (a Workflow item, #33); none: the owner's own chat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<Workflow>,
    /// Sent ahead of the next message after a folder was attached (#86): the agent started again there.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub handover: String,
    /// A chat, or a room (#154).
    #[serde(default, skip_serializing_if = "Kind::is_chat")]
    pub kind: Kind,
    /// A room's members, in room order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<Member>,
    /// A room's turns still to take on the owner's last message, in order: the first is the one taking it (#155).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub round: Vec<RoomTurn>,
    /// How many member turns the owner's last message has started (at most 6, G2).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub taken: u8,
    /// When the owner's last message in a room was sent (ms): its round stops 10 minutes on (G2).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub asked_at: u64,
    /// A member's session: the room it is a member of (hidden from the thread list).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub room: String,
    /// A linked exchange (#156): the thread whose persona asked this one's (`ask_persona`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub asked_by: String,
}

/// What a [`Thread`] is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A conversation with one agent.
    #[default]
    Chat,
    /// The owner with 2 to 5 member personas, each in its own session.
    Room,
}

impl Kind {
    fn is_chat(&self) -> bool {
        *self == Self::Chat
    }
}

fn is_zero<T: Default + PartialEq>(n: &T) -> bool {
    *n == T::default()
}

/// A room member's turn (#155): whose, and why it has one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomTurn {
    /// The member's persona id.
    pub member: String,
    /// Why it has the turn.
    pub reason: Reason,
}

/// Why a room member has a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reason {
    /// The owner's message named it.
    Mention,
    /// The owner's message named no one: every member gets one turn, in room order (G1).
    Round,
    /// Another member's reply named it.
    Chained,
}

/// A room's member: its persona, its session (a hidden thread, empty until its first turn) and the newest room
/// item it was given.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    /// The persona's id.
    pub persona: String,
    /// Its session's thread id.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub session: String,
    /// The newest room item id its session was given.
    #[serde(default)]
    pub heard: u64,
}

/// A message queued during a turn.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Queued {
    /// The text.
    pub text: String,
    /// The page's id for the send: the item it becomes keeps it.
    pub random_id: String,
    /// Its files' stored names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
}

/// A session a tool started, as its shed labels say (#33): `source` (the tool), `group` (its repo), `item`
/// (`#issue`), `role`, `round` and an optional `url`. Any tool may set them; cooee names none.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workflow {
    /// The tool that started it.
    pub source: String,
    /// What it works on (a repo).
    #[serde(default)]
    pub group: String,
    /// Which item of it (`#33`).
    #[serde(default)]
    pub item: String,
    /// Its role (`impl`, `rev`, …).
    #[serde(default)]
    pub role: String,
    /// Its round.
    #[serde(default)]
    pub round: String,
    /// A link to what it works on.
    #[serde(default)]
    pub url: String,
}

impl Workflow {
    /// The session's labels as a Workflow item: one with a `source` that cooee didn't start itself.
    #[must_use]
    pub fn from_labels(labels: &BTreeMap<String, String>) -> Option<Self> {
        if labels.keys().any(|k| k.starts_with("cooee")) {
            return None;
        }
        let get = |k: &str| labels.get(k).cloned().unwrap_or_default();
        let source = get("source");
        (!source.is_empty()).then(|| Self {
            source,
            group: get("group"),
            item: get("item"),
            role: get("role"),
            round: get("round"),
            url: get("url"),
        })
    }
}

/// One message in a thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    /// Who wrote it.
    pub role: Role,
    /// The message text.
    pub text: String,
}

/// The author of a [`Message`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// The owner.
    User,
    /// The coding agent.
    Agent,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored<T> {
    format: u32,
    thread: T,
}

/// Why a store operation failed.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// A file or folder couldn't be read or written.
    #[error("{path}: {source}")]
    Io {
        /// The file or folder.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// A thread file isn't a valid thread.
    #[error("{path}: {source}")]
    Parse {
        /// The thread file.
        path: PathBuf,
        /// The underlying error.
        source: serde_json::Error,
    },
    /// A thread file was written by a newer cooee.
    #[error("{path}: format {found} is newer than this cooee's {FORMAT}; upgrade cooee")]
    Newer {
        /// The thread file.
        path: PathBuf,
        /// Its format version.
        found: u32,
    },
    /// A thread id that can't name a file.
    #[error("invalid thread id {0:?}: use 1 to 64 of A-Z a-z 0-9 - _")]
    InvalidId(String),
}

/// The thread store in one folder.
#[derive(Debug)]
pub struct Store {
    threads: PathBuf,
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl Store {
    /// Opens the store in `dir`, creating `dir/threads/` if needed.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Io`] if the folder can't be created.
    pub fn open(dir: &Path) -> Result<Self, StoreError> {
        let threads = dir.join("threads");
        fs::create_dir_all(&threads).map_err(io_err(&threads))?;
        Ok(Self {
            threads,
            locks: Mutex::default(),
        })
    }

    /// The ids of every stored thread, sorted.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Io`] if the folder can't be read.
    pub fn ids(&self) -> Result<Vec<String>, StoreError> {
        let mut ids = Vec::new();
        for entry in fs::read_dir(&self.threads).map_err(io_err(&self.threads))? {
            let path = entry.map_err(io_err(&self.threads))?.path();
            if path.extension().is_some_and(|e| e == "json")
                && let Some(id) = path.file_stem().and_then(|s| s.to_str())
            {
                ids.push(id.to_owned());
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// Reads thread `id`.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the id is invalid, or the file can't be read, isn't a thread or is from a newer cooee.
    pub fn load(&self, id: &str) -> Result<Thread, StoreError> {
        let path = self.path(id)?;
        let bytes = fs::read(&path).map_err(io_err(&path))?;
        let parse = |source| StoreError::Parse {
            path: path.clone(),
            source,
        };
        let Stored { format, .. } =
            serde_json::from_slice::<Stored<serde::de::IgnoredAny>>(&bytes).map_err(parse)?;
        if format > FORMAT {
            return Err(StoreError::Newer {
                path,
                found: format,
            });
        }
        Ok(serde_json::from_slice::<Stored<Thread>>(&bytes)
            .map_err(parse)?
            .thread)
    }

    /// Writes `thread` atomically, replacing any earlier version.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the id is invalid or the file can't be written.
    pub fn save(&self, thread: &Thread) -> Result<(), StoreError> {
        let lock = self.lock(&thread.id);
        let _held = lock.lock().unwrap_or_else(PoisonError::into_inner);
        let (tmp, path) = self.stage(thread)?;
        fs::rename(&tmp, &path).map_err(io_err(&path))?;
        File::open(&self.threads)
            .and_then(|dir| dir.sync_all())
            .map_err(io_err(&self.threads))
    }

    fn stage(&self, thread: &Thread) -> Result<(PathBuf, PathBuf), StoreError> {
        let path = self.path(&thread.id)?;
        let mut bytes = serde_json::to_vec_pretty(&Stored {
            format: FORMAT,
            thread,
        })
        .map_err(|source| StoreError::Parse {
            path: path.clone(),
            source,
        })?;
        bytes.push(b'\n');
        let tmp = path.with_extension("json.tmp");
        let mut file = File::create(&tmp).map_err(io_err(&tmp))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(io_err(&tmp))?;
        Ok((tmp, path))
    }

    /// Removes thread `id`'s files (a start that never ran); files already gone are fine.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the id is invalid or a file can't be removed.
    pub fn remove(&self, id: &str) -> Result<(), StoreError> {
        let path = self.path(id)?;
        let lock = self.lock(id);
        let _held = lock.lock().unwrap_or_else(PoisonError::into_inner);
        for file in [path.with_extension("jsonl"), path] {
            match fs::remove_file(&file) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(io_err(&file)(e)),
                _ => {}
            }
        }
        Ok(())
    }

    /// Appends `item` as one line to thread `id`'s items and fsyncs it.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the id is invalid or the line can't be written.
    pub fn append<T: Serialize>(&self, id: &str, item: &T) -> Result<(), StoreError> {
        let path = self.path(id)?.with_extension("jsonl");
        let mut line = serde_json::to_vec(item).map_err(|source| StoreError::Parse {
            path: path.clone(),
            source,
        })?;
        line.push(b'\n');
        let lock = self.lock(id);
        let _held = lock.lock().unwrap_or_else(PoisonError::into_inner);
        let mut file = File::options()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)
            .map_err(io_err(&path))?;
        let len = file.metadata().map_err(io_err(&path))?.len();
        let mut last = *b"\n";
        if len > 0 {
            file.read_exact_at(&mut last, len - 1)
                .map_err(io_err(&path))?;
        }
        if last[0] != b'\n' {
            line.insert(0, b'\n');
        }
        file.write_all(&line)
            .and_then(|()| file.sync_data())
            .map_err(io_err(&path))
    }

    /// Thread `id`'s item lines, oldest first; none when it has no items file. A line that doesn't parse (the torn
    /// end of a crashed append) is skipped.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the id is invalid or the file can't be read.
    pub fn items<T: DeserializeOwned>(&self, id: &str) -> Result<Vec<T>, StoreError> {
        self.items_if(id, |_| true)
    }

    /// [`Store::items`], or none when `keep` says no to the whole file's bytes, read once and never parsed then.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the id is invalid or the file can't be read.
    pub fn items_if<T: DeserializeOwned>(
        &self,
        id: &str,
        keep: impl FnOnce(&[u8]) -> bool,
    ) -> Result<Vec<T>, StoreError> {
        let path = self.path(id)?.with_extension("jsonl");
        match fs::read(&path) {
            Ok(bytes) if !keep(&bytes) => Ok(Vec::new()),
            Ok(bytes) => Ok(bytes
                .split(|b| *b == b'\n')
                .filter_map(|line| serde_json::from_slice(line).ok())
                .collect()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(source) => Err(StoreError::Io { path, source }),
        }
    }

    fn lock(&self, id: &str) -> Arc<Mutex<()>> {
        Arc::clone(
            self.locks
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .entry(id.to_owned())
                .or_default(),
        )
    }

    fn path(&self, id: &str) -> Result<PathBuf, StoreError> {
        let valid = (1..=64).contains(&id.len())
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
        if !valid {
            return Err(StoreError::InvalidId(id.to_owned()));
        }
        Ok(self.threads.join(format!("{id}.json")))
    }
}

fn io_err(path: &Path) -> impl FnOnce(io::Error) -> StoreError + '_ {
    move |source| StoreError::Io {
        path: path.to_owned(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env,
        process::{Command, Stdio},
        thread,
    };

    fn scratch(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("cooee-store-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn sample(id: &str, title: &str) -> Thread {
        let text = "สวัสดีครับ ".repeat(200);
        Thread {
            id: id.into(),
            title: title.into(),
            messages: vec![Message {
                role: Role::User,
                text,
            }],
            ..Thread::default()
        }
    }

    #[test]
    fn save_then_load_round_trips_and_leaves_no_temp() {
        let dir = scratch("round-trip");
        let store = Store::open(&dir).unwrap();
        store.save(&sample("a", "first")).unwrap();
        store.save(&sample("a", "second")).unwrap();
        assert_eq!(store.load("a").unwrap(), sample("a", "second"));
        assert_eq!(store.ids().unwrap(), ["a"]);
        assert!(!dir.join("threads/a.json.tmp").exists());
    }

    #[test]
    fn item_lines_append_and_a_torn_last_line_is_skipped() {
        let dir = scratch("items");
        let store = Store::open(&dir).unwrap();
        assert_eq!(store.items::<u32>("a").unwrap(), Vec::<u32>::new());
        store.append("a", &1).unwrap();
        store.append("a", &2).unwrap();
        let mut file = File::options()
            .append(true)
            .open(dir.join("threads/a.jsonl"))
            .unwrap();
        file.write_all(b"{\"torn").unwrap();
        assert_eq!(store.items::<u32>("a").unwrap(), [1, 2]);
        store.append("a", &3).unwrap();
        assert_eq!(store.items::<u32>("a").unwrap(), [1, 2, 3]);
        assert!(store.append("../x", &3).is_err());
        assert_eq!(store.ids().unwrap(), Vec::<String>::new());
    }

    #[test]
    fn bad_ids_and_newer_formats_are_refused() {
        let dir = scratch("refused");
        let store = Store::open(&dir).unwrap();
        for id in ["", "../x", "a/b", "a.json", &"x".repeat(65)] {
            assert!(
                matches!(store.save(&sample(id, "t")), Err(StoreError::InvalidId(_))),
                "{id:?}"
            );
        }
        fs::write(
            dir.join("threads/n.json"),
            r#"{"format":2,"thread":{"id":"n","future":true}}"#,
        )
        .unwrap();
        assert!(matches!(
            store.load("n"),
            Err(StoreError::Newer { found: 2, .. })
        ));
    }

    #[test]
    fn concurrent_saves_of_one_thread_never_tear() {
        let dir = scratch("concurrent");
        let store = Store::open(&dir).unwrap();
        thread::scope(|s| {
            for i in 0..8 {
                let store = &store;
                s.spawn(move || {
                    for j in 0..25 {
                        store.save(&sample("c", &format!("{i}-{j}"))).unwrap();
                    }
                });
            }
        });
        assert_eq!(store.load("c").unwrap().messages, sample("c", "").messages);
    }

    #[test]
    fn crash_between_temp_write_and_rename_keeps_old_file() {
        let dir = scratch("crash");
        let store = Store::open(&dir).unwrap();
        store.save(&sample("a", "old")).unwrap();
        let status = Command::new(env::current_exe().unwrap())
            .args([
                "--exact",
                "store::tests::crash_child",
                "--ignored",
                "--test-threads=1",
            ])
            .env("COOEE_CRASH_DIR", &dir)
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(!status.success(), "the child must die mid-write");
        assert!(
            dir.join("threads/a.json.tmp").exists(),
            "the child got as far as the temp file"
        );
        assert_eq!(store.load("a").unwrap(), sample("a", "old"));
        assert_eq!(store.ids().unwrap(), ["a"]);
        store.save(&sample("a", "new")).unwrap();
        assert_eq!(store.load("a").unwrap(), sample("a", "new"));
    }

    #[test]
    #[ignore = "the child process of crash_between_temp_write_and_rename_keeps_old_file"]
    fn crash_child() {
        let Some(dir) = env::var_os("COOEE_CRASH_DIR") else {
            return;
        };
        let store = Store::open(Path::new(&dir)).unwrap();
        store.stage(&sample("a", "new")).unwrap();
        std::process::abort();
    }
}
