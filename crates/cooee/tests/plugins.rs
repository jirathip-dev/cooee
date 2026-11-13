//! Plugins end to end: `cooee serve` with two plugins, each on a fake backend that records what it is sent. The
//! proxy carries the plugin's own token and never the device's cookie, an event stream comes through, the frame is
//! sandboxed, a backend posts with its token or is refused, and a choice card's answer goes to its own plugin.

mod support;

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, is_item, others_go_on, scratch, version};

/// A plugin backend on a free port: every request's head (request line and lowercased headers) and body. Its
/// next answer to `/cooee/choice` is `choice`'s, when set; while `hang` is set it answers no choice.
struct Backend {
    port: u16,
    seen: Arc<Mutex<Vec<(String, String)>>>,
    choice: Arc<Mutex<Option<(u16, Value)>>>,
    hang: Arc<AtomicBool>,
}

impl Backend {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let choice = Arc::new(Mutex::new(None));
        let hang = Arc::new(AtomicBool::new(false));
        let (log, next, held) = (Arc::clone(&seen), Arc::clone(&choice), Arc::clone(&hang));
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (log, next, held) = (Arc::clone(&log), Arc::clone(&next), Arc::clone(&held));
                thread::spawn(move || answer(stream, &log, &next, &held));
            }
        });
        Self {
            port,
            seen,
            choice,
            hang,
        }
    }

    /// The `/cooee/choice` bodies it was sent.
    fn choices(&self) -> Vec<Value> {
        self.seen()
            .into_iter()
            .filter(|(h, _)| h.starts_with("post /cooee/choice "))
            .map(|(_, b)| serde_json::from_str(&b).unwrap())
            .collect()
    }

    fn seen(&self) -> Vec<(String, String)> {
        self.seen.lock().unwrap().clone()
    }
}

fn answer(
    stream: TcpStream,
    log: &Mutex<Vec<(String, String)>>,
    choice: &Mutex<Option<(u16, Value)>>,
    hang: &AtomicBool,
) {
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
    while path == "/cooee/choice" && hang.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(20));
    }
    if path == "/cooee/choice"
        && let Some((code, v)) = choice.lock().unwrap().take()
    {
        let mut s = r.into_inner();
        let _ = write!(
            s,
            "HTTP/1.0 {code} X\r\nContent-Type: application/json\r\n\r\n{v}"
        );
        return;
    }
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
    // Both plugins attached to persona `chief` (#129): their cards go to its threads only.
    let chief = json!({ "name": "Chief", "agent": "fake-acp", "plugins": ["a", "b"] });
    let (code, body) = cooee.post("/personas", &chief);
    assert_eq!(code, 200, "{body}");
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
    let id = new_thread(&s, "Say hello");

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
        json!({ "thread": id, "item": a["id"], "option": "yes", "by": me(&s) })
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

/// This device's label, as an answer's `by` names it.
fn me(s: &Setup) -> String {
    let devices = s.cooee.json("/devices");
    let me = devices.as_array().unwrap().iter().find(|d| d["me"] == true);
    me.unwrap()["label"].as_str().unwrap().to_owned()
}

/// A new thread's id: one of persona `chief`'s.
fn new_thread(s: &Setup, text: &str) -> String {
    thread_of(s, "chief", text)
}

