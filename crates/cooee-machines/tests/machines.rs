//! The machines plugin against a fake `shed` CLI (each machine's answer a file beside it) and a fake cooee that
//! records what the plugin posts: online and offline, the numbers on a card, the 24 h history, one alert per state,
//! and the backend's token gate.

use std::{
    env, fs,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU16, Ordering},
    },
    thread,
};

use cooee_machines::{Cooee, DAY, Machines, render, serve};
use serde_json::{Value, json};

/// The fake CLI: `machine ls` prints `ls.json`; `--on <m> … machine stats` prints `<m>.json` (exit 1 when it says
/// offline, as shed does); `machine stats` prints `local.json`, or fails like an unreachable server without one.
const SHED: &str = r#"#!/bin/sh
d=$(dirname "$0")
printf '%s\n' "$*" >>"$d/calls.log"
case "$*" in
*"machine ls") cat "$d/ls.json" ;;
*"--on "*)
  m=$(printf '%s' "$*" | sed 's/.*--on \([^ ]*\).*/\1/')
  cat "$d/$m.json"; grep -q '"online":false' "$d/$m.json" && exit 1; exit 0 ;;
*"machine stats"*) cat "$d/local.json" 2>/dev/null || { echo "shed: no server at shed.sock" >&2; exit 1; } ;;
esac
"#;

const NOW: u64 = 1_800_000_000;

