//! The thread store: one typed JSON file per thread under `<dir>/threads/`, and every thread's items in one SQLite
//! file, `<dir>/cooee.db` (#221).
//!
//! An item is a row keyed by its thread and id, its body the JSON line the items files held; a later version replaces
//! the row. Its message text (the owner's and the agent's replies, never tool output) is in a trigram full-text index,
//! so any language's text matches. Every item write commits with `synchronous=FULL`. Every thread file write is
//! write-temp → fsync → rename (then fsync the folder), under the thread's lock, so a crash leaves either the old file
//! or the new one, never half of either. On open each `<id>.jsonl` items file of an earlier release is imported in one
//! transaction and renamed `<id>.jsonl.imported`; a torn last line is skipped. `tests/upgrade_compat.rs` reads every
//! earlier release's files from `tests/fixtures/upgrade/<version>/`.

use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::Duration,
};

use rusqlite::{Connection, Transaction, params};
use serde::{Deserialize, Serialize};

use crate::{Item, history};

/// The on-disk format version this build writes. A file with a newer one is refused, not guessed at.
pub const FORMAT: u32 = 2;

/// A conversation with one agent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Thread {
    /// The thread's id: 1 to 64 of `A-Z a-z 0-9 - _` (it names the file).
    pub id: String,
    /// The title shown in the thread list.
    pub title: String,
    /// The messages, oldest first (0.1.0; a chat's rows are its items, in `cooee.db`).
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
    /// The routine whose run started it (#75).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routine: Option<crate::routines::Mark>,
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
    /// The routine that sent it (#75).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routine: Option<crate::routines::Mark>,
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

/// Where a [`Store::page`] read starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// The newest items.
    Last,
    /// The newest items before this id.
    Before(u64),
    /// The oldest items after this id.
    After(u64),
}

/// A thread file as stored: the format version, then the thread.
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
    /// The items database failed.
    #[error("{path}: {source}")]
    Db {
        /// The database file.
        path: PathBuf,
        /// The underlying error.
        source: rusqlite::Error,
    },
    /// A thread id that can't name a file.
    #[error("invalid thread id {0:?}: use 1 to 64 of A-Z a-z 0-9 - _")]
    InvalidId(String),
}

/// The thread store in one folder.
#[derive(Debug)]
pub struct Store {
    threads: PathBuf,
    db: Mutex<Connection>,
    path: PathBuf,
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl Store {
    /// Opens the store in `dir`, creating `dir/threads/` and `dir/cooee.db` if needed, and imports any items files.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the folder or database can't be created, or an items file can't be imported.
    pub fn open(dir: &Path) -> Result<Self, StoreError> {
        let threads = dir.join("threads");
        fs::create_dir_all(&threads).map_err(io_err(&threads))?;
        let path = dir.join("cooee.db");
        let db = Connection::open(&path).map_err(db_err(&path))?;
        db.busy_timeout(Duration::from_secs(10))
            .and_then(|()| {
                db.pragma_update_and_check(None, "journal_mode", "wal", |r| r.get::<_, String>(0))
            })
            .and_then(|_| {
                db.execute_batch(
                    "PRAGMA synchronous = FULL;
                     CREATE TABLE IF NOT EXISTS items (thread TEXT NOT NULL, id INTEGER NOT NULL,
                         at INTEGER NOT NULL, kind TEXT NOT NULL, body TEXT NOT NULL, PRIMARY KEY (thread, id));
                     CREATE VIRTUAL TABLE IF NOT EXISTS said USING fts5(text, tokenize = 'trigram');",
                )
            })
            .map_err(db_err(&path))?;
        let store = Self {
            threads,
            db: Mutex::new(db),
            path,
            locks: Mutex::default(),
        };
        for entry in fs::read_dir(&store.threads).map_err(io_err(&store.threads))? {
            let file = entry.map_err(io_err(&store.threads))?.path();
            if file.extension().is_some_and(|e| e == "jsonl") {
                store.import(&file)?;
            }
        }
        Ok(store)
    }

    /// Imports the items file `file` in one transaction, then renames it `<id>.jsonl.imported`. Run again after a
    /// crash it imports the same rows again.
    fn import(&self, file: &Path) -> Result<(), StoreError> {
        let id = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let id = self
            .path(id)?
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let bytes = fs::read(file).map_err(io_err(file))?;
        let mut db = self.db();
        let tx = db.transaction().map_err(db_err(&self.path))?;
        for item in bytes
            .split(|b| *b == b'\n')
            .filter_map(|l| serde_json::from_slice(l).ok())
        {
            self.put(&tx, &id, &item)?;
        }
        tx.commit().map_err(db_err(&self.path))?;
        drop(db);
        let done = file.with_extension("jsonl.imported");
        fs::rename(file, &done).map_err(io_err(&done))?;
        File::open(&self.threads)
            .and_then(|dir| dir.sync_all())
            .map_err(io_err(&self.threads))
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

    /// The first half of [`Store::save`]: writes `thread` to `<id>.json.tmp` and fsyncs it. Returns the temp file
    /// and the file it replaces.
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
        let db = self.db();
        db.execute(
            "DELETE FROM said WHERE rowid IN (SELECT rowid FROM items WHERE thread = ?1)",
            [id],
        )
        .and_then(|_| db.execute("DELETE FROM items WHERE thread = ?1", [id]))
        .map_err(db_err(&self.path))?;
        for file in [path.with_extension("jsonl.imported"), path] {
            match fs::remove_file(&file) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(io_err(&file)(e)),
                _ => {}
            }
        }
        Ok(())
    }

