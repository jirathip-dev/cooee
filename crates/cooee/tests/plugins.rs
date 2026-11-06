//! Plugins end to end: `cooee serve` with two plugins, each on a fake backend that records what it is sent. The
//! proxy carries the plugin's own token and never the device's cookie, an event stream comes through, the frame is
//! sandboxed, a backend posts with its token or is refused, and a choice card's answer goes to its own plugin.

mod support;

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    process::Command,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

/// A plugin backend on a free port: every request's head (request line and lowercased headers) and body.
struct Backend {
    port: u16,
    seen: Arc<Mutex<Vec<(String, String)>>>,
}

impl Backend {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&seen);
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                answer(stream, &log);
            }
        });
        Self { port, seen }
    }

    fn seen(&self) -> Vec<(String, String)> {
        self.seen.lock().unwrap().clone()
    }
}

fn answer(stream: TcpStream, log: &Mutex<Vec<(String, String)>>) {
    let mut r = BufReader::new(stream);
    let (mut head, mut line) = (String::new(), String::new());
    while r.read_line(&mut line).unwrap() > 2 {
        head.push_str(&line.to_ascii_lowercase());
        line.clear();
    }
    let len = head
        .lines()
        .find_map(|l| l.strip_prefix("content-length: "))
        .map_or(0, |n| n.trim().parse().unwrap());
    let mut body = vec![0; len];
    r.read_exact(&mut body).unwrap();
    let path = head.split(' ').nth(1).unwrap_or_default().to_owned();
    log.lock()
        .unwrap()
        .push((head, String::from_utf8(body).unwrap()));
    let (kind, text) = match path.as_str() {
        "/tab.js" => (
            "text/javascript",
            "document.body.append('module ran');".to_owned(),
        ),
        "/events" => (
            "text/event-stream",
            ": open\n\nid: 7\nevent: tick\ndata: one\ndata: two\n\n".to_owned(),
        ),
        _ => ("application/json", json!({ "path": path }).to_string()),
    };
    let mut s = r.into_inner();
    let _ = write!(s, "HTTP/1.0 200 OK\r\nContent-Type: {kind}\r\n\r\n{text}");
}

const TOKEN_A: &str = "token-a-0123456789abcdef";
const TOKEN_B: &str = "token-b-0123456789abcdef";

struct Setup {
    dir: std::path::PathBuf,
    shed: FakeShed,
    cooee: Cooee,
    a: Backend,
    b: Backend,
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let (a, b) = (Backend::start(), Backend::start());
    let plugin = |name: &str, label: &str, port: u16, token: &str| {
        format!(
            "[[plugins]]\nname = \"{name}\"\nlabel = \"{label}\"\nurl = \"http://127.0.0.1:{port}\"\nmodule = \"/tab.js\"\ntoken = \"{token}\"\n"
        )
    };
    let extra = plugin("a", "Alpha", a.port, TOKEN_A) + &plugin("b", "Beta", b.port, TOKEN_B);
    let cooee = Cooee::start_with(&dir, &shed.dir, &extra);
    Setup {
        dir,
        shed,
        cooee,
        a,
        b,
    }
}

fn status(head: &str) -> u16 {
    head[9..12].parse().unwrap()
}

impl Setup {
    /// POSTs `body` to `path` as a plugin backend would: with `auth` (maybe none) and no cookie.
    fn as_backend(&self, path: &str, auth: &str, body: &Value) -> (u16, String) {
        let json = ("Content-Type", "application/json");
        let headers: &[(&str, &str)] = if auth.is_empty() {
            &[json]
        } else {
            &[json, ("Authorization", auth)]
        };
        let (head, body) = self.cooee.raw("POST", path, headers, &body.to_string());
        (status(&head), body)
    }
}

