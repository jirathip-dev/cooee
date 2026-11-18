use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
};

use rusqlite::{TransactionBehavior, params};
use serde::{Deserialize, Serialize};

use super::{
    Store, StoreError,
    error::{db_err, io_err},
    items::int,
};
use crate::{Body, Item, config::History, history};

const DAY: u64 = 86_400_000;
const MARK: &str = "\n… trimmed: the whole text is in the archive …\n";

#[derive(Serialize, Deserialize)]
struct Line {
    thread: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    whole: bool,
    item: Item,
}

impl Store {
    /// Applies `history` at `now` (Unix ms): each ended thread (closed, or its session over) quiet for `hot_days`
    /// moves to the archive of its last month, and each tool item older than `trim_after_days` over `trim_over_kb`
    /// keeps its head and tail, archived whole first. Returns how many items it moved or trimmed; run again, none.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the database or an archive file can't be read or written.
    pub fn retain(&self, now: u64, history: &History) -> Result<usize, StoreError> {
        let quiet = now.saturating_sub(history.hot_days.saturating_mul(DAY));
        let old = now.saturating_sub(history.trim_after_days.saturating_mul(DAY));
        let over = history.trim_over_kb.saturating_mul(1024);
        let mut db = self.db();
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_err(&self.path))?;
        let mut months: BTreeMap<String, Vec<Line>> = BTreeMap::new();
        let (mut moved, mut trims, mut n) = (Vec::new(), Vec::new(), 0);
        let stale = strings(
            &tx,
            "SELECT thread FROM items GROUP BY thread HAVING MAX(at) < ?1",
            params![int(quiet)],
        )
        .map_err(db_err(&self.path))?;
        for id in stale {
            let items = self.bodies(&tx, "thread = ?1 ORDER BY id", params![id])?;
            let ended = items
                .last()
                .is_some_and(|i| matches!(i.body, Body::Ended { .. }));
            let Some(last) = items.iter().map(|i| i.at).max() else {
                continue;
            };
            if ended || self.load(&id).is_ok_and(|t| t.closed) {
                n += items.len();
                let lines = months.entry(month(last)).or_default();
                moved.push((id.clone(), month(last)));
                lines.extend(items.into_iter().map(|item| Line {
                    thread: id.clone(),
                    whole: false,
                    item,
                }));
            }
        }
        let long = self.lines(
            &tx,
            "SELECT thread, body FROM items WHERE kind = 'tool' AND at < ?1 AND length(body) > ?2",
            params![int(old), int(over)],
        )?;
        for line in long {
            if let Some(short) = trim(&line.item, usize::try_from(over).unwrap_or(usize::MAX))
                && !moved.iter().any(|m| m.0 == line.thread)
            {
                trims.push((line.thread.clone(), short));
                let at = month(line.item.at);
                months.entry(at).or_default().push(Line {
                    whole: true,
                    ..line
                });
            }
        }
        for (month, lines) in &months {
            self.write_month(month, lines)?;
        }
        for (id, month) in &moved {
            tx.execute(
                "DELETE FROM said WHERE rowid IN (SELECT rowid FROM items WHERE thread = ?1)",
                [id],
            )
            .and_then(|_| tx.execute("DELETE FROM items WHERE thread = ?1", [id]))
            .and_then(|_| {
                tx.execute(
                    "INSERT OR IGNORE INTO archived (thread, month) VALUES (?1, ?2)",
                    [id, month],
                )
            })
            .map_err(db_err(&self.path))?;
        }
        for (id, short) in &trims {
            self.put(&tx, id, short)?;
        }
        tx.commit().map_err(db_err(&self.path))?;
        n += trims.len();
        if n > 0 {
            db.execute_batch("VACUUM").map_err(db_err(&self.path))?;
        }
        Ok(n)
    }

    /// Thread `id`'s items with their whole text: the hot store's, a trimmed tool item's from the archive.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the id is invalid, or the items or the archive can't be read.
    pub fn whole(&self, id: &str) -> Result<Vec<Item>, StoreError> {
        let mut items: BTreeMap<u64, Item> =
            self.items(id)?.into_iter().map(|i| (i.id, i)).collect();
        for line in self.archived(true)? {
            if line.thread == id {
                items.insert(line.item.id, line.item);
            }
        }
        Ok(items.into_values().collect())
    }

    pub(super) fn moved(
        &self,
        db: &rusqlite::Connection,
        id: &str,
    ) -> Result<Option<BTreeMap<u64, Item>>, StoreError> {
        let months = strings(
            db,
            "SELECT month FROM archived WHERE thread = ?1 ORDER BY month",
            params![id],
        )
        .map_err(db_err(&self.path))?;
        if months.is_empty() {
            return Ok(None);
        }
        let mut items = BTreeMap::new();
        for month in &months {
            for line in read_month(&self.month_path(month))? {
                if !line.whole && line.thread == id {
                    items.insert(line.item.id, line.item);
                }
            }
        }
        Ok(Some(items))
    }

    pub(super) fn archived_said(
        &self,
        words: &[String],
    ) -> Result<Vec<(String, Item)>, StoreError> {
        let mut found: Vec<(String, Item)> = self
            .archived(false)?
            .into_iter()
            .filter(|l| {
                history::said(&l.item).is_some_and(|(_, text)| {
                    let text = text.to_lowercase();
                    words.iter().all(|w| text.contains(w.as_str()))
                })
            })
            .map(|l| (l.thread, l.item))
            .collect();
        found.sort_by_key(|(_, i)| std::cmp::Reverse((i.at, i.id)));
        Ok(found)
    }

    fn archived(&self, whole: bool) -> Result<Vec<Line>, StoreError> {
        let mut lines = Vec::new();
        let entries = match fs::read_dir(&self.archive) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(lines),
            entries => entries.map_err(io_err(&self.archive))?,
        };
        for entry in entries {
            let path = entry.map_err(io_err(&self.archive))?.path();
            if path.to_string_lossy().ends_with(".jsonl.zst") {
                lines.extend(read_month(&path)?.into_iter().filter(|l| l.whole == whole));
            }
        }
        Ok(lines)
    }

    fn lines<P: rusqlite::Params>(
        &self,
        db: &rusqlite::Connection,
        sql: &str,
        params: P,
    ) -> Result<Vec<Line>, StoreError> {
        let rows = db
            .prepare(sql)
            .and_then(|mut q| {
                q.query_map(params, |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()
            })
            .map_err(db_err(&self.path))?;
        rows.into_iter()
            .map(|(thread, body)| {
                let item = serde_json::from_str(&body).map_err(|source| StoreError::Parse {
                    path: self.path.clone(),
                    source,
                })?;
                Ok(Line {
                    thread,
                    whole: false,
                    item,
                })
            })
            .collect()
    }

    fn month_path(&self, month: &str) -> std::path::PathBuf {
        self.archive.join(format!("threads-{month}.jsonl.zst"))
    }

    fn write_month(&self, month: &str, lines: &[Line]) -> Result<(), StoreError> {
        fs::create_dir_all(&self.archive).map_err(io_err(&self.archive))?;
        let path = self.month_path(month);
        let mut text = unpack_file(&path)?;
        for line in lines {
            serde_json::to_writer(&mut text, line).map_err(|source| StoreError::Parse {
                path: path.clone(),
                source,
            })?;
            text.push(b'\n');
        }
        let tmp = path.with_extension("zst.tmp");
        let mut file = File::create(&tmp).map_err(io_err(&tmp))?;
        file.write_all(&pack(&text))
            .and_then(|()| file.sync_all())
            .map_err(io_err(&tmp))?;
        fs::rename(&tmp, &path).map_err(io_err(&path))?;
        File::open(&self.archive)
            .and_then(|dir| dir.sync_all())
            .map_err(io_err(&self.archive))
    }
}

