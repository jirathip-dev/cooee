//! A persona's history (#217): a search over its own threads and a paged read of one, as plain text for the `history`
//! tool. The search looks words up in the store's full-text index (#221), the read pages back from the store: no
//! model call. Only the owner's messages and the agent's
//! final replies are read (never tool calls, their output or cards), redacted, each result under a hard cap.

use std::{
    collections::{BTreeMap, HashSet},
    fs, io,
    path::Path,
    sync::{Mutex, PoisonError},
};

use serde::{Deserialize, Serialize};

use crate::{Body, Item, Store, StoreError, redact, store::Page};

/// At most this many hits a search.
pub const HITS: usize = 5;
/// A hit's snippet is at most this many characters.
pub const SNIPPET: usize = 200;
/// A read is at most this many characters.
pub const PAGE: usize = 4000;
/// A consolidation run reads at most this many characters of new history (#219).
pub const NEW: usize = 20_000;

static MARKS: Mutex<()> = Mutex::new(());

/// Where a routine stands in a persona's history (#219, the heartbeat's too): the newest line read and the newest
/// due time run, in Unix ms.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Watermark {
    /// The newest transcript line read.
    pub line: u64,
    /// The newest due time run (0: none seen yet).
    pub due: u64,
}

/// Watermark `key` of `<data>/watermarks.json` changed by `f`, one change at a time, written when it changed: what
/// `f` returns.
///
/// # Errors
/// When the file can't be read or written.
pub fn watermark<T>(data: &Path, key: &str, f: impl FnOnce(&mut Watermark) -> T) -> io::Result<T> {
    let _held = MARKS.lock().unwrap_or_else(PoisonError::into_inner);
    let path = data.join("watermarks.json");
    let mut all: BTreeMap<String, Watermark> = match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes)?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => BTreeMap::new(),
        Err(e) => return Err(e),
    };
    let was = all.get(key).copied().unwrap_or_default();
    let mut w = was;
    let out = f(&mut w);
    if w != was {
        all.insert(key.to_owned(), w);
        crate::write_private(&path, &serde_json::to_vec_pretty(&all)?)?;
    }
    Ok(out)
}

/// What `threads` said after line `mark` (ms), a routine's prompts left out: none when the owner said nothing since;
/// else the newest lines that fit in [`NEW`] characters, oldest first, `<thread> <date> <who>: <text>` each, and the
/// newest line's time, the next mark.
///
/// # Errors
/// [`StoreError`] when a thread's items can't be read.
pub fn since(
    store: &Store,
    threads: &[String],
    mark: u64,
) -> Result<Option<(String, u64)>, StoreError> {
    let (mut lines, mut owner) = (Vec::new(), false);
    for id in threads {
        for i in store.items(id)?.iter().filter(|i| i.at > mark) {
            if matches!(
                &i.body,
                Body::You {
                    routine: Some(_),
                    ..
                }
            ) {
                continue;
            }
            let Some((who, text)) = said(i) else { continue };
            owner |= matches!(i.body, Body::You { .. });
            lines.push((i.at, format!("{id} {} {who}: {text}", date(i.at))));
        }
    }
    if !owner {
        return Ok(None);
    }
    lines.sort_by_key(|l| std::cmp::Reverse(l.0));
    let newest = lines.first().map_or(mark, |l| l.0);
    let (mut out, mut left) = (Vec::new(), NEW);
    for (_, line) in lines {
        let n = line.chars().count() + 1;
        if n > left {
            if out.is_empty() {
                out.push(line.chars().take(left - 1).collect());
            }
            break;
        }
        left -= n;
        out.push(line);
    }
    out.reverse();
    Ok(Some((out.join("\n"), newest)))
}

pub(crate) fn said(i: &Item) -> Option<(&str, String)> {
    let (who, text) = match &i.body {
        Body::You { text, .. } => ("owner", text),
        Body::Agent { text, .. } if i.speaker.is_empty() => ("reply", text),
        Body::Agent { text, .. } => (i.speaker.as_str(), text),
        _ => return None,
    };
    Some((
        who,
        redact(&text.split_whitespace().collect::<Vec<_>>().join(" ")),
    ))
}

pub(crate) fn date(ms: u64) -> String {
    let days = i64::try_from(ms / 86_400_000).unwrap_or(0) + 719_468;
    let (era, doe) = (days.div_euclid(146_097), days.rem_euclid(146_097));
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let (d, m) = (
        doy - (153 * mp + 2) / 5 + 1,
        if mp < 10 { mp + 3 } else { mp - 9 },
    );
    format!("{:04}-{m:02}-{d:02}", yoe + era * 400 + i64::from(m <= 2))
}

