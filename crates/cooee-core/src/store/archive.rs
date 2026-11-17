use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
};

use rusqlite::{TransactionBehavior, params};
use serde::{Deserialize, Serialize};

use super::{Store, StoreError, db_err, int, io_err};
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