    /// Stores `item` in thread `id`, replacing an earlier version of it, and commits it to disk.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the id is invalid or the item can't be written.
    pub fn append(&self, id: &str, item: &Item) -> Result<(), StoreError> {
        self.path(id)?;
        let mut db = self.db();
        let tx = db.transaction().map_err(db_err(&self.path))?;
        self.put(&tx, id, item)?;
        tx.commit().map_err(db_err(&self.path))
    }

    /// Writes `item`'s row and its message text's index row in `tx`.
    fn put(&self, tx: &Transaction, id: &str, item: &Item) -> Result<(), StoreError> {
        let body = serde_json::to_value(item).map_err(|source| StoreError::Parse {
            path: self.path.clone(),
            source,
        })?;
        let kind = body["kind"].as_str().unwrap_or_default();
        let row: i64 = tx
            .query_row(
                "INSERT INTO items (thread, id, at, kind, body) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (thread, id) DO UPDATE SET at = excluded.at, kind = excluded.kind, body = excluded.body
                 RETURNING rowid",
                params![id, int(item.id), int(item.at), kind, body.to_string()],
                |r| r.get(0),
            )
            .and_then(|row| tx.execute("DELETE FROM said WHERE rowid = ?1", [row]).map(|_| row))
            .map_err(db_err(&self.path))?;
        if let Some((_, text)) = history::said(item) {
            tx.execute(
                "INSERT INTO said (rowid, text) VALUES (?1, ?2)",
                params![row, text.to_lowercase()],
            )
            .map_err(db_err(&self.path))?;
        }
        Ok(())
    }

    /// Thread `id`'s items, oldest first; none when it has none.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the id is invalid or the items can't be read.
    pub fn items(&self, id: &str) -> Result<Vec<Item>, StoreError> {
        self.page(id, Page::Last, usize::MAX)
    }