/// The newest message across `threads` holding every word of `query` (case-insensitive) per thread, newest first,
/// at most `limit` and [`HITS`]: one line each, `<thread> <date>: <snippet>`, the snippet at most [`SNIPPET`] characters.
///
/// # Errors
/// [`StoreError`] when a thread's items can't be read.
pub fn search(
    store: &Store,
    threads: &[String],
    query: &str,
    limit: usize,
) -> Result<String, StoreError> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut left: HashSet<&str> = threads.iter().map(String::as_str).collect();
    let want = limit.min(HITS);
    let mut hits = Vec::new();
    store.said(&words, |id, i| {
        if hits.len() >= want {
            return false;
        }
        let Some((_, text)) = said(i) else {
            return true;
        };
        let lower = text.to_lowercase();
        if !words.iter().all(|w| lower.contains(w.as_str())) || !left.remove(id) {
            return true;
        }
        let at = words.first().and_then(|w| lower.find(w.as_str()));
        let at = lower[..at.unwrap_or(0)].chars().count();
        let from = at.saturating_sub(SNIPPET / 4);
        let snippet: String = text.chars().skip(from).take(SNIPPET).collect();
        hits.push((i.at, format!("{id} {}: {snippet}", date(i.at))));
        true
    })?;
    hits.sort_by_key(|h| std::cmp::Reverse(h.0));
    let lines: Vec<String> = hits.into_iter().map(|h| h.1).collect();
    Ok(if lines.is_empty() {
        "no match".to_owned()
    } else {
        lines.join("\n")
    })
}

