use std::path::Path;

use rusqlite::{Connection, Transaction, params};

use super::{Store, StoreError, error::db_err};
use crate::{Body, Item, history};

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

impl Store {
    /// Stores `item` in thread `id`, replacing an earlier version of it, and waits until its batch is on disk.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the id is invalid or the item can't be written.
    pub fn append(&self, id: &str, item: &Item) -> Result<(), StoreError> {
        self.path(id)?;
        self.writer.send(Some((id.to_owned(), item.clone())), true)
    }

    /// Hands `item` to the writer, which stores it with the next batch; a crash before that batch syncs loses it.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the id is invalid or the writer has stopped.
    pub fn queue(&self, id: &str, item: &Item) -> Result<(), StoreError> {
        self.path(id)?;
        self.writer.send(Some((id.to_owned(), item.clone())), false)
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
        self.writer.flush();
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

    /// What thread `id`'s list row needs without reading it whole (#346): its approvals, the owner's newest message and
    /// its newest item, oldest first, with how many items after `read` are not the owner's.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the id is invalid or the items can't be read.
    pub fn glance(&self, id: &str, read: u64) -> Result<(Vec<Item>, usize), StoreError> {
        self.path(id)?;
        self.writer.flush();
        let db = self.db();
        if self.moved(&db, id)?.is_some() {
            drop(db);
            let items = self.items(id)?;
            let after = items.iter().filter(|i| i.id > read);
            let unseen = after
                .filter(|i| !matches!(i.body, Body::You { .. }))
                .count();
            return Ok((items, unseen));
        }
        let mut items = Vec::new();
        for filter in [
            "kind = 'approval'",
            "kind = 'you' ORDER BY id DESC LIMIT 1",
            "1 ORDER BY id DESC LIMIT 1",
        ] {
            items.extend(self.bodies(&db, &format!("thread = ?1 AND {filter}"), params![id])?);
        }
        items.sort_by_key(|i| i.id);
        items.dedup_by_key(|i| i.id);
        let unseen: i64 = db
            .prepare_cached(
                "SELECT (SELECT count(*) FROM items WHERE thread = ?1 AND id > ?2)
                 - (SELECT count(*) FROM items WHERE thread = ?1 AND kind = 'you' AND id > ?2)",
            )
            .and_then(|mut q| q.query_row(params![id, int(read)], |r| r.get(0)))
            .map_err(db_err(&self.path))?;
        Ok((items, usize::try_from(unseen).unwrap_or(0)))
    }

    pub(super) fn bodies<P: rusqlite::Params>(
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
    /// their thread, until `each` says stop. A word is looked up in the trigram index; `each` sees a superset (a `%` or `_` in a word
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
        self.writer.flush();
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
}

pub(super) fn put(tx: &Transaction, path: &Path, id: &str, item: &Item) -> Result<(), StoreError> {
    let body = serde_json::to_value(item).map_err(|source| StoreError::Parse {
        path: path.to_owned(),
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
        .map_err(db_err(path))?;
    if let Some((_, text)) = history::said(item) {
        tx.execute(
            "INSERT INTO said (rowid, text) VALUES (?1, ?2)",
            params![row, text.to_lowercase()],
        )
        .map_err(db_err(path))?;
    }
    Ok(())
}

pub(super) fn int(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::store::files::tests::{crash, crash_dir, item, scratch};

    pub(in crate::store) fn ids(items: &[Item]) -> Vec<u64> {
        items.iter().map(|i| i.id).collect()
    }

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

    #[test]
    fn a_kill_mid_append_keeps_every_acknowledged_item() {
        let dir = scratch("crash-append");
        crash("store::items::tests::crash_append_child", &dir);
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
        put(&tx, &store.path, "a", &item(101, "agent", "torn")).unwrap();
        std::process::abort();
    }
}