#[test]
fn the_proxy_sends_the_plugins_own_token_and_never_the_cookie() {
    let s = setup("plugin-proxy");
    let (code, body) = s.cooee.post("/p/a/api/echo?x=1", &json!({ "n": 1 }));
    assert_eq!((code, body.as_str()), (200, r#"{"path":"/echo?x=1"}"#));
    let (head, sent) = s.a.seen().pop().unwrap();
    assert!(head.starts_with("post /echo?x=1 http/1.0\r\n"), "{head}");
    assert!(
        head.contains(&format!("authorization: bearer {TOKEN_A}\r\n")),
        "{head}"
    );
    assert!(head.contains("content-type: application/json"), "{head}");
    assert!(
        !head.contains("cookie") && !head.contains(&s.cooee.cookie),
        "{head}"
    );
    assert_eq!(sent, r#"{"n":1}"#);

    // The other plugin's backend gets its own token; neither sees the other's calls.
    assert_eq!(s.cooee.get("/p/b/api/echo").0, 200);
    let (head, _) = s.b.seen().pop().unwrap();
    assert!(
        head.contains(&format!("authorization: bearer {TOKEN_B}\r\n")),
        "{head}"
    );
    assert_eq!(s.a.seen().len(), 1);
    assert!(s.b.seen().iter().all(|(h, _)| !h.contains(TOKEN_A)));

    // An unpaired caller doesn't get through; an unknown plugin is not found.
    let (head, _) = s.cooee.raw("GET", "/p/a/api/echo", &[], "");
    assert_eq!(status(&head), 401);
    assert_eq!(s.a.seen().len(), 1);
    assert_eq!(s.cooee.get("/p/nope/api/echo").0, 404);
}

#[test]
fn a_backends_event_stream_comes_through_the_proxy() {
    let s = setup("plugin-sse");
    let mut stream = TcpStream::connect(&s.cooee.addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    write!(
        stream,
        "GET /p/a/api/events HTTP/1.0\r\nHost: {}\r\nCookie: {}\r\nLast-Event-ID: 6\r\n\r\n",
        s.cooee.addr, s.cooee.cookie
    )
    .unwrap();
    let mut lines = BufReader::new(stream).lines().map(Result::unwrap);
    let head: Vec<String> = lines.by_ref().take_while(|l| !l.is_empty()).collect();
    assert!(head[0].contains(" 200 "), "{head:?}");
    let event: Vec<String> = lines
        .skip_while(|l| l.is_empty() || l.starts_with(':'))
        .take_while(|l| !l.is_empty())
        .collect();
    assert!(event.contains(&"id: 7".to_owned()), "{event:?}");
    let data = event.iter().find_map(|l| l.strip_prefix("data: ")).unwrap();
    let data: Value = serde_json::from_str(data).unwrap();
    assert_eq!(data, json!({ "event": "tick", "data": "one\ntwo" }));
    let (head, _) = s.a.seen().pop().unwrap();
    assert!(head.contains("last-event-id: 6\r\n"), "{head}");
}

#[test]
fn the_tab_frames_the_module_in_a_sandbox() {
    let s = setup("plugin-frame");
    let (code, page) = s.cooee.get("/");
    assert_eq!(code, 200);
    assert!(
        page.contains(r#"<a href="/p/a""#) && page.contains("Alpha") && page.contains("Beta"),
        "{page}"
    );
    let (code, page) = s.cooee.get("/p/a");
    assert_eq!(code, 200);
    assert!(
        page.contains(r#"sandbox="allow-scripts""#) && page.contains(r#"data-src="/p/a/frame""#),
        "{page}"
    );

    let (head, page) = s
        .cooee
        .raw("GET", "/p/a/frame", &[("Cookie", &s.cooee.cookie)], "");
    assert_eq!(status(&head), 200, "{head}");
    let csp = head
        .lines()
        .find_map(|l| l.strip_prefix("content-security-policy: "))
        .unwrap();
    for rule in [
        "sandbox allow-scripts;",
        "default-src 'none';",
        "script-src 'sha256-",
    ] {
        assert!(csp.contains(rule), "{csp}");
    }
    assert!(
        !csp.contains("allow-same-origin") && !csp.contains("connect-src"),
        "{csp}"
    );
    assert!(
        page.contains("globalThis.cooee") && page.contains("document.body.append('module ran');"),
        "{page}"
    );
    let (head, _) = s.a.seen().pop().unwrap();
    assert!(
        head.starts_with("get /tab.js ") && head.contains(TOKEN_A),
        "{head}"
    );
}

#[test]
fn a_plugin_without_its_token_is_refused() {
    let s = setup("plugin-token");
    let push = json!({ "text": "build green", "path": "lane/3" });
    let card = json!({ "thread": "t1", "text": "Ship it?", "options": ["yes"] });
    for auth in ["", "Bearer", "Bearer wrong-token-0123456789", TOKEN_A] {
        assert_eq!(s.as_backend("/plugin/push", auth, &push).0, 401, "{auth}");
        assert_eq!(s.as_backend("/plugin/card", auth, &card).0, 401, "{auth}");
    }
    // A paired device's cookie is not a plugin's token.
    assert_eq!(s.cooee.post("/plugin/push", &push).0, 401);

    let (code, note) = s.as_backend("/plugin/push", &format!("Bearer {TOKEN_A}"), &push);
    assert_eq!(code, 200, "{note}");
    assert_eq!(
        serde_json::from_str::<Value>(&note).unwrap(),
        json!({ "title": "Alpha", "body": "build green", "tag": "plugin-a", "url": "/p/a#lane/3", "item": 0 })
    );
}

#[test]
fn a_plugin_with_no_token_configured_does_not_start() {
    let dir = scratch("plugin-config");
    let config = dir.join("config.toml");
    std::fs::write(
        &config,
        "port = 0\n[[plugins]]\nname = \"a\"\nlabel = \"A\"\nurl = \"http://127.0.0.1:1\"\nmodule = \"/tab.js\"\ntoken = \"\"\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
        .args(["serve", "--config"])
        .arg(&config)
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success() && err.contains("`token`"), "{err}");
}

#[test]
fn a_choice_card_goes_into_the_thread_and_its_answer_to_its_own_plugin() {
    let s = setup("plugin-card");
    let new = json!({ "agent": "fake-acp", "folder": s.dir.join("root/project"), "text": "Say hello", "random_id": "r0" });
    let (code, body) = s.cooee.post("/threads", &new);
    assert_eq!(code, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let card = |token: &str, text: &str| {
        let card = json!({ "thread": id, "text": text, "options": ["yes", "no"] });
        let (code, body) = s.as_backend("/plugin/card", &format!("Bearer {token}"), &card);
        assert_eq!(code, 200, "{body}");
        serde_json::from_str::<Value>(&body).unwrap()
    };
    let a = card(TOKEN_A, "Ship it?");
    let b = card(TOKEN_B, "Merge it?");
    assert_eq!(
        (&a["kind"], &a["plugin"]),
        (&json!("approval"), &json!("a"))
    );
    let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    assert!(
        items
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["tool"] == "Merge it?" && i["plugin"] == "b"),
        "{items}"
    );

    let tap = json!({ "item": a["id"], "option": "yes" });
    let path = format!("/threads/{id}/approve");
    let (code, body) = s.cooee.post(&path, &tap);
    assert_eq!(code, 200, "{body}");
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["chosen"],
        "yes"
    );
    assert_eq!(s.cooee.post(&path, &tap).0, 200);
    let choices = |b: &Backend| -> Vec<(String, String)> {
        b.seen()
            .into_iter()
            .filter(|(h, _)| h.starts_with("post /cooee/choice "))
            .collect()
    };
    let sent = choices(&s.a);
    assert_eq!(sent.len(), 1, "a second tap sends nothing");
    assert!(sent[0].0.contains(TOKEN_A), "{}", sent[0].0);
    assert_eq!(
        serde_json::from_str::<Value>(&sent[0].1).unwrap(),
        json!({ "thread": id, "item": a["id"], "option": "yes" })
    );
    assert!(
        choices(&s.b).is_empty(),
        "plugin b's backend hears nothing of a's card"
    );
    assert!(b["chosen"].is_null());
    let to_shed = s.shed.sent(&format!("cooee-{id}"));
    assert!(
        to_shed.iter().all(|t| !t.contains("approve")),
        "{to_shed:?}"
    );
}

#[test]
fn the_docs_show_the_example_plugin_the_ui_tests_run() {
    let doc = include_str!("../../../docs/plugins.md");
    for file in [
        include_str!("../../../docs/plugins/hello/server.mjs"),
        include_str!("../../../docs/plugins/hello/tab.js"),
    ] {
        assert!(doc.contains(file), "docs/plugins.md is out of date: {file}");
    }
}

#[test]
fn a_plugin_names_its_push_tag_and_puts_a_dot_on_its_own_tab() {
    let s = setup("plugin-dot");
    let a = format!("Bearer {TOKEN_A}");
    let push = json!({ "text": "air offline", "path": "air", "tag": "machine" });
    let (code, note) = s.as_backend("/plugin/push", &a, &push);
    assert_eq!(code, 200, "{note}");
    let note: Value = serde_json::from_str(&note).unwrap();
    assert_eq!(
        (&note["tag"], &note["url"]),
        (&json!("machine"), &json!("/p/a#air"))
    );

    let dot = "Alpha<i class=\"dot\" aria-hidden=\"true\">●</i>";
    assert!(!s.cooee.get("/").1.contains("class=\"dot\""));
    assert_eq!(
        s.as_backend("/plugin/dot", "", &json!({ "on": true })).0,
        401
    );
    assert_eq!(
        s.as_backend("/plugin/dot", &a, &json!({ "on": true })).0,
        200
    );
    let home = s.cooee.get("/").1;
    assert!(
        home.contains(dot) && home.matches("class=\"dot\"").count() == 1,
        "{home}"
    );
    assert!(s.cooee.get("/p/b").1.contains(dot));
    assert_eq!(
        s.as_backend("/plugin/dot", &a, &json!({ "on": false })).0,
        200
    );
    assert!(!s.cooee.get("/").1.contains("class=\"dot\""));
}