/// Thread `id`'s messages before item `before` (all without it), oldest first, as many of the newest as fit in
/// [`PAGE`] characters: `<date> <who>: <text>` a line, and `older: before=<item>` last when more are left.
///
/// # Errors
/// [`StoreError`] when its items can't be read.
pub fn read(store: &Store, id: &str, before: Option<u64>) -> Result<String, StoreError> {
    let (mut lines, mut left, mut more) = (Vec::new(), PAGE - 40, None);
    let mut from = before.map_or(Page::Last, Page::Before);
    'pages: loop {
        let page = store.page(id, from, 50)?;
        let Some(oldest) = page.first() else { break };
        from = Page::Before(oldest.id);
        for i in page.iter().rev() {
            let Some((who, text)) = said(i) else { continue };
            let line = format!("{} {who}: {text}", date(i.at));
            let n = line.chars().count();
            if n > left && !lines.is_empty() {
                more = lines.last().map(|l: &(u64, String)| l.0);
                break 'pages;
            }
            lines.push((i.id, line.chars().take(left).collect()));
            left = left.saturating_sub(n + 1);
            if left == 0 {
                more = Some(i.id);
                break 'pages;
            }
        }
    }
    let mut out: Vec<String> = lines.into_iter().rev().map(|l| l.1).collect();
    out.extend(more.map(|b| format!("older: before={b}")));
    Ok(out.join("\n"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn store(name: &str) -> Store {
        let dir = crate::scratch::owned(
            std::env::temp_dir().join(format!("cooee-history-{name}-{}", std::process::id())),
        );
        fs::remove_dir_all(&dir).ok();
        Store::open(&dir).unwrap()
    }

    fn item(id: u64, at: u64, body: Body) -> Item {
        Item {
            id,
            at,
            seq: 0,
            speaker: String::new(),
            body,
        }
    }

    fn you(text: &str) -> Body {
        Body::You {
            text: text.to_owned(),
            random_id: String::new(),
            files: Vec::new(),
            routine: None,
        }
    }

    fn agent(text: &str) -> Body {
        Body::Agent {
            text: text.to_owned(),
            stop_reason: "end_turn".to_owned(),
        }
    }

    fn tool(title: &str) -> Body {
        Body::Tool {
            call: "c".to_owned(),
            title: title.to_owned(),
            status: "completed".to_owned(),
            ended: None,
        }
    }

    #[test]
    fn a_search_finds_a_past_message_newest_first_within_the_caps() {
        let s = store("search");
        let day = 86_400_000;
        for n in 0..8u64 {
            let id = format!("t{n}");
            s.append(&id, &item(1, n * day, you("hello there")))
                .unwrap();
            let long = format!("{} the Kiwi recipe {}", "x ".repeat(300), "y ".repeat(300));
            s.append(&id, &item(2, n * day + 1, agent(&long))).unwrap();
            s.append(&id, &item(3, n * day + 2, tool("kiwi grep")))
                .unwrap();
        }
        let ids: Vec<String> = (0..8).map(|n| format!("t{n}")).collect();
        let got = search(&s, &ids, "KIWI recipe", 9).unwrap();
        let lines: Vec<&str> = got.lines().collect();
        assert_eq!(lines.len(), HITS, "{got}");
        assert!(lines[0].starts_with("t7 1970-01-08: "), "{got}");
        assert!(lines[4].starts_with("t3 1970-01-04: "), "{got}");
        for l in &lines {
            let snippet = l.split_once(": ").unwrap().1;
            assert!(snippet.chars().count() <= SNIPPET, "{l}");
            assert!(snippet.contains("Kiwi recipe"), "{l}");
        }
        assert_eq!(search(&s, &ids, "kiwi", 2).unwrap().lines().count(), 2);
        assert_eq!(search(&s, &ids, "grep", 5).unwrap(), "no match");
    }

    #[test]
    fn a_read_leaves_out_tool_rows_and_pages_with_before() {
        let s = store("read");
        let mut n = 0;
        for turn in 0..40 {
            for body in [
                you(&format!("ask {turn} {}", "a".repeat(100))),
                tool("cat secret.txt"),
                agent(&format!("answer {turn} {}", "b".repeat(100))),
            ] {
                n += 1;
                s.append("t1", &item(n, 1_762_819_200_000, body)).unwrap();
            }
        }
        let first = read(&s, "t1", None).unwrap();
        assert!(first.chars().count() <= PAGE);
        assert!(!first.contains("cat secret.txt"));
        assert!(first.contains("2025-11-11 reply: answer 39"), "{first}");
        let cursor = first.lines().last().unwrap();
        let before: u64 = cursor
            .strip_prefix("older: before=")
            .unwrap()
            .parse()
            .unwrap();
        let second = read(&s, "t1", Some(before)).unwrap();
        assert!(second.chars().count() <= PAGE);
        let oldest_first = first.lines().next().unwrap();
        assert!(!second.contains(oldest_first), "pages overlap");
        let mut all = String::new();
        let mut page = Some(None);
        while let Some(b) = page {
            let got = read(&s, "t1", b).unwrap();
            page = got
                .lines()
                .last()
                .and_then(|l| l.strip_prefix("older: before="))
                .map(|b| Some(b.parse().unwrap()));
            all = got + "\n" + &all;
        }
        assert!(all.contains("owner: ask 0 ") && all.contains("reply: answer 0 "));
    }

    #[test]
    fn new_history_is_the_newest_20000_characters_after_the_mark_and_none_without_an_owner_turn() {
        let s = store("since");
        let prompt = Body::You {
            text: "a routine's prompt".to_owned(),
            random_id: String::new(),
            files: Vec::new(),
            routine: Some(crate::routines::Mark {
                id: "consolidate".to_owned(),
                name: "Consolidation".to_owned(),
            }),
        };
        s.append("t1", &item(1, 10, you("old"))).unwrap();
        s.append("t1", &item(2, 20, prompt)).unwrap();
        s.append("t1", &item(3, 30, agent("a reply"))).unwrap();
        let ids = vec!["t1".to_owned(), "t2".to_owned()];
        assert_eq!(since(&s, &ids, 10).unwrap(), None, "no owner turn after 10");
        let (text, newest) = since(&s, &ids, 0).unwrap().unwrap();
        assert_eq!(newest, 30);
        assert_eq!(
            text,
            "t1 1970-01-01 owner: old\nt1 1970-01-01 reply: a reply"
        );
        for n in 0..300u64 {
            s.append(
                "t2",
                &item(
                    n + 1,
                    100 + n,
                    you(&format!("turn {n} {}", "w".repeat(100))),
                ),
            )
            .unwrap();
        }
        let (text, newest) = since(&s, &ids, 30).unwrap().unwrap();
        assert!(text.chars().count() <= NEW, "{}", text.chars().count());
        assert_eq!(newest, 399);
        assert!(
            text.ends_with(&format!("turn 299 {}", "w".repeat(100))),
            "newest kept"
        );
        assert!(
            !text.contains("turn 0 ") && !text.contains("a reply"),
            "oldest dropped"
        );
        assert_eq!(
            since(&s, &ids, newest).unwrap(),
            None,
            "nothing after the newest"
        );
    }

    #[test]
    fn a_watermark_changes_only_its_key_and_is_kept() {
        let dir = crate::scratch::owned(
            std::env::temp_dir().join(format!("cooee-marks-{}", std::process::id())),
        );
        fs::create_dir_all(&dir).unwrap();
        watermark(&dir, "a", |w| w.line = 5).unwrap();
        let got = watermark(&dir, "b", |w| *w).unwrap();
        assert_eq!(got, Watermark::default());
        assert_eq!(watermark(&dir, "a", |w| *w).unwrap().line, 5);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_search_over_1000_threads_stays_under_200_ms() {
        drop(store("perf"));
        let ids: Vec<String> = (0..1000).map(|n| format!("t{n:x}")).collect();
        let dir = crate::scratch::owned(
            std::env::temp_dir().join(format!("cooee-history-perf-{}", std::process::id())),
        );
        for (n, id) in (0u64..).zip(&ids) {
            let mut lines = String::new();
            for k in 0..10 {
                let asked = you(&format!("question {k} of thread {n}"));
                for i in [item(2 * k + 1, n, asked), item(2 * k + 2, n, tool("ls"))] {
                    lines += &(serde_json::to_string(&i).unwrap() + "\n");
                }
            }
            fs::write(dir.join(format!("threads/{id}.jsonl")), lines).unwrap();
        }
        let s = Store::open(&dir).unwrap();
        let start = crate::clock::instant();
        let got = search(&s, &ids, "question 9 thread 999", 5).unwrap();
        let ms = start.elapsed().as_millis();
        assert!(got.starts_with("t3e7 "), "{got}");
        assert!(ms < 200, "{ms} ms");
    }
}