fn strings<P: rusqlite::Params>(
    db: &rusqlite::Connection,
    sql: &str,
    params: P,
) -> rusqlite::Result<Vec<String>> {
    db.prepare(sql)?.query_map(params, |r| r.get(0))?.collect()
}

fn month(ms: u64) -> String {
    history::date(ms).chars().take(7).collect()
}

fn trim(item: &Item, over: usize) -> Option<Item> {
    let Body::Tool { title, .. } = &item.body else {
        return None;
    };
    let keep = over / 4;
    let head = title.get(..title.floor_char_boundary(keep))?;
    let tail = title.get(title.ceil_char_boundary(title.len().saturating_sub(keep))..)?;
    let short = format!("{head}{MARK}{tail}");
    if title.len() <= over || short.len() >= title.len() {
        return None;
    }
    let mut item = item.clone();
    if let Body::Tool { title, .. } = &mut item.body {
        *title = short;
    }
    Some(item)
}

fn read_month(path: &Path) -> Result<Vec<Line>, StoreError> {
    let text = unpack_file(path)?;
    Ok(text
        .split(|b| *b == b'\n')
        .filter_map(|l| serde_json::from_slice(l).ok())
        .collect())
}

fn unpack_file(path: &Path) -> Result<Vec<u8>, StoreError> {
    match fs::read(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        bytes => bytes.and_then(|b| unpack(&b)).map_err(io_err(path)),
    }
}

fn pack(text: &[u8]) -> Vec<u8> {
    ruzstd::encoding::compress_to_vec(text, ruzstd::encoding::CompressionLevel::Fastest)
}

