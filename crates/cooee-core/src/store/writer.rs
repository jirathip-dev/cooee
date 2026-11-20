use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    slice,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use rusqlite::Connection;

use super::{StoreError, error::db_err, items::put};
use crate::Item;

type Done = mpsc::Sender<Result<(), StoreError>>;
type Pending = Arc<Mutex<HashMap<String, usize>>>;

#[derive(Debug)]
struct Job {
    write: Option<(String, Item)>,
    done: Option<Done>,
}

#[derive(Debug)]
pub(super) struct Writer {
    jobs: mpsc::Sender<Job>,
    path: PathBuf,
    pending: Pending,
    #[cfg(test)]
    commits: Arc<AtomicU64>,
}

impl Writer {
    pub(super) fn open(path: &Path) -> Result<Self, StoreError> {
        let db = Connection::open(path).map_err(db_err(path))?;
        db.busy_timeout(Duration::from_secs(10))
            .and_then(|()| db.execute_batch("PRAGMA synchronous = FULL;"))
            .map_err(db_err(path))?;
        let (jobs, next) = mpsc::channel();
        let commits = Arc::new(AtomicU64::new(0));
        let counted = Arc::clone(&commits);
        let at = path.to_owned();
        let pending = Pending::default();
        let queued = Arc::clone(&pending);
        thread::spawn(move || run(db, &at, &next, &counted, &queued));
        Ok(Self {
            jobs,
            path: path.to_owned(),
            pending,
            #[cfg(test)]
            commits,
        })
    }

    pub(super) fn send(&self, write: Option<(String, Item)>, wait: bool) -> Result<(), StoreError> {
        let (done, got) = mpsc::channel();
        let job = Job {
            write,
            done: wait.then_some(done),
        };
        let stopped = || StoreError::Io {
            path: self.path.clone(),
            source: io::Error::other("the store's writer stopped"),
        };
        if let Some((id, _)) = &job.write {
            *lock(&self.pending).entry(id.clone()).or_default() += 1;
        }
        self.jobs.send(job).map_err(|_| stopped())?;
        if wait {
            return got.recv().map_err(|_| stopped())?;
        }
        Ok(())
    }

    pub(super) fn flush(&self) {
        self.send(None, true).ok();
    }

    pub(super) fn flush_thread(&self, id: &str) {
        if lock(&self.pending).get(id).is_some_and(|n| *n > 0) {
            self.flush();
        }
    }
}

fn lock(pending: &Pending) -> std::sync::MutexGuard<'_, HashMap<String, usize>> {
    pending.lock().unwrap_or_else(PoisonError::into_inner)
}

fn run(
    mut db: Connection,
    path: &Path,
    next: &mpsc::Receiver<Job>,
    commits: &AtomicU64,
    pending: &Pending,
) {
    while let Ok(first) = next.recv() {
        let jobs: Vec<Job> = std::iter::once(first).chain(next.try_iter()).collect();
        let writes: Vec<&(String, Item)> = jobs.iter().filter_map(|j| j.write.as_ref()).collect();
        let done = |writes: &[&(String, Item)]| {
            let mut pending = lock(pending);
            for (id, _) in writes {
                if let Some(n) = pending.get_mut(id) {
                    *n = n.saturating_sub(1);
                }
            }
        };
        if writes.is_empty() || commit(&mut db, path, &writes, commits).is_ok() {
            done(&writes);
            for done in jobs.iter().filter_map(|j| j.done.as_ref()) {
                done.send(Ok(())).ok();
            }
            continue;
        }
        for job in &jobs {
            let r = job.write.as_ref().map_or(Ok(()), |w| {
                let r = commit(&mut db, path, slice::from_ref(&w), commits);
                done(slice::from_ref(&w));
                r
            });
            if let Some(done) = &job.done {
                done.send(r).ok();
            } else if let Err(e) = r {
                crate::note!("cooee: {e}");
            }
        }
    }
}

fn commit(
    db: &mut Connection,
    path: &Path,
    writes: &[&(String, Item)],
    commits: &AtomicU64,
) -> Result<(), StoreError> {
    let tx = db.transaction().map_err(db_err(path))?;
    for (id, item) in writes {
        put(&tx, path, id, item)?;
    }
    tx.commit().map_err(db_err(path))?;
    commits.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Store,
        store::files::tests::{crash, crash_dir, item, scratch},
    };

    #[test]
    fn three_hundred_queued_tool_versions_take_at_most_ten_syncs() {
        let dir = scratch("batch");
        let store = Store::open(&dir).unwrap();
        let before = store.writer.commits.load(Ordering::Relaxed);
        for n in 0..300 {
            store
                .queue("a", &item(n / 3 + 1, "tool", "pending"))
                .unwrap();
        }
        store.writer.flush();
        let syncs = store.writer.commits.load(Ordering::Relaxed) - before;
        assert!((1..=10).contains(&syncs), "{syncs} syncs");
        assert_eq!(store.items("a").unwrap().len(), 100);
    }

    #[test]
    fn a_kill_before_the_batch_syncs_keeps_every_acknowledged_message() {
        let dir = scratch("crash-batch");
        crash("store::writer::tests::crash_batch_child", &dir);
        let store = Store::open(&dir).unwrap();
        let items = store.items("a").unwrap();
        let kept: Vec<u64> = items.iter().map(|i| i.id).filter(|n| *n <= 100).collect();
        assert_eq!(kept, (1..=100).collect::<Vec<_>>());
        let mut said = items.iter().filter(|i| i.id % 2 == 0 && i.id <= 100);
        assert!(said.all(|i| *i == item(i.id, "you", "kept")));
    }

    #[test]
    #[ignore = "the child process of a_kill_before_the_batch_syncs_keeps_every_acknowledged_message"]
    fn crash_batch_child() {
        let Some(dir) = crash_dir() else { return };
        let store = Store::open(&dir).unwrap();
        for n in 1..=50 {
            for k in 0..20 {
                store
                    .queue("a", &item(n * 2 - 1, "tool", &format!("v{k}")))
                    .unwrap();
            }
            store.append("a", &item(n * 2, "you", "kept")).unwrap();
        }
        for n in 0..1000 {
            store
                .queue("a", &item(1000 + n, "tool", "unsynced"))
                .unwrap();
        }
        std::process::abort();
    }
}
