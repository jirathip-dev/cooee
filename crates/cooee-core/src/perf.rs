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

/// The log's name in the data folder.
pub const FILE: &str = "perf.log";

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