/// A new thread of persona `persona` (none: Plain), once its agent has answered: its id.
fn thread_of(s: &Setup, persona: &str, text: &str) -> String {
    let random = format!("r-{persona}-{text}");
    let new = json!({ "agent": "fake-acp", "profile": persona, "folder": s.dir.join("root/project"), "text": text, "random_id": random });
    let (code, body) = s.cooee.post("/threads", &new);
    assert_eq!(code, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let cursor = s.cooee.json(&format!("/threads/{id}/messages"))["cursor"].clone();
    let boot = cursor.as_str().unwrap().split('.').next().unwrap();
    let mut events = s.cooee.events(&id, &format!("{boot}.0"), false);
    events.until(|e| is_item(e, "agent"));
    id
}

impl Setup {
    /// Plugin a's card: its status and answer.
    fn card(&self, card: &Value) -> (u16, Value) {
        let (code, body) = self.as_backend("/plugin/card", &format!("Bearer {TOKEN_A}"), card);
        (code, serde_json::from_str(&body).unwrap_or(Value::Null))
    }

    /// The owner's answer to `item` in `thread`.
    fn tap(&self, thread: &str, answer: &Value) -> Value {
        let (code, body) = self
            .cooee
            .post(&format!("/threads/{thread}/approve"), answer);
        assert_eq!(code, 200, "{body}");
        serde_json::from_str(&body).unwrap()
    }
}

/// #553 row 30: the recommended choice comes first and says so; the "other" box's text goes to the plugin like a
/// choice; the answer keeps who (this device) and when.
#[test]
fn row_30_choice_rows_recommended_first_other_text_and_who_and_when() {
    let s = setup("card-30");
    let id = new_thread(&s, "Say hello");
    let (code, card) = s.card(
        &json!({ "thread": id, "text": "Which first?", "options": ["#43", "#44"], "recommend": "#44" }),
    );
    assert_eq!(code, 200, "{card}");
    assert_eq!(
        card["options"],
        json!([{ "id": "#44", "name": "#44", "kind": "recommended" }, { "id": "#43", "name": "#43", "kind": "" }])
    );
    let answer = s.tap(
        &id,
        &json!({ "item": card["id"], "option": "neither: #48 first" }),
    );
    assert_eq!(
        s.a.choices(),
        [json!({ "thread": id, "item": card["id"], "option": "neither: #48 first", "by": me(&s) })]
    );
    assert_eq!(
        (&answer["chosen"], &answer["by"]),
        (&json!("neither: #48 first"), &json!(me(&s)))
    );
    assert!(answer["answered"].as_u64().unwrap() >= card["at"].as_u64().unwrap());
}

/// #553 row 31: a card the conductor answered itself shows its answer with [change] for 30 minutes; the owner's
/// change goes to the plugin, and is the last.
#[test]
fn row_31_a_conductors_answer_takes_one_change_from_the_owner() {
    let s = setup("card-31");
    let id = new_thread(&s, "Say hello");
    let (_, card) = s.card(&json!({
        "thread": id, "text": "answered q12 for #44: split", "options": ["split", "keep"],
        "answered": "split", "by": "conductor-t6"
    }));
    assert_eq!(
        (&card["chosen"], &card["by"]),
        (&json!("split"), &json!("conductor-t6"))
    );
    let at = card["answered"].as_u64().unwrap();
    assert_eq!(card["until"].as_u64(), Some(at + 30 * 60 * 1000));
    assert!(
        s.a.choices().is_empty(),
        "the conductor's own answer isn't sent back"
    );

    let changed = s.tap(&id, &json!({ "item": card["id"], "option": "keep" }));
    assert_eq!(
        (&changed["chosen"], &changed["by"], &changed["until"]),
        (&json!("keep"), &json!(me(&s)), &Value::Null)
    );
    let again = s.tap(&id, &json!({ "item": card["id"], "option": "split" }));
    assert_eq!(again["chosen"], "keep");
    assert_eq!(
        s.a.choices(),
        [json!({ "thread": id, "item": card["id"], "option": "keep", "by": me(&s) })]
    );
}

/// #553 row 32: a config card carries its diff and waits (the slice and the thread list say so); [apply] settles
/// it with the plugin's line; a plugin's 409 leaves it stale, with no buttons.
#[test]
fn row_32_config_card_waits_applies_with_the_plugins_line_or_goes_stale() {
    let s = setup("card-32");
    let id = new_thread(&s, "Say hello");
    let no_diff = json!({ "thread": id, "text": "config c1", "form": "config" });
    assert_eq!(s.card(&no_diff).0, 400);
    let diff = "-max_impl = 2\n+max_impl = 3";
    let (_, c1) =
        s.card(&json!({ "thread": id, "text": "config c1", "form": "config", "detail": diff }));
    assert_eq!(
        c1["options"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| &o["id"])
            .collect::<Vec<_>>(),
        ["apply", "discard"]
    );
    assert_eq!(
        s.cooee.json(&format!("/threads/{id}/messages"))["waiting"]["id"],
        c1["id"]
    );
    let row = |list: &Value| -> Value {
        let rows = list.as_array().unwrap().iter();
        rows.into_iter().find(|r| r["id"] == id).unwrap().clone()
    };
    assert_eq!(row(&s.cooee.json("/threads"))["last"], "config c1 waiting");

    let line = "config c1 applied by owner, backup kept, doctor ✓";
    *s.a.choice.lock().unwrap() = Some((200, json!({ "line": line })));
    let applied = s.tap(&id, &json!({ "item": c1["id"], "option": "apply" }));
    assert_eq!(
        (&applied["chosen"], &applied["line"]),
        (&json!("apply"), &json!(line))
    );
    assert!(s.cooee.json(&format!("/threads/{id}/messages"))["waiting"].is_null());

    let (_, c2) =
        s.card(&json!({ "thread": id, "text": "config c2", "form": "config", "detail": diff }));
    *s.a.choice.lock().unwrap() = Some((409, json!({ "line": "stale: fleet.toml changed since" })));
    let stale = s.tap(&id, &json!({ "item": c2["id"], "option": "apply" }));
    assert_eq!(
        (&stale["chosen"], &stale["line"]),
        (&Value::Null, &json!("stale: fleet.toml changed since"))
    );
    assert!(s.cooee.json(&format!("/threads/{id}/messages"))["waiting"].is_null());
    assert_eq!(
        s.tap(&id, &json!({ "item": c2["id"], "option": "apply" }))["line"],
        stale["line"]
    );
    assert_eq!(s.a.choices().len(), 2, "a stale card takes no second tap");
}

/// #553 row 33: a secret card's value goes to its plugin and nowhere else: not the item, not the thread's files.
#[test]
fn row_33_a_secrets_value_reaches_its_plugin_only() {
    let s = setup("card-33");
    let id = new_thread(&s, "Say hello");
    let no_name = json!({ "thread": id, "text": "for the bot", "form": "secret" });
    assert_eq!(s.card(&no_name).0, 400);
    let (_, card) = s.card(
        &json!({ "thread": id, "text": "for the bot", "form": "secret", "detail": "BOT_TOKEN" }),
    );
    let value = "v4lue-only-the-plugin-sees";
    let saved = s.tap(
        &id,
        &json!({ "item": card["id"], "option": "save", "value": value }),
    );
    assert_eq!(
        (&saved["chosen"], &saved["detail"]),
        (&json!("save"), &json!("BOT_TOKEN"))
    );
    assert_eq!(s.a.choices()[0]["value"], value);
    for f in std::fs::read_dir(s.dir.join("data/threads")).unwrap() {
        let text = std::fs::read_to_string(f.unwrap().path()).unwrap();
        assert!(!text.contains(value));
    }
    assert!(
        !s.cooee
            .get(&format!("/threads/{id}/messages"))
            .1
            .contains(value)
    );
}

/// #553 row 35: a handoff puts `continued in` in the source and the text `from` it in the target; a line flows
/// back; a pick-a-thread card offers the other open threads.
#[test]
fn row_35_handoff_chips_flow_back_lines_and_pick_a_thread() {
    let s = setup("card-35");
    let (t5, t6) = (
        new_thread(&s, "Plan the week"),
        new_thread(&s, "Fix the build"),
    );
    let auth = format!("Bearer {TOKEN_A}");
    let handoff = json!({ "from": t5, "to": t6, "text": "take the build" });
    let (code, body) = s.as_backend("/plugin/handoff", &auth, &handoff);
    assert_eq!(code, 200, "{body}");
    let last = |id: &str| {
        s.cooee.json(&format!("/threads/{id}/messages"))["items"]
            .as_array()
            .unwrap()
            .last()
            .cloned()
            .unwrap()
    };
    let link = |v: Value| {
        (
            v["kind"].clone(),
            v["text"].clone(),
            v["thread"].clone(),
            v["title"].clone(),
            v["out"].clone(),
        )
    };
    assert_eq!(
        link(last(&t5)),
        link(
            json!({ "kind": "link", "text": "", "thread": t6, "title": "Fix the build", "out": true })
        )
    );
    assert_eq!(
        link(last(&t6)),
        link(
            json!({ "kind": "link", "text": "take the build", "thread": t5, "title": "Plan the week", "out": false })
        )
    );
    let flow = json!({ "thread": t5, "from": t6, "text": "filed #61: build cache" });
    assert_eq!(s.as_backend("/plugin/line", &auth, &flow).0, 200);
    assert_eq!(
        link(last(&t5)),
        link(
            json!({ "kind": "link", "text": "filed #61: build cache", "thread": t6, "title": "Fix the build", "out": false })
        )
    );

    let (_, pick) =
        s.card(&json!({ "thread": t5, "text": "Continue in which thread?", "form": "thread" }));
    assert_eq!(
        pick["options"],
        json!([{ "id": t6, "name": "Fix the build", "kind": "" }])
    );
}

/// #83: a herdsman run card shows its command and sha; [approve] goes to the plugin once, carrying the card's sha
/// (never the page's value), and settles with the exit code and output tail; a second approve sends nothing; [deny]
/// carries no sha; an expired card's 409 leaves it stale.
#[test]
fn a_run_card_approves_once_with_its_sha_and_shows_the_tail() {
    let s = setup("card-83");
    let id = new_thread(&s, "Say hello");
    let sha = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
    let run = |sha: Option<&str>| {
        let mut c = json!({ "thread": id, "text": "run r1: re-tag the release", "form": "run-card", "detail": "git tag -f v1" });
        if let Some(sha) = sha {
            c["sha"] = sha.into();
        }
        c
    };
    assert_eq!(s.card(&run(None)).0, 400, "a run card needs its sha");
    let (_, r1) = s.card(&run(Some(sha)));
    assert_eq!(
        (&r1["form"], &r1["detail"], &r1["sha"]),
        (&json!("run-card"), &json!("git tag -f v1"), &json!(sha))
    );
    assert_eq!(
        r1["options"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| &o["id"])
            .collect::<Vec<_>>(),
        ["approve", "deny"]
    );

    let line = "run r1 approved by owner, ran in /repo: exit 0\n```\nUpdated tag 'v1'\n```";
    *s.a.choice.lock().unwrap() = Some((200, json!({ "line": line })));
    let ran = s.tap(
        &id,
        &json!({ "item": r1["id"], "option": "approve", "value": "0000" }),
    );
    assert_eq!(
        (&ran["chosen"], &ran["line"], &ran["by"]),
        (&json!("approve"), &json!(line), &json!(me(&s)))
    );
    let again = s.tap(&id, &json!({ "item": r1["id"], "option": "approve" }));
    assert_eq!(again["line"], line);
    assert_eq!(
        s.a.choices(),
        [
            json!({ "thread": id, "item": r1["id"], "option": "approve", "by": me(&s), "value": sha })
        ],
        "approved once, with the card's sha"
    );

    let (_, r2) = s.card(&run(Some(sha)));
    let denied = s.tap(&id, &json!({ "item": r2["id"], "option": "deny" }));
    assert_eq!(denied["chosen"], "deny");
    assert_eq!(
        s.a.choices()[1],
        json!({ "thread": id, "item": r2["id"], "option": "deny", "by": me(&s) })
    );

    let (_, r3) = s.card(&run(Some(sha)));
    let expired = "r3 expired 15 min after it was posted; post a new card";
    *s.a.choice.lock().unwrap() = Some((409, json!({ "line": expired })));
    let stale = s.tap(&id, &json!({ "item": r3["id"], "option": "approve" }));
    assert_eq!(
        (&stale["chosen"], &stale["line"]),
        (&Value::Null, &json!(expired))
    );
    s.tap(&id, &json!({ "item": r3["id"], "option": "approve" }));
    assert_eq!(
        s.a.choices().len(),
        3,
        "a stale card takes no second approve"
    );
}

/// #129: a plugin's card goes only to a thread of a persona it is attached to; any other is refused, with why.
#[test]
fn a_card_to_a_thread_of_a_persona_its_plugin_isnt_attached_to_is_refused() {
    let s = setup("card-unattached");
    let only_b = json!({ "name": "Bee", "agent": "fake-acp", "plugins": ["b"] });
    assert_eq!(s.cooee.post("/personas", &only_b).0, 200);
    for id in [
        thread_of(&s, "bee", "Say hello"),
        thread_of(&s, "", "Say hi"),
    ] {
        let card = json!({ "thread": id, "text": "Ship it?", "options": ["yes"] });
        let (code, why) = s.as_backend("/plugin/card", &format!("Bearer {TOKEN_A}"), &card);
        assert_eq!(code, 400, "{why}");
        assert!(why.contains("plugin a is not attached"), "{why}");
        let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
        assert!(!items.to_string().contains("Ship it?"), "{items}");
    }
    let card =
        json!({ "thread": thread_of(&s, "bee", "Again"), "text": "Merge?", "options": ["yes"] });
    let (code, body) = s.as_backend("/plugin/card", &format!("Bearer {TOKEN_B}"), &card);
    assert_eq!(code, 200, "{body}");
}

/// `cooee plugin check` on `answer`: whether it passed, and what it printed.
fn checked(s: &Setup, answer: &Value) -> (bool, String) {
    let file = s.dir.join("answer.json");
    std::fs::write(&file, answer.to_string()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
        .args(["plugin", "check"])
        .arg(&file)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

/// #138: cooee's own choice, config, secret and run cards go out as #125 card blocks the schema passes, in the
/// card's answer and in the thread's slice; once settled, a card shows its line and no longer asks.
#[test]
fn cooees_own_cards_go_out_as_blocks_and_settle_to_their_line() {
    let s = setup("card-blocks");
    let id = new_thread(&s, "Say hello");
    let cards = [
        json!({ "thread": id, "text": "Which first?", "options": ["#43", "#44"], "recommend": "#44" }),
        json!({ "thread": id, "text": "config c1", "form": "config", "detail": "+max_impl = 3" }),
        json!({ "thread": id, "text": "for the bot", "form": "secret", "detail": "BOT_TOKEN" }),
        json!({ "thread": id, "text": "re-tag", "form": "run-card", "detail": "git tag -f v1", "sha": "9f86d081" }),
    ];
    let items: Vec<Value> = cards.iter().map(|c| s.card(c).1).collect();
    let shape = |b: &Value| {
        let inner: Vec<&Value> = b["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| &x["type"])
            .collect();
        (b["kind"].clone(), json!(inner))
    };
    let blocks: Vec<&Value> = items.iter().map(|i| &i["block"]).collect();
    assert_eq!(
        blocks.iter().map(|b| shape(b)).collect::<Vec<_>>(),
        [
            (json!("ask"), json!(["choice"])),
            (json!("config"), json!(["text", "choice"])),
            (json!("secret"), json!(["secret"])),
            (json!("run"), json!(["run"])),
        ]
    );
    let choice = &blocks[0]["blocks"][0];
    assert_eq!(
        (
            &blocks[0]["id"],
            &blocks[0]["title"],
            &choice["recommended"],
            &choice["options"][0]
        ),
        (
            &json!(items[0]["id"].to_string()),
            &json!("Which first?"),
            &json!("#44"),
            &json!({ "value": "#44", "label": "#44" })
        )
    );
    assert_eq!(blocks[2]["blocks"][0]["name"], "BOT_TOKEN");
    assert_eq!(
        (
            &blocks[3]["blocks"][0]["command"],
            &blocks[3]["blocks"][0]["sha"]
        ),
        (&json!("git tag -f v1"), &json!("9f86d081"))
    );
    let (ok, said) = checked(&s, &json!(blocks));
    assert!(ok, "{said}");
    let slice = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let sent: Vec<&Value> = slice
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|i| i.get("block"))
        .collect();
    assert_eq!(sent, blocks);

    *s.a.choice.lock().unwrap() = Some((200, json!({ "line": "ran: exit 0" })));
    let ran = s.tap(&id, &json!({ "item": items[3]["id"], "option": "approve" }));
    assert_eq!(
        ran["block"]["blocks"],
        json!([{ "type": "text", "style": "footnote", "markdown": "ran: exit 0" }])
    );
}

/// #138: `cooee plugin check` passes every #125 and #184 fixture and fails a broken answer, naming the field.
#[test]
fn plugin_check_passes_the_fixtures_and_names_a_broken_field() {
    let design = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/design");
    let mut files: Vec<_> = ["screens-as-data", "fleets-config"]
        .iter()
        .flat_map(|d| std::fs::read_dir(design.join(d).join("fixtures")).unwrap())
        .map(|f| f.unwrap().path())
        .collect();
    files.sort();
    let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
        .args(["plugin", "check"])
        .args(&files)
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success()
            && said.lines().count() == 19
            && said.lines().all(|l| l.starts_with("ok")),
        "{said}"
    );

    let s = setup("card-check");
    let broken = json!({ "type": "screen", "title": "Queue", "blocks": [
        { "type": "list", "rows": [{ "type": "row", "title": "#722", "badge": { "text": "impl", "tone": "red" } }] },
        { "type": "secret", "name": "GH_TOKEN", "action": "set", "value": "leaked" },
        { "type": "editor", "name": "toml", "text": "" },
        { "type": "a-newer-block" } ] });
    let (ok, said) = checked(&s, &broken);
    assert!(!ok);
    assert_eq!(
        said.lines().skip(1).map(str::trim).collect::<Vec<_>>(),
        [
            "$.blocks[0].rows[0].badge.tone: \"red\" is not one of \"ok\", \"warn\", \"bad\", \"info\", \"mute\", \"accent\", \"working\", \"needs\", \"done\", \"fail\", \"idle\"",
            "$.blocks[1].value: must not be sent",
            "$.blocks[2].action: is required",
        ]
    );
}

/// #288: a plugin whose answer hangs holds only its own card: meanwhile another thread's send, the thread list and
/// the home page each answer within 200 ms, and that thread's reply streams in.
#[test]
fn a_hanging_plugin_answer_never_holds_another_thread() {
    let s = setup("plugin-hang");
    let id = new_thread(&s, "Say hello");
    let other = new_thread(&s, "Say hi");
    let (code, card) =
        s.card(&json!({ "thread": id, "text": "Merge it?", "options": ["yes", "no"] }));
    assert_eq!(code, 200, "{card}");
    s.a.hang.store(true, Ordering::SeqCst);
    let tap = json!({ "item": card["id"], "option": "yes" });
    let path = format!("/threads/{id}/approve");
    thread::scope(|scope| {
        let tapped = scope.spawn(|| s.cooee.post(&path, &tap));
        while s.a.choices().is_empty() {
            thread::sleep(Duration::from_millis(20));
        }
        others_go_on(&s.cooee, &other);
        assert!(!tapped.is_finished(), "the plugin's answer still hangs");
        s.a.hang.store(false, Ordering::SeqCst);
        let (code, body) = tapped.join().unwrap();
        assert_eq!(code, 200, "{body}");
    });
}

/// #226: `cooee plugin check` checks a `cooee-plugin.toml` by the manifest's rules, `attach` included.
#[test]
fn plugin_check_checks_a_manifest() {
    let s = setup("manifest-check");
    let file = s.dir.join("cooee-plugin.toml");
    let check = |text: &str| {
        std::fs::write(&file, text).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
            .args(["plugin", "check"])
            .arg(&file)
            .output()
            .unwrap();
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };
    let ok = "name = \"drover\"\nversion = \"1\"\nbuild = \"true\"\nrun = \"true\"\n\
              attach = \"http://127.0.0.1:7777\"\n[[places]]\ntype = \"menu-row\"\nlabel = \"drover\"\n\
              screen = \"/screens/fleets\"\n[permissions]\nplaces = [\"menu-row\"]\n";
    assert!(check(ok).0, "{}", check(ok).1);
    let (passed, said) = check(&ok.replace("127.0.0.1:7777", "example.com"));
    assert!(
        !passed && said.contains("`attach` must be http://127.0.0.1:<port>"),
        "{said}"
    );
}
