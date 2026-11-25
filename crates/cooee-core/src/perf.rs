//! Timing (#53): each HTTP request, shed call and first reply chunk is one JSON line in the data folder's
//! `perf.log` (`at`, `level`, `kind`, `name`, `ms`, and `status`, `via` or `thread`); `cooee perf` sums them up.
//! The first request after [`init`] adds a `start` line: the time from start to its answer (#346).
//! The app's own timings (#606) are `client` lines: `tap`, `event` and `echo`, checked against [`BUDGETS`].
//! Only route patterns, methods, agent and thread ids and durations are written: never a prompt, a token or a URL.

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
    sync::{Mutex, OnceLock, PoisonError},
    time::{Instant, UNIX_EPOCH},
};

use serde_json::{Value, json};

use crate::{Store, history, store::Page};

/// The log's name in the data folder.
pub const FILE: &str = "perf.log";

struct Log {
    file: Mutex<File>,
    slow_ms: u64,
    started: Mutex<Option<Instant>>,
}

static LOG: OnceLock<Log> = OnceLock::new();

fn now() -> u64 {
    crate::clock::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// Starts timing into `data/perf.log` for the rest of the process; a request taking `slow_ms` or more is a `warn`
/// line, and said on stderr too. Without it nothing is timed.
///
/// # Errors
/// When the log can't be opened.
pub fn init(data: &Path, slow_ms: u64) -> io::Result<()> {
    fs::create_dir_all(data)?;
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(data.join(FILE))?;
    LOG.set(Log {
        file: Mutex::new(file),
        slow_ms,
        started: Mutex::new(Some(crate::clock::instant())),
    })
    .ok();
    Ok(())
}

fn since(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn write(log: &Log, mut line: Value, ms: u64) {
    if let Some(o) = line.as_object_mut() {
        o.insert("at".to_owned(), now().into());
        o.insert("ms".to_owned(), ms.into());
    }
    let mut file = log.file.lock().unwrap_or_else(PoisonError::into_inner);
    file.write_all(format!("{line}\n").as_bytes()).ok();
}

/// One HTTP request (or SSE stream opened) since `start`: its method, route pattern (`/threads/{id}`) and status.
pub fn request(method: &str, route: &str, status: u16, start: Instant) {
    let Some(log) = LOG.get() else { return };
    let ms = start.elapsed().as_millis();
    let name = format!("{method} {route}");
    let slow = ms >= u128::from(log.slow_ms);
    if slow {
        crate::note!("cooee: slow: {name} {status} {ms} ms");
    }
    let level = if slow { "warn" } else { "debug" };
    let line = json!({ "level": level, "kind": "http", "name": name, "status": status });
    write(log, line, since(start));
    let started = log
        .started
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    if let Some(at) = started {
        let line = json!({ "level": "debug", "kind": "start", "name": "to first request" });
        write(log, line, since(at));
    }
}

/// One shed call since `start`: its method, `via` the socket or the CLI.
pub fn shed(method: &str, via: &str, start: Instant) {
    let Some(log) = LOG.get() else { return };
    let line = json!({ "level": "debug", "kind": "shed", "name": method, "via": via });
    write(log, line, since(start));
}

/// The first reply chunk of thread `thread`'s prompt to `agent`, sent at `start`.
pub fn first_token(thread: &str, agent: &str, start: Instant) {
    let Some(log) = LOG.get() else { return };
    let line = json!({ "level": "debug", "kind": "first_token", "name": agent, "thread": thread });
    write(log, line, since(start));
}

/// The app's p95 budgets in ms (#606): tap feedback, and fresh data (a stream event, a send's echo).
pub const BUDGETS: [(&str, u64); 3] = [("tap", 100), ("event", 150), ("echo", 150)];

/// One timing the app measured: `name` is one of [`BUDGETS`]' names.
pub fn client(name: &str, ms: u64) {
    let Some(log) = LOG.get() else { return };
    write(
        log,
        json!({ "level": "debug", "kind": "client", "name": name }),
        ms,
    );
}

fn rank(sorted: &[u64], p: usize) -> u64 {
    let at = (sorted.len() * p).div_ceil(100).max(1) - 1;
    sorted.get(at).copied().unwrap_or(0)
}

fn rows(log: &str, since: u64) -> BTreeMap<(String, String), Vec<u64>> {
    let mut rows: BTreeMap<(String, String), Vec<u64>> = BTreeMap::new();
    for line in log.lines() {
        let Ok(l) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let (Some(at), Some(ms), Some(kind), Some(name)) = (
            l.get("at").and_then(Value::as_u64),
            l.get("ms").and_then(Value::as_u64),
            l.get("kind").and_then(Value::as_str),
            l.get("name").and_then(Value::as_str),
        ) else {
            continue;
        };
        let name = match l.get("via").and_then(Value::as_str) {
            Some(via) => format!("{name} ({via})"),
            None => name.to_owned(),
        };
        if at >= since {
            rows.entry((kind.to_owned(), name)).or_default().push(ms);
        }
    }
    for ms in rows.values_mut() {
        ms.sort_unstable();
    }
    rows
}

/// Each of [`BUDGETS`] whose p95 in `perf.log`'s lines `at` or after `since` is over it, as `tap p95 140 ms > 100 ms`.
#[must_use]
pub fn over_budget(log: &str, since: u64) -> Vec<String> {
    let rows = rows(log, since);
    BUDGETS
        .iter()
        .filter_map(|(name, budget)| {
            let p95 = rank(rows.get(&("client".to_owned(), (*name).to_owned()))?, 95);
            (p95 > *budget).then(|| format!("{name} p95 {p95} ms > {budget} ms"))
        })
        .collect()
}

/// `perf.log`'s lines `at` or after `since` as a Markdown table: count, p50, p95 and max in ms per route, shed
/// method (and transport), agent's first token and the app's timings.
#[must_use]
pub fn table(log: &str, since: u64) -> String {
    let mut out =
        "| kind | name | n | p50 ms | p95 ms | max ms |\n|---|---|--:|--:|--:|--:|\n".to_owned();
    for ((kind, name), ms) in rows(log, since) {
        let (n, p50, p95, max) = (ms.len(), rank(&ms, 50), rank(&ms, 95), rank(&ms, 100));
        writeln!(out, "| {kind} | {name} | {n} | {p50} | {p95} | {max} |").ok();
    }
    out
}

const RUNS: usize = 20;

fn seed(dir: &Path, id: &str, n: u64) -> io::Result<()> {
    let mut lines = String::new();
    for k in 1..=n {
        let text = format!("question {k} of thread {id} {}", "ข้อความ words ".repeat(12));
        let line = match k % 3 {
            1 => json!({ "id": k, "at": k, "kind": "you", "text": text, "random_id": "r" }),
            2 => {
                json!({ "id": k, "at": k, "kind": "tool", "call": "c", "title": "ls", "status": "completed" })
            }
            _ => {
                json!({ "id": k, "at": k, "kind": "agent", "text": text, "stop_reason": "end_turn" })
            }
        };
        writeln!(lines, "{line}").ok();
    }
    fs::write(dir.join(format!("threads/{id}.jsonl")), lines)
}

/// The store's timings (#221) on a new store in `dir`: opening a 1,000- and a 10,000-item thread, reading the last
/// 50 items of each, and a search over 1,000 threads of 10 items. Each row's runs in microseconds, sorted.
///
/// # Errors
/// When the store can't be written or read.
pub fn store(dir: &Path) -> Result<Vec<(&'static str, Vec<u64>)>, String> {
    fs::create_dir_all(dir.join("threads")).map_err(|e| e.to_string())?;
    let ids: Vec<String> = (0..1000).map(|n| format!("s{n}")).collect();
    for (id, n) in ids
        .iter()
        .map(|i| (i.as_str(), 10))
        .chain([("t1k", 1000), ("t10k", 10_000)])
    {
        seed(dir, id, n).map_err(|e| e.to_string())?;
    }
    let store = Store::open(dir).map_err(|e| e.to_string())?;
    let time = |run: &dyn Fn() -> Result<usize, String>| -> Result<Vec<u64>, String> {
        let mut us = Vec::new();
        for _ in 0..RUNS {
            let start = crate::clock::instant();
            if run()? == 0 {
                return Err("read nothing".to_owned());
            }
            us.push(u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX));
        }
        us.sort_unstable();
        Ok(us)
    };
    let open = |id: &str| store.items(id).map(|i| i.len()).map_err(|e| e.to_string());
    let last = |id: &str| {
        store
            .page(id, Page::Last, 50)
            .map(|i| i.len())
            .map_err(|e| e.to_string())
    };
    let search = || {
        let got = history::search(&store, &ids, "question 7 thread s999", 5)
            .map_err(|e| e.to_string())?;
        Ok(usize::from(got.starts_with("s999 ")))
    };
    Ok(vec![
        ("open a 1,000-item thread", time(&|| open("t1k"))?),
        ("open a 10,000-item thread", time(&|| open("t10k"))?),
        ("last page of 1,000 items", time(&|| last("t1k"))?),
        ("last page of 10,000 items", time(&|| last("t10k"))?),
        ("search 1,000 threads", time(&search)?),
    ])
}

/// [`store`]'s rows as a Markdown table, as [`table`]'s but in microseconds.
#[must_use]
pub fn store_table(rows: &[(&str, Vec<u64>)]) -> String {
    let mut out =
        "| kind | name | n | p50 µs | p95 µs | max µs |\n|---|---|--:|--:|--:|--:|\n".to_owned();
    for (name, us) in rows {
        let (n, p50, p95, max) = (
            us.len(),
            rank(us, 50),
            rank(us, 95),
            us.last().copied().unwrap_or(0),
        );
        writeln!(out, "| store | {name} | {n} | {p50} | {p95} | {max} |").ok();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_page_does_not_grow_with_the_thread() {
        let dir = crate::scratch::owned(
            std::env::temp_dir().join(format!("cooee-perf-store-{}", std::process::id())),
        );
        fs::remove_dir_all(&dir).ok();
        let rows = store(&dir).unwrap();
        let fastest = |name: &str| rows.iter().find(|r| r.0 == name).unwrap().1[0];
        let (short, long) = (
            fastest("last page of 1,000 items"),
            fastest("last page of 10,000 items"),
        );
        assert!(long < 2 * short + 500, "{}", store_table(&rows));
    }
}