fn unpack(mut bytes: &[u8]) -> io::Result<Vec<u8>> {
    let mut text = Vec::new();
    while !bytes.is_empty() {
        ruzstd::decoding::StreamingDecoder::new(&mut bytes)
            .map_err(io::Error::other)?
            .read_to_end(&mut text)?;
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::super::{
        Page, Thread,
        files::tests::{item, sample, scratch},
        items::tests::ids,
    };
    use super::*;

    fn at(mut item: Item, at: u64) -> Item {
        item.at = at;
        item
    }

    fn keys(toml: &str) -> crate::config::History {
        toml::from_str::<crate::Config>(toml).unwrap().history
    }

    fn hot_rows(store: &Store, id: &str) -> i64 {
        store
            .db()
            .query_row("SELECT COUNT(*) FROM items WHERE thread = ?1", [id], |r| {
                r.get(0)
            })
            .unwrap()
    }

    fn seed_ended(store: &Store, id: &str, closed: bool, last: u64) -> Vec<Item> {
        store
            .save(&Thread {
                closed,
                ..sample(id, id)
            })
            .unwrap();
        let items = vec![
            at(item(1, "you", &format!("archived question {id}")), last - 2),
            at(item(2, "tool", "ls"), last - 1),
            at(item(3, "agent", "the archived answer"), last),
        ];
        for i in &items {
            store.append(id, i).unwrap();
        }
        items
    }

    #[test]
    fn a_thread_past_hot_days_is_archived_and_reopens_with_every_item() {
        let dir = scratch("retain-move");
        let store = Store::open(&dir).unwrap();
        let old = seed_ended(&store, "old", true, 40 * DAY);
        let open = seed_ended(&store, "open", false, 40 * DAY);
        let recent = seed_ended(&store, "recent", true, 60 * DAY);
        let now = 75 * DAY;
        assert_eq!(store.retain(now, &keys("")).unwrap(), 3);
        assert_eq!(store.retain(now, &keys("")).unwrap(), 0);
        let zst = fs::read(dir.join("archive/threads-1970-02.jsonl.zst")).unwrap();
        assert_eq!(zst.get(..4), Some(&[0x28, 0xb5, 0x2f, 0xfd][..]));
        assert_eq!(
            (
                hot_rows(&store, "old"),
                hot_rows(&store, "open"),
                hot_rows(&store, "recent")
            ),
            (0, 3, 3)
        );
        drop(store);
        let store = Store::open(&dir).unwrap();
        assert_eq!(store.items("old").unwrap(), old);
        assert_eq!(store.items("open").unwrap(), open);
        assert_eq!(store.items("recent").unwrap(), recent);
        assert_eq!(ids(&store.page("old", Page::Last, 2).unwrap()), [2, 3]);
        assert_eq!(ids(&store.page("old", Page::Before(3), 1).unwrap()), [2]);
        assert_eq!(ids(&store.page("old", Page::After(1), 5).unwrap()), [2, 3]);
        let read = history::read(&store, "old", None).unwrap();
        assert!(read.contains("owner: archived question old"), "{read}");
        let found = history::search(&store, &["old".to_owned()], "archived question", 5).unwrap();
        assert!(found.starts_with("old 1970-02-"), "{found}");
        let more = at(item(4, "you", "back again"), now);
        store.append("old", &more).unwrap();
        let mut all = old;
        all.push(more);
        assert_eq!(store.items("old").unwrap(), all);
    }

    #[test]
    fn a_trimmed_tool_item_shows_its_head_and_tail_and_its_whole_text_comes_back() {
        let dir = scratch("retain-trim");
        let store = Store::open(&dir).unwrap();
        let long = format!("HEAD{}TAIL", "ก".repeat(5000));
        let tool = at(item(1, "tool", &long), DAY);
        let short = at(item(2, "tool", "ls"), DAY);
        let said = at(item(3, "agent", &long), DAY);
        for i in [&tool, &short, &said] {
            store.append("t", i).unwrap();
        }
        assert_eq!(store.retain(9 * DAY, &keys("")).unwrap(), 1);
        assert_eq!(store.retain(9 * DAY, &keys("")).unwrap(), 0);
        let items = store.items("t").unwrap();
        let Some(Item {
            body: crate::Body::Tool { title, .. },
            ..
        }) = items.first()
        else {
            panic!("{items:?}");
        };
        assert!(
            title.starts_with("HEADก") && title.ends_with("กTAIL"),
            "{title}"
        );
        assert!(
            title.contains("trimmed") && title.len() < 4096,
            "{}",
            title.len()
        );
        assert_eq!(items.get(1..), Some(&[short.clone(), said.clone()][..]));
        assert_eq!(store.whole("t").unwrap(), [tool, short, said]);
        assert!(dir.join("archive/threads-1970-01.jsonl.zst").is_file());
    }

    #[test]
    fn changing_a_key_changes_the_behaviour() {
        let dir = scratch("retain-keys");
        let store = Store::open(&dir).unwrap();
        seed_ended(&store, "a", true, 40 * DAY);
        let long = at(item(1, "tool", &"x".repeat(3000)), 40 * DAY);
        store.append("b", &long).unwrap();
        let now = 75 * DAY;
        let keep = "[history]\nhot_days = 40\ntrim_after_days = 40\ntrim_over_kb = 3\n";
        assert_eq!(store.retain(now, &keys(keep)).unwrap(), 0);
        assert_eq!(
            store
                .retain(now, &keys("[history]\nhot_days = 30"))
                .unwrap(),
            3
        );
        assert_eq!(
            store
                .retain(now, &keys("[history]\ntrim_over_kb = 2"))
                .unwrap(),
            1
        );
        assert_eq!(hot_rows(&store, "a"), 0);
        assert_eq!(store.whole("b").unwrap(), [long]);
        assert!(toml::from_str::<crate::Config>("[history]\nhot_dayz = 1").is_err());
    }
}