    /// At most `n` of thread `id`'s items at `from`, oldest first.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the id is invalid or the items can't be read.
    pub fn page(&self, id: &str, from: Page, n: usize) -> Result<Vec<Item>, StoreError> {
        self.path(id)?;
        let (sql, at) = match from {
            Page::Last => ("id < ?3 ORDER BY id DESC", i64::MAX),
            Page::Before(b) => ("id < ?3 ORDER BY id DESC", int(b)),
            Page::After(a) => ("id > ?3 ORDER BY id", int(a)),
        };
        let n = u64::try_from(n).map_or(i64::MAX, int);
        let db = self.db();
        let mut items = db
            .prepare_cached(&format!(
                "SELECT body FROM items WHERE thread = ?1 AND {sql} LIMIT ?2"
            ))
            .and_then(|mut q| {
                q.query_map(params![id, n, at], |r| r.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(db_err(&self.path))?
            .iter()
            .map(|body| serde_json::from_str(body))
            .collect::<Result<Vec<Item>, _>>()
            .map_err(|source| StoreError::Parse {
                path: self.path.clone(),
                source,
            })?;
        if !matches!(from, Page::After(_)) {
            items.reverse();
        }
        Ok(items)
    }

    /// The items whose message text holds every one of `words` (lowercase), newest first, with their thread, until
    /// `each` says stop. A word is looked up in the trigram index; `each` sees a superset (a `%` or `_` in a word
    /// matches any text there), so it checks the text itself.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the index can't be read.
    pub fn said(
        &self,
        words: &[String],
        mut each: impl FnMut(&str, &Item) -> bool,
    ) -> Result<(), StoreError> {
        let like = " AND said.text LIKE ?".repeat(words.len());
        let sql = format!(
            "SELECT items.thread, items.body FROM said JOIN items ON items.rowid = said.rowid
             WHERE 1 {like} ORDER BY items.at DESC, items.id DESC"
        );
        let db = self.db();
        let mut q = db.prepare(&sql).map_err(db_err(&self.path))?;
        let patterns = words.iter().map(|w| format!("%{w}%"));
        let mut rows = q
            .query(rusqlite::params_from_iter(patterns))
            .map_err(db_err(&self.path))?;
        while let Some(row) = rows.next().map_err(db_err(&self.path))? {
            let (thread, body) = row
                .get::<_, String>(0)
                .and_then(|t| row.get::<_, String>(1).map(|b| (t, b)))
                .map_err(db_err(&self.path))?;
            if let Ok(item) = serde_json::from_str(&body)
                && !each(&thread, &item)
            {
                break;
            }
        }
        Ok(())
    }

    fn db(&self) -> MutexGuard<'_, Connection> {
        self.db.lock().unwrap_or_else(PoisonError::into_inner)
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

/// `n` as SQLite's integer (ids and times are far below its limit).
fn int(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

fn db_err(path: &Path) -> impl FnOnce(rusqlite::Error) -> StoreError + '_ {
    move |source| StoreError::Db {
        path: path.to_owned(),
        source,
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

    fn item(id: u64, kind: &str, text: &str) -> Item {
        let body = match kind {
            "you" => serde_json::json!({ "text": text, "random_id": "r" }),
            "agent" => serde_json::json!({ "text": text, "stop_reason": "end_turn" }),
            _ => serde_json::json!({ "call": "c", "title": text, "status": "completed" }),
        };
        let mut v = serde_json::json!({ "id": id, "at": id, "kind": kind });
        v.as_object_mut()
            .unwrap()
            .extend(body.as_object().unwrap().clone());
        serde_json::from_value(v).unwrap()
    }

    fn ids(items: &[Item]) -> Vec<u64> {
        items.iter().map(|i| i.id).collect()
    }

    /// Items come back by id, a later version replacing the row; a page reads the newest, or before or after an id.
    #[test]
    fn items_replace_by_id_and_page_from_either_end() {
        let dir = scratch("items");
        let store = Store::open(&dir).unwrap();
        assert_eq!(ids(&store.items("a").unwrap()), [0; 0]);
        for id in [1, 3, 2, 4, 5] {
            store.append("a", &item(id, "tool", "pending")).unwrap();
        }
        store.append("a", &item(2, "tool", "done")).unwrap();
        store.append("b", &item(1, "you", "other")).unwrap();
        let all = store.items("a").unwrap();
        assert_eq!(ids(&all), [1, 2, 3, 4, 5]);
        assert_eq!(all[1], item(2, "tool", "done"));
        assert_eq!(ids(&store.page("a", Page::Last, 2).unwrap()), [4, 5]);
        assert_eq!(ids(&store.page("a", Page::Before(4), 2).unwrap()), [2, 3]);
        assert_eq!(ids(&store.page("a", Page::After(1), 2).unwrap()), [2, 3]);
        assert_eq!(ids(&store.page("a", Page::After(5), 2).unwrap()), [0; 0]);
        store.remove("a").unwrap();
        assert_eq!(ids(&store.items("a").unwrap()), [0; 0]);
        assert_eq!(store.items("b").unwrap().len(), 1);
        assert!(store.append("../x", &item(1, "you", "x")).is_err());
    }

    /// Only the owner's messages and replies are indexed, never a tool's output; a Thai word finds a Thai message.
    #[test]
    fn the_index_holds_message_text_only_and_finds_thai() {
        let dir = scratch("index");
        let store = Store::open(&dir).unwrap();
        store
            .append("a", &item(1, "you", "ช่วยหาสูตรต้มยำกุ้ง"))
            .unwrap();
        store
            .append("a", &item(2, "tool", "cat ต้มยำ.txt secret"))
            .unwrap();
        store
            .append("a", &item(3, "agent", "สูตรต้มยำกุ้งน้ำข้น"))
            .unwrap();
        store
            .append("a", &item(3, "agent", "สูตรต้มยำกุ้งน้ำใส"))
            .unwrap();
        let found = |words: &[&str]| {
            let words: Vec<String> = words.iter().map(|w| (*w).to_owned()).collect();
            let mut got = Vec::new();
            store
                .said(&words, |t, i| {
                    got.push((t.to_owned(), i.id));
                    true
                })
                .unwrap();
            got
        };
        assert_eq!(found(&[]), [("a".to_owned(), 3), ("a".to_owned(), 1)]);
        assert_eq!(found(&["ต้มยำ"]), [("a".to_owned(), 3), ("a".to_owned(), 1)]);
        assert_eq!(found(&["น้ำใส"]), [("a".to_owned(), 3)]);
        assert!(
            found(&["น้ำข้น"]).is_empty(),
            "the replaced version left the index"
        );
        assert!(found(&["secret"]).is_empty(), "tool output is not indexed");
    }

    /// An earlier release's items file imports once with its versions folded and a torn last line skipped, and is
    /// renamed; opening again changes nothing.
    #[test]
    fn an_items_file_imports_once_and_skips_a_torn_line() {
        let dir = scratch("import");
        fs::create_dir_all(dir.join("threads")).unwrap();
        let lines: String = [
            item(1, "you", "hi"),
            item(2, "tool", "a"),
            item(2, "tool", "b"),
        ]
        .iter()
        .map(|i| serde_json::to_string(i).unwrap() + "\n")
        .collect();
        fs::write(dir.join("threads/a.jsonl"), lines + "{\"torn").unwrap();
        let want = [item(1, "you", "hi"), item(2, "tool", "b")];
        assert_eq!(Store::open(&dir).unwrap().items("a").unwrap(), want);
        assert!(!dir.join("threads/a.jsonl").exists());
        assert!(dir.join("threads/a.jsonl.imported").exists());
        let store = Store::open(&dir).unwrap();
        assert_eq!(store.items("a").unwrap(), want);
        store.remove("a").unwrap();
        assert!(!dir.join("threads/a.jsonl.imported").exists());
    }

    /// Runs this test binary's ignored `child` test with `dir` and waits for it to die.
    fn crash(child: &str, dir: &Path) {
        let status = Command::new(env::current_exe().unwrap())
            .args(["--exact", child, "--ignored", "--test-threads=1"])
            .env("COOEE_CRASH_DIR", dir)
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(!status.success(), "{child} must die mid-write");
    }

    fn crash_dir() -> Option<PathBuf> {
        env::var_os("COOEE_CRASH_DIR").map(PathBuf::from)
    }

    /// Killed mid-import (one file committed but not renamed, one half inserted): the items files are all still
    /// there, and the next open imports them cleanly with nothing doubled or lost.
    #[test]
    fn a_kill_mid_import_keeps_the_files_and_imports_again() {
        let dir = scratch("crash-import");
        fs::create_dir_all(dir.join("threads")).unwrap();
        for t in ["a", "b"] {
            let lines: String = (1..=50)
                .map(|n| serde_json::to_string(&item(n, "you", t)).unwrap() + "\n")
                .collect();
            fs::write(dir.join(format!("threads/{t}.jsonl")), lines).unwrap();
        }
        crash("store::tests::crash_import_child", &dir);
        assert!(dir.join("threads/a.jsonl").exists() && dir.join("threads/b.jsonl").exists());
        let store = Store::open(&dir).unwrap();
        for t in ["a", "b"] {
            let want: Vec<Item> = (1..=50).map(|n| item(n, "you", t)).collect();
            assert_eq!(store.items(t).unwrap(), want, "{t}");
        }
    }

    #[test]
    #[ignore = "the child process of a_kill_mid_import_keeps_the_files_and_imports_again"]
    fn crash_import_child() {
        let Some(dir) = crash_dir() else { return };
        // The files are set aside while the store opens, so this child imports them by hand and dies part way.
        let threads = dir.join("threads");
        let (a, b) = (threads.join("a.jsonl"), threads.join("b.jsonl"));
        let (aside_a, aside_b) = (dir.join("a.jsonl"), dir.join("b.jsonl"));
        fs::rename(&a, &aside_a).unwrap();
        fs::rename(&b, &aside_b).unwrap();
        let store = Store::open(&dir).unwrap();
        fs::rename(&aside_a, &a).unwrap();
        fs::rename(&aside_b, &b).unwrap();
        let mut db = store.db();
        let tx = db.transaction().unwrap();
        for n in 1..=50 {
            store.put(&tx, "a", &item(n, "you", "a")).unwrap();
        }
        tx.commit().unwrap();
        let tx = db.transaction().unwrap();
        for n in 1..=25 {
            store.put(&tx, "b", &item(n, "you", "b")).unwrap();
        }
        std::process::abort();
    }

    /// Killed in the middle of an append: every append that returned is there, the torn one is not.
    #[test]
    fn a_kill_mid_append_keeps_every_acknowledged_item() {
        let dir = scratch("crash-append");
        crash("store::tests::crash_append_child", &dir);
        let store = Store::open(&dir).unwrap();
        let want: Vec<Item> = (1..=100).map(|n| item(n, "agent", "kept")).collect();
        assert_eq!(store.items("a").unwrap(), want);
    }

    #[test]
    #[ignore = "the child process of a_kill_mid_append_keeps_every_acknowledged_item"]
    fn crash_append_child() {
        let Some(dir) = crash_dir() else { return };
        let store = Store::open(&dir).unwrap();
        for n in 1..=100 {
            store.append("a", &item(n, "agent", "kept")).unwrap();
        }
        let mut db = store.db();
        let tx = db.transaction().unwrap();
        store.put(&tx, "a", &item(101, "agent", "torn")).unwrap();
        std::process::abort();
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
            r#"{"format":3,"thread":{"id":"n","future":true}}"#,
        )
        .unwrap();
        assert!(matches!(
            store.load("n"),
            Err(StoreError::Newer { found: 3, .. })
        ));
    }

    /// Many writers of one thread: the lock keeps them from sharing the temp file, so every save succeeds and the
    /// file is always one whole version.
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

    /// The process is killed (SIGABRT) after the temp file is written and before the rename: the old file is
    /// intact, the half-done write is ignored, and the next save replaces the stale temp.
    #[test]
    fn crash_between_temp_write_and_rename_keeps_old_file() {
        let dir = scratch("crash");
        let store = Store::open(&dir).unwrap();
        store.save(&sample("a", "old")).unwrap();
        crash("store::tests::crash_child", &dir);
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
        let Some(dir) = crash_dir() else { return };
        let store = Store::open(&dir).unwrap();
        store.stage(&sample("a", "new")).unwrap();
        std::process::abort();
    }
}
