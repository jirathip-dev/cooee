//! Timing (#53): `cooee serve` against the fake shed writes request, shed-call and first-token lines to
//! `perf.log`, slow requests warn, nothing private is logged, and `cooee perf` turns a log into its table.

mod support;

use std::{
    fmt::Write as _,
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, is_item, scratch, version};

fn lines(dir: &Path) -> Vec<Value> {
    fs::read_to_string(dir.join("data/perf.log"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn named<'a>(lines: &'a [Value], kind: &str, name: &str) -> Vec<&'a Value> {
    lines
        .iter()
        .filter(|l| l["kind"] == kind && l["name"] == name)
        .collect()
}

/// Starts a thread with `text` and waits for its reply; its id.
fn create(cooee: &Cooee, folder: &Path, text: &str) -> String {
    let new =
        json!({ "agent": "fake-acp", "folder": folder, "text": text, "random_id": "start-1" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let cursor = cooee.json(&format!("/threads/{id}/messages"))["cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let boot = cursor.split('.').next().unwrap();
    cooee
        .events(&id, &format!("{boot}.0"), false)
        .until(|e| is_item(e, "agent"));
    id
}

#[test]
fn requests_and_shed_calls_are_timed_by_route_and_method_and_slow_ones_warn() {
    let dir = scratch("perf-slow");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start_with(&dir, &shed.dir, "[perf]\nslow_ms = 0\n");
    let id = create(&cooee, &dir.join("root/project"), "hi");
    assert_eq!(cooee.get(&format!("/threads/{id}/messages?limit=5")).0, 200);
    let log = lines(&dir);

    let messages = named(&log, "http", "GET /threads/{id}/messages");
    assert!(!messages.is_empty(), "{log:?}");
    for l in &messages {
        assert_eq!((&l["level"], &l["status"]), (&json!("warn"), &json!(200)));
        assert!(l["ms"].is_u64() && l["at"].is_u64(), "{l}");
    }
    let created = named(&log, "http", "POST /threads");
    assert_eq!(created.len(), 1, "{log:?}");
    let err = fs::read_to_string(dir.join("cooee.err")).unwrap();
    assert!(err.contains("cooee: slow: POST /threads 200 "), "{err}");

    let via = |name: &str, via: &str| {
        let calls = named(&log, "shed", name);
        assert!(
            calls.iter().any(|l| l["via"] == via),
            "{name} {via}: {log:?}"
        );
    };
    via("hello", "socket");
    via("run", "cli");
    via("wait", "socket");
    via("state", "socket");
    via("send", "socket");
    assert!(named(&log, "shed", "run").iter().all(|l| l["via"] == "cli"));
}

#[test]
fn a_fast_request_under_the_default_threshold_is_debug_only() {
    let dir = scratch("perf-fast");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    assert_eq!(cooee.get("/threads").0, 200);
    let log = lines(&dir);
    let list = named(&log, "http", "GET /threads");
    assert_eq!(list.len(), 1, "{log:?}");
    assert_eq!(list[0]["level"], "debug");
    let err = fs::read_to_string(dir.join("cooee.err")).unwrap();
    assert!(!err.contains("slow: GET /threads "), "{err}");
}

#[test]
fn first_token_is_logged_once_per_prompt_and_nothing_private_is() {
    let dir = scratch("perf-first");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let secret = "SECRET-PROMPT-7f3a";
    let id = create(&cooee, &dir.join("root/project"), &format!("hi {secret}"));
    let cursor = cooee.json(&format!("/threads/{id}/messages"))["cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut events = cooee.events(&id, &cursor, false);
    let send = json!({ "text": format!("again {secret}"), "random_id": "r2" });
    assert_eq!(cooee.post(&format!("/threads/{id}/send"), &send).0, 200);
    events.until(|e| is_item(e, "agent"));

    let log = lines(&dir);
    let first = named(&log, "first_token", "fake-acp");
    assert_eq!(first.len(), 2, "{log:?}");
    assert!(
        first
            .iter()
            .all(|l| l["thread"] == id.as_str() && l["ms"].is_u64())
    );

    let text = fs::read_to_string(dir.join("data/perf.log")).unwrap();
    let token = cooee.cookie.strip_prefix("cooee=").unwrap();
    for private in [secret, token, "limit=", "root/project", "Hello"] {
        assert!(!text.contains(private), "{private} logged: {text}");
    }
    assert!(text.contains("/threads/{id}/send"), "{text}");
}

#[test]
fn cooee_perf_turns_a_log_into_the_table() {
    let dir = scratch("perf-table");
    fs::create_dir_all(dir.join("data")).unwrap();
    let config = dir.join("config.toml");
    let data = dir.join("data");
    fs::write(&config, format!("data = {:?}\n", data.to_string_lossy())).unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let ago = |min: u128| now - min * 60_000;
    let mut log = String::new();
    for (i, ms) in [5, 1, 9, 3, 7, 2, 8, 4, 6, 10].iter().enumerate() {
        let l = json!({ "at": ago(1 + i as u128), "level": "debug", "kind": "http", "name": "GET /threads", "status": 200, "ms": ms });
        writeln!(log, "{l}").unwrap();
    }
    let lines = [
        json!({ "at": ago(2), "level": "warn", "kind": "http", "name": "POST /threads", "status": 200, "ms": 900 }),
        json!({ "at": ago(3), "level": "debug", "kind": "shed", "name": "get", "via": "socket", "ms": 2 }),
        json!({ "at": ago(3), "level": "debug", "kind": "shed", "name": "agents", "via": "cli", "ms": 40 }),
        json!({ "at": ago(4), "level": "debug", "kind": "first_token", "name": "fake-acp", "thread": "t1", "ms": 700 }),
        json!({ "at": ago(5), "level": "debug", "kind": "first_token", "name": "fake-acp", "thread": "t1", "ms": 300 }),
        // Older than the window.
        json!({ "at": ago(90), "level": "debug", "kind": "shed", "name": "get", "via": "socket", "ms": 999 }),
    ];
    for l in lines {
        writeln!(log, "{l}").unwrap();
    }
    log.push_str("not json\n");
    fs::write(data.join("perf.log"), log).unwrap();

    let out = std::process::Command::new(env!("CARGO_BIN_EXE_cooee"))
        .arg("--config")
        .arg(&config)
        .arg("perf")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "| kind | name | n | p50 ms | p95 ms | max ms |
|---|---|--:|--:|--:|--:|
| first_token | fake-acp | 2 | 300 | 700 | 700 |
| http | GET /threads | 10 | 5 | 10 | 10 |
| http | POST /threads | 1 | 900 | 900 | 900 |
| shed | agents (cli) | 1 | 40 | 40 | 40 |
| shed | get (socket) | 1 | 2 | 2 | 2 |
"
    );

    let out = std::process::Command::new(env!("CARGO_BIN_EXE_cooee"))
        .arg("--config")
        .arg(&config)
        .args(["perf", "--minutes", "3"])
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("| http | GET /threads | 2 | 1 | 5 | 5 |") && !text.contains("first_token"),
        "{text}"
    );
}
