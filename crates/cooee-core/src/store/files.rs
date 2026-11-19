use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::Duration,
};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::{
    StoreError, Thread,
    error::{db_err, io_err},
    items::put,
    writer::Writer,
};

/// The on-disk format version this build writes. A file with a newer one is refused, not guessed at.
pub const FORMAT: u32 = 2;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored<T> {
    format: u32,
    thread: T,
}

/// The thread store in one folder.
#[derive(Debug)]
pub struct Store {
    threads: PathBuf,
    pub(super) archive: PathBuf,
    db: Mutex<Connection>,
    pub(super) path: PathBuf,
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    pub(super) writer: Writer,
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
        let writer = Writer::open(&path)?;
        let store = Self {
            threads,
            archive: dir.join("archive"),
            db: Mutex::new(db),
            path,
            locks: Mutex::default(),
            writer,
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
            put(&tx, &self.path, &id, &item)?;
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
        self.writer.flush();
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

    pub(super) fn db(&self) -> MutexGuard<'_, Connection> {
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

    pub(super) fn path(&self, id: &str) -> Result<PathBuf, StoreError> {
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

#[cfg(test)]
pub(super) mod tests {
    use std::{env, process::Stdio, thread};

    use super::*;
    use crate::{Item, Message, Role};

    pub(in crate::store) fn scratch(name: &str) -> PathBuf {
        let dir = crate::scratch::owned(
            env::temp_dir().join(format!("cooee-store-{name}-{}", std::process::id())),
        );
        fs::remove_dir_all(&dir).ok();
        dir
    }

    pub(in crate::store) fn sample(id: &str, title: &str) -> Thread {
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

    pub(in crate::store) fn item(id: u64, kind: &str, text: &str) -> Item {
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

    pub(in crate::store) fn crash(child: &str, dir: &Path) {
        let status = crate::runner::command(env::current_exe().unwrap())
            .args(["--exact", child, "--ignored", "--test-threads=1"])
            .env("COOEE_CRASH_DIR", dir)
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(!status.success(), "{child} must die mid-write");
    }

    pub(in crate::store) fn crash_dir() -> Option<PathBuf> {
        crate::env_var("COOEE_CRASH_DIR").map(PathBuf::from)
    }

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
        crash("store::files::tests::crash_import_child", &dir);
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
            put(&tx, &store.path, "a", &item(n, "you", "a")).unwrap();
        }
        tx.commit().unwrap();
        let tx = db.transaction().unwrap();
        for n in 1..=25 {
            put(&tx, &store.path, "b", &item(n, "you", "b")).unwrap();
        }
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
        crash("store::files::tests::crash_child", &dir);
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
