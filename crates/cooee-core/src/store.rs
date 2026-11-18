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
    /// The agent's open handoff (#164): its card's item id, 0 for none; until the owner hands back, nothing they
    /// send reaches the agent.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub handoff: u64,
    /// A setup chat (#270): its persona's brief asks it to interview the owner and write its Character and
    /// Instructions, and its `update_persona` saves at once.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub setup: bool,
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
    archive: PathBuf,
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
                     CREATE VIRTUAL TABLE IF NOT EXISTS said USING fts5(text, tokenize = 'trigram');
                     CREATE TABLE IF NOT EXISTS archived (thread TEXT NOT NULL, month TEXT NOT NULL,
                         PRIMARY KEY (thread, month));",
                )
            })
            .map_err(db_err(&path))?;
        let store = Self {
            threads,
            archive: dir.join("archive"),
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

    fn put(&self, tx: &Transaction, id: &str, item: &Item) -> Result<(), StoreError> {
        let body = serde_json::to_value(item).map_err(|source| StoreError::Parse {
            path: self.path.clone(),
            source,
        })?;
        let kind = body
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
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
        let db = self.db();
        if let Some(mut all) = self.moved(&db, id)? {
            for item in self.bodies(&db, "thread = ?1", params![id])? {
                all.insert(item.id, item);
            }
            let mut items: Vec<Item> = match from {
                Page::Last => all.into_values().rev().take(n).collect(),
                Page::Before(b) => all.range(..b).rev().take(n).map(|i| i.1.clone()).collect(),
                Page::After(a) => all.into_values().filter(|i| i.id > a).take(n).collect(),
            };
            if !matches!(from, Page::After(_)) {
                items.reverse();
            }
            return Ok(items);
        }
        let (sql, at) = match from {
            Page::Last => ("id < ?3 ORDER BY id DESC", i64::MAX),
            Page::Before(b) => ("id < ?3 ORDER BY id DESC", int(b)),
            Page::After(a) => ("id > ?3 ORDER BY id", int(a)),
        };
        let n = u64::try_from(n).map_or(i64::MAX, int);
        let mut items = self.bodies(
            &db,
            &format!("thread = ?1 AND {sql} LIMIT ?2"),
            params![id, n, at],
        )?;
        if !matches!(from, Page::After(_)) {
            items.reverse();
        }
        Ok(items)
    }

    fn bodies<P: rusqlite::Params>(
        &self,
        db: &Connection,
        filter: &str,
        params: P,
    ) -> Result<Vec<Item>, StoreError> {
        db.prepare_cached(&format!("SELECT body FROM items WHERE {filter}"))
            .and_then(|mut q| {
                q.query_map(params, |r| r.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(db_err(&self.path))?
            .iter()
            .map(|body| serde_json::from_str(body))
            .collect::<Result<Vec<Item>, _>>()
            .map_err(|source| StoreError::Parse {
                path: self.path.clone(),
                source,
            })
    }

    /// The items whose message text holds every one of `words` (lowercase), newest first and then the archive's, with
    /// their thread, until
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
                return Ok(());
            }
        }
        drop(rows);
        drop(q);
        drop(db);
        for (thread, item) in self.archived_said(words)? {
            if !each(&thread, &item) {
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

mod archive;

#[cfg(test)]
mod tests;