fn scratch(name: &str) -> PathBuf {
    let dir = env::temp_dir().join(format!("cooee-machines-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let shed = dir.join("shed");
    fs::write(&shed, SHED).unwrap();
    fs::set_permissions(&shed, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        dir.join("ls.json"),
        r#"[{"name":"air","target":"air.local"}]"#,
    )
    .unwrap();
    dir
}

fn machines(dir: &Path) -> Machines {
    Machines::new(dir.join("shed"), None, "mini".to_owned())
}

fn sample(at: u64, load: f64, mem: u64, disk_mb: u64) -> Value {
    json!({
        "at": at, "load_per_cpu": load, "mem_pressure_pct": mem, "swap_used_mb": 1536, "disk_free_mb": disk_mb,
        "cpus": 10, "uptime_s": 3600, "terminals": { "working": 2, "blocked": 1, "exited": 3 }
    })
}

fn answer(dir: &Path, machine: &str, v: &Value) {
    fs::write(dir.join(format!("{machine}.json")), v.to_string()).unwrap();
}

fn offline(dir: &Path, machine: &str, last_seen: u64) {
    let v = json!({ "machine": machine, "online": false, "last_seen": last_seen });
    answer(dir, machine, &v);
}

#[test]
fn online_and_offline_with_last_seen() {
    let dir = scratch("online");
    answer(&dir, "local", &sample(NOW, 0.4, 50, 50_000));
    offline(&dir, "air", NOW - 12 * 60);
    let m = machines(&dir);
    let cards = m.poll(NOW);
    let names: Vec<_> = cards.iter().map(|c| (c.name.as_str(), c.online)).collect();
    assert_eq!(names, [("mini", true), ("air", false)]);
    assert_eq!(cards[1].last_seen, Some(NOW - 12 * 60));
    let html = render(&cards, NOW);
    assert!(
        html.contains(
            "<h2 id=\"m-mini\"><i aria-hidden=\"true\">●</i> mini</h2><p class=\"seen\">online</p>"
        ),
        "{html}"
    );
    assert!(
        html.contains("<article class=\"m off\" aria-labelledby=\"m-air\">"),
        "{html}"
    );
    assert!(html.contains("offline · last seen 12 min ago"), "{html}");

    // The Air comes back; the Mini's shed stops answering: offline, last seen when it last answered, and why.
    answer(&dir, "air", &sample(NOW + 60, 0.1, 20, 90_000));
    fs::remove_file(dir.join("local.json")).unwrap();
    let cards = m.poll(NOW + 3 * 3600);
    assert!(cards[1].online);
    assert!(!cards[0].online);
    assert_eq!(cards[0].last_seen, Some(NOW));
    let html = render(&cards, NOW + 3 * 3600);
    assert!(html.contains("offline · last seen 3 h ago"), "{html}");
    assert!(
        html.contains("<p class=\"er\">shed: no server at shed.sock</p>"),
        "{html}"
    );
    // Its last numbers stay on the card.
    assert!(
        html.contains("<span class=\"k\">load/cpu</span><b>0.40</b>"),
        "{html}"
    );
}

#[test]
fn a_card_shows_load_memory_swap_disk_and_terminals() {
    let dir = scratch("stats");
    answer(&dir, "local", &sample(NOW, 0.42, 62, 48 * 1024));
    answer(&dir, "air", &sample(NOW, 1.25, 93, 8 * 1024));
    let m = machines(&dir);
    let html = render(&m.poll(NOW), NOW);
    let (mini, air) = html.split_once("aria-labelledby=\"m-air\"").unwrap();
    for (card, want) in [
        (mini, "<span class=\"k\">load/cpu</span><b>0.42</b>"),
        (mini, "<span class=\"k\">memory</span><b>62%</b>"),
        (mini, "<span class=\"k\">swap</span><b>1.5 GB</b>"),
        (mini, "<span class=\"k\">disk free</span><b>48.0 GB</b>"),
        (
            mini,
            "<p class=\"tm\">terminals: 1 blocked · 3 exited · 2 working</p>",
        ),
        (
            air,
            "<span class=\"k\">load/cpu</span><b class=\"bad\">1.25</b>",
        ),
        (
            air,
            "<span class=\"k\">memory</span><b class=\"bad\">93% critical</b>",
        ),
        (
            air,
            "<span class=\"k\">disk free</span><b class=\"bad\">8.0 GB low</b>",
        ),
    ] {
        assert!(card.contains(want), "{want}\n{card}");
    }
    // Each number opens its chart, named for a screen reader.
    assert!(
        mini.contains("<details id=\"mini-load\"><summary>"),
        "{mini}"
    );
    assert!(
        mini.contains("aria-label=\"memory, last 24 h: low 62% · high 62%\""),
        "{mini}"
    );
}

#[test]
fn the_24_h_history_is_asked_for_once_then_only_what_is_new() {
    let dir = scratch("history");
    let day: Vec<Value> = (1..DAY / 60)
        .map(|i| sample(NOW - DAY + i * 60, 0.5, 40, 60_000))
        .collect();
    let mut first = sample(NOW, 0.5, 40, 60_000);
    first["history"] = json!(day);
    answer(&dir, "local", &first);
    offline(&dir, "air", NOW - 60);
    let m = machines(&dir);
    let card = m.poll(NOW).remove(0);
    assert_eq!(card.history.len(), 1439);
    let log = fs::read_to_string(dir.join("calls.log")).unwrap();
    assert!(
        log.contains(&format!("--json machine stats --since {}", NOW - DAY)),
        "{log}"
    );
    assert!(
        log.contains(&format!(
            "--json --on air machine stats --since {}",
            NOW - DAY
        )),
        "{log}"
    );
    // Every sample and the one taken now is a point of the sparkline, oldest at the left, now at the right.
    let html = render(&[card], NOW);
    let spark = html.split("<svg class=\"sp\"").nth(1).unwrap();
    let points = spark
        .split("points=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    assert_eq!(points.split(' ').count(), 1440);
    assert!(
        points.starts_with("0.1,") && points.ends_with(" 100.0,10.0"),
        "{points}"
    );

    // A minute later: only the samples after the newest kept one are asked for, and the oldest falls out of 24 h.
    let mut next = sample(NOW + 60, 0.5, 40, 60_000);
    next["history"] = json!([sample(NOW, 0.5, 40, 60_000)]);
    answer(&dir, "local", &next);
    let card = m.poll(NOW + 60).remove(0);
    let log = fs::read_to_string(dir.join("calls.log")).unwrap();
    assert!(
        log.contains(&format!("--json machine stats --since {}", NOW - 60)),
        "{log}"
    );
    assert_eq!(card.history.len(), 1439);
    assert_eq!(card.history[0].at, NOW - DAY + 120);
    assert_eq!(card.history.last().unwrap().at, NOW);
}

/// A fake cooee: every POST's path and body; it answers `status`.
struct FakeCooee {
    cooee: Cooee,
    posts: Arc<Mutex<Vec<(String, Value)>>>,
    status: Arc<AtomicU16>,
}

impl FakeCooee {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let posts: Arc<Mutex<Vec<(String, Value)>>> = Arc::default();
        let status = Arc::new(AtomicU16::new(200));
        let (log, code) = (Arc::clone(&posts), Arc::clone(&status));
        thread::spawn(move || {
            for s in listener.incoming().flatten() {
                let mut r = BufReader::new(s);
                let (mut head, mut line) = (String::new(), String::new());
                while r.read_line(&mut line).unwrap() > 2 {
                    head.push_str(&line);
                    line.clear();
                }
                assert!(
                    head.contains("Authorization: Bearer machines-token-0123"),
                    "{head}"
                );
                let len: usize = head
                    .lines()
                    .find_map(|l| l.strip_prefix("Content-Length: "))
                    .unwrap()
                    .parse()
                    .unwrap();
                let mut body = vec![0; len];
                r.read_exact(&mut body).unwrap();
                let path = head.split(' ').nth(1).unwrap().to_owned();
                log.lock()
                    .unwrap()
                    .push((path, serde_json::from_slice(&body).unwrap()));
                let code = code.load(Ordering::SeqCst);
                let _ = write!(
                    r.into_inner(),
                    "HTTP/1.0 {code} X\r\nContent-Length: 0\r\n\r\n"
                );
            }
        });
        let cooee = Cooee {
            addr,
            token: "machines-token-0123".to_owned(),
        };
        Self {
            cooee,
            posts,
            status,
        }
    }

    fn take(&self) -> Vec<(String, Value)> {
        std::mem::take(&mut *self.posts.lock().unwrap())
    }
}

#[test]
fn an_alert_is_pushed_once_per_state_and_the_tab_dot_follows() {
    let dir = scratch("alerts");
    answer(&dir, "local", &sample(NOW, 0.4, 50, 50_000));
    offline(&dir, "air", NOW - 12 * 60);
    let (m, cooee) = (machines(&dir), FakeCooee::start());
    m.tick(NOW, &cooee.cooee);
    let push =
        json!({ "text": "air: offline, last seen 12 min ago", "path": "air", "tag": "machine" });
    let dot = |on| ("/plugin/dot".to_owned(), json!({ "on": on }));
    assert_eq!(cooee.take(), [("/plugin/push".to_owned(), push), dot(true)]);
    // Still offline: nothing more.
    m.tick(NOW + 60, &cooee.cooee);
    m.tick(NOW + 120, &cooee.cooee);
    assert_eq!(cooee.take(), []);

    // Back, but its disk is low and its memory critical: a new state, one push.
    answer(&dir, "air", &sample(NOW + 180, 0.1, 95, 8 * 1024));
    m.tick(NOW + 180, &cooee.cooee);
    m.tick(NOW + 240, &cooee.cooee);
    let push = json!({ "text": "air: disk 8.0 GB free; memory pressure critical (95%)", "path": "air", "tag": "machine" });
    assert_eq!(cooee.take(), [("/plugin/push".to_owned(), push)]);

    // All well: no push, the dot goes.
    answer(&dir, "air", &sample(NOW + 300, 0.1, 20, 90_000));
    m.tick(NOW + 300, &cooee.cooee);
    assert_eq!(cooee.take(), [dot(false)]);

    // A push cooee refuses is tried again on the next poll, then not again.
    fs::remove_file(dir.join("local.json")).unwrap();
    cooee.status.store(500, Ordering::SeqCst);
    m.tick(NOW + 360, &cooee.cooee);
    cooee.status.store(200, Ordering::SeqCst);
    m.tick(NOW + 420, &cooee.cooee);
    m.tick(NOW + 480, &cooee.cooee);
    let push =
        json!({ "text": "mini: offline, last seen 7 min ago", "path": "mini", "tag": "machine" });
    let pushes: Vec<_> = cooee
        .take()
        .into_iter()
        .filter(|p| p.0 == "/plugin/push")
        .collect();
    assert_eq!(pushes.len(), 2);
    assert_eq!(pushes[1], ("/plugin/push".to_owned(), push));
}

fn get(port: u16, path: &str, auth: &str) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(s, "GET {path} HTTP/1.0\r\nHost: x\r\n{auth}\r\n").unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    out
}

#[test]
fn the_backend_answers_only_with_the_plugin_token() {
    let dir = scratch("serve");
    answer(&dir, "local", &sample(NOW, 0.4, 50, 50_000));
    offline(&dir, "air", NOW - 60);
    let m = Arc::new(machines(&dir));
    m.poll(NOW);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let served = Arc::clone(&m);
    thread::spawn(move || serve(&listener, "machines-token-0123", &served));
    let ok = "Authorization: Bearer machines-token-0123\r\n";
    for auth in ["", "Authorization: Bearer machines-token-0124\r\n"] {
        assert!(get(port, "/cards", auth).starts_with("HTTP/1.0 401"));
    }
    let cards = get(port, "/cards", ok);
    assert!(
        cards.starts_with("HTTP/1.0 200 OK\r\nContent-Type: text/html"),
        "{cards}"
    );
    assert!(
        cards.contains("aria-labelledby=\"m-mini\"") && cards.contains("aria-labelledby=\"m-air\""),
        "{cards}"
    );
    let tab = get(port, "/tab.js", ok);
    assert!(
        tab.contains("text/javascript") && tab.contains("cooee.fetch(\"/cards\")"),
        "{tab}"
    );
    assert!(get(port, "/nope", ok).starts_with("HTTP/1.0 404"));
}
