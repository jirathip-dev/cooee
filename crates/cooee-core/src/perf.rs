//! Timing (#53): each HTTP request, shed call and first reply chunk is one JSON line in the data folder's
//! `perf.log` (`at`, `level`, `kind`, `name`, `ms`, and `status`, `via` or `thread`); `cooee perf` sums them up.
//! Only route patterns, methods, agent and thread ids and durations are written: never a prompt, a token or a URL.

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
    sync::{Mutex, OnceLock, PoisonError},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};

use crate::{Item, Page, Store, history};

/// The log's name in the data folder.
pub const FILE: &str = "perf.log";

/// A log bigger than this when the server starts is moved to `perf.log.old`.
const ROTATE: u64 = 16 << 20;

struct Log {
    file: Mutex<File>,
    slow_ms: u64,
}

static LOG: OnceLock<Log> = OnceLock::new();

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// Starts timing into `data/perf.log` for the rest of the process; a request taking `slow_ms` or more is a `warn`
/// line, and said on stderr too. Without it nothing is timed.
///
/// # Errors
/// When the log can't be opened.
pub fn init(data: &Path, slow_ms: u64) -> io::Result<()> {
    let path = data.join(FILE);
    if fs::metadata(&path).is_ok_and(|m| m.len() > ROTATE) {
        fs::rename(&path, data.join("perf.log.old"))?;
    }
    fs::create_dir_all(data)?;
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let _ = LOG.set(Log {
        file: Mutex::new(file),
        slow_ms,
    });
    Ok(())
}

fn write(log: &Log, mut line: Value, start: Instant) {
    line["at"] = now().into();
    line["ms"] = u64::try_from(start.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .into();
    let mut file = log.file.lock().unwrap_or_else(PoisonError::into_inner);
    let _ = writeln!(file, "{line}");
}

/// One HTTP request (or SSE stream opened) since `start`: its method, route pattern (`/threads/{id}`) and status.
pub fn request(method: &str, route: &str, status: u16, start: Instant) {
    let Some(log) = LOG.get() else { return };
    let ms = start.elapsed().as_millis();
    let name = format!("{method} {route}");
    let slow = ms >= u128::from(log.slow_ms);
    if slow {
        eprintln!("cooee: slow: {name} {status} {ms} ms");
    }
    let level = if slow { "warn" } else { "debug" };
    let line = json!({ "level": level, "kind": "http", "name": name, "status": status });
    write(log, line, start);
}

/// One shed call since `start`: its method, `via` the socket or the CLI.
pub fn shed(method: &str, via: &str, start: Instant) {
    let Some(log) = LOG.get() else { return };
    let line = json!({ "level": "debug", "kind": "shed", "name": method, "via": via });
    write(log, line, start);
}

/// The first reply chunk of thread `thread`'s prompt to `agent`, sent at `start`.
pub fn first_token(thread: &str, agent: &str, start: Instant) {
    let Some(log) = LOG.get() else { return };
    let line = json!({ "level": "debug", "kind": "first_token", "name": agent, "thread": thread });
    write(log, line, start);
}

/// The value at `p` percent of `sorted` (nearest rank).
fn rank(sorted: &[u64], p: usize) -> u64 {
    sorted[(sorted.len() * p).div_ceil(100).max(1) - 1]
}

/// `perf.log`'s lines `at` or after `since` as a Markdown table: count, p50, p95 and max in ms per route, shed
/// method (and transport) and agent's first token.
#[must_use]
pub fn table(log: &str, since: u64) -> String {
    let mut rows: BTreeMap<(String, String), Vec<u64>> = BTreeMap::new();
    for line in log.lines() {
        let Ok(l) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let (Some(at), Some(ms), Some(kind), Some(name)) = (
            l["at"].as_u64(),
            l["ms"].as_u64(),
            l["kind"].as_str(),
            l["name"].as_str(),
        ) else {
            continue;
        };
        let name = match l["via"].as_str() {
            Some(via) => format!("{name} ({via})"),
            None => name.to_owned(),
        };
        if at >= since {
            rows.entry((kind.to_owned(), name)).or_default().push(ms);
        }
    }
    let mut out =
        "| kind | name | n | p50 ms | p95 ms | max ms |\n|---|---|--:|--:|--:|--:|\n".to_owned();
    for ((kind, name), mut ms) in rows {
        ms.sort_unstable();
        let (n, p50, p95, max) = (ms.len(), rank(&ms, 50), rank(&ms, 95), ms[ms.len() - 1]);
        let _ = writeln!(out, "| {kind} | {name} | {n} | {p50} | {p95} | {max} |");
    }
    out
}

/// How often [`store`] times each row.
const RUNS: usize = 20;

/// Thread `id`'s items file in `dir` with `n` items: an owner's message, a tool call and a reply in turn.
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
        let _ = writeln!(lines, "{line}");
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
            let start = Instant::now();
            if run()? == 0 {
                return Err("read nothing".to_owned());
            }
            us.push(u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX));
        }
        us.sort_unstable();
        Ok(us)
    };
    let open = |id: &str| {
        store
            .items::<Item>(id)
            .map(|i| i.len())
            .map_err(|e| e.to_string())
    };
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
        let (n, p50, p95, max) = (us.len(), rank(us, 50), rank(us, 95), us[us.len() - 1]);
        let _ = writeln!(out, "| store | {name} | {n} | {p50} | {p95} | {max} |");
    }
    out
}
