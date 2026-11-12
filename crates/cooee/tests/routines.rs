//! Routines (#75) end to end with the fake shed (tests/support): an agent makes one in chat with cooee's own
//! `create_routine` tool (its chat gets the routine line, Settings › routines lists it; a schedule under 15 minutes
//! apart is refused). A due time that passed runs once, into that chat, marked with the routine in the chat and the
//! thread list; a restart runs nothing again, and a run whose record was lost (a crash between the run and its
//! record) adopts the same message instead of sending a second. Settings pauses and deletes it.
//!
//! GitHub events (#261): a routine on an event is saved only once the owner approves its card with Face ID (declined:
//! never); a signed delivery on the `[hooks]` listener starts its thread once, a wrong signature nothing; the
//! catch-up reads a fake GitHub's delivery log and runs a missed delivery once, across restarts.

mod support;

use std::{
    fmt::Write as _,
    fs,
    io::{Read as _, Write as _},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use support::{Authenticator, Cooee, FakeShed, scratch, version};

/// `name` called with `args` by thread `id`'s agent through cooee's own MCP server: the tool's result.
fn call(shed: &FakeShed, id: &str, name: &str, args: &Value) -> Value {
    // A thread that starts behind its answer gets its file once its agent starts.
    let file = shed.dir.join(format!("mcp-cooee-{id}.json"));
    for _ in 0..100 {
        if file.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let servers: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
    let cooee = &servers[0];
    let args_of = cooee["args"].as_array().unwrap().iter();
    let mut child = Command::new(cooee["command"].as_str().unwrap())
        .args(args_of.map(|a| a.as_str().unwrap()))
        .env("COOEE_MCP", cooee["env"][0]["value"].as_str().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let call = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                       "params": { "name": name, "arguments": args } });
    writeln!(child.stdin.take().unwrap(), "{call}").unwrap();
    let out = child.wait_with_output().unwrap();
    serde_json::from_slice::<Value>(&out.stdout).unwrap()["result"].clone()
}

/// The routines' file, changed by `f`.
fn change(file: &Path, f: impl FnOnce(&mut Vec<Value>)) {
    let mut v: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
    f(v["routines"].as_array_mut().unwrap());
    fs::write(file, v.to_string()).unwrap();
}

/// Routine `id` once its tick recorded a due time (the runner ticks as cooee starts).
fn recorded(file: &Path, id: &str) -> Value {
    for _ in 0..100 {
        let v: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
        let r = v["routines"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == id);
        if let Some(r) = r.filter(|r| r["last_due"].as_u64().unwrap_or(0) > 0) {
            return r.clone();
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!(
        "routine {id} never ran: {}",
        fs::read_to_string(file).unwrap()
    );
}

/// Thread `id`'s messages a routine sent.
fn runs(cooee: &Cooee, id: &str) -> Vec<Value> {
    let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let items = items.as_array().unwrap().iter();
    items
        .filter(|i| i["kind"] == "you" && i["routine"].is_object())
        .cloned()
        .collect()
}

/// cooee with the fake shed and a chat with the fake agent: the folder, the shed, cooee and the chat's id.
fn chat(name: &str) -> (PathBuf, FakeShed, Cooee, String) {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let new = json!({ "agent": "fake-acp", "folder": dir.join("root/project"), "text": "hi", "random_id": "start" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    (dir, shed, cooee, id)
}

/// `days` days ago, in Unix ms.
fn ago(days: u64) -> u64 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    u64::try_from(now.as_millis()).unwrap() - days * 86_400_000
}

/// A form post to `path`, as Settings › routines sends one: the status.
fn form(cooee: &Cooee, path: &str, body: &str) -> String {
    let headers = [
        ("Content-Type", "application/x-www-form-urlencoded"),
        ("Cookie", cooee.cookie.as_str()),
    ];
    cooee.raw("POST", path, &headers, body).0[9..12].to_owned()
}

#[test]
fn a_schedule_made_in_chat_runs_once_per_due_time_even_across_restarts() {
    let (dir, shed, cooee, id) = chat("routines");
    let terminal = format!("cooee-{id}");
    shed.report(&terminal, "done", None);

    let often = json!({ "name": "Ping", "when": "*/5 * * * *", "prompt": "ping" });
    let refused = call(&shed, &id, "create_routine", &often);
    assert_eq!(refused["isError"], true, "{refused}");
    let said = refused["content"][0]["text"].as_str().unwrap();
    assert!(said.starts_with("BAD_SCHEDULE"), "{said}");
    let ask = json!({ "name": "Monday renewals", "when": "0 9 * * 1", "prompt": "Renewals due {{due}}?" });
    let made = call(&shed, &id, "create_routine", &ask);
    assert_eq!(made["isError"], false, "{made}");
    let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let line = items
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["kind"] == "routine");
    let line = line.unwrap_or_else(|| panic!("no routine line: {items}"));
    assert_eq!(
        (&line["name"], &line["when"], &line["place"]),
        (
            &json!("Monday renewals"),
            &json!("Mondays 09:00"),
            &json!("in this chat")
        )
    );
    let list = cooee.json("/routines.json");
    assert_eq!(list.as_array().unwrap().len(), 1, "{list}");
    let rid = list[0]["id"].as_str().unwrap().to_owned();
    assert_eq!(
        (&list[0]["made_in"], &list[0]["last_due"]),
        (&json!(id), &json!(0))
    );
    let (status, page) = cooee.get("/routines");
    assert_eq!(status, 200);
    assert!(
        page.contains("Monday renewals") && page.contains("Mondays 09:00"),
        "{page}"
    );
    drop(cooee);

    // Made eight days ago, so a Monday 09:00 has passed since: the tick as cooee starts runs it, into its chat.
    let eight_days_ago = ago(8);
    let file = dir.join("data/routines.json");
    change(&file, |list| list[0]["made_at"] = json!(eight_days_ago));
    let cooee = Cooee::start(&dir, &shed.dir);
    let ran = recorded(&file, &rid);
    let due = ran["last_due"].as_u64().unwrap();
    assert!(due > eight_days_ago, "{ran}");
    let run = &ran["last_run"];
    assert_eq!(
        (&run["outcome"], &run["thread"], &run["key"]),
        (
            &json!("ran"),
            &json!(id),
            &json!(format!("routine-{rid}-{due}"))
        )
    );
    let sent = runs(&cooee, &id);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0]["routine"]["name"], "Monday renewals");
    let text = sent[0]["text"].as_str().unwrap();
    assert!(
        text.starts_with("Renewals due 20") && !text.contains("{{"),
        "{text}"
    );
    let rows = cooee.json("/threads");
    let row = rows.as_array().unwrap().iter().find(|r| r["id"] == id);
    assert_eq!(row.unwrap()["routine"], "Monday renewals", "{rows}");
    let prompts = || {
        let sent = shed.sent(&terminal);
        sent.iter().filter(|t| t.contains("Renewals due")).count()
    };
    assert_eq!(prompts(), 1);
    drop(cooee);

    // A restart runs nothing again: a second routine, due too and after it in the file, marks the tick done.
    change(&file, |list| {
        let mut other = list[0].clone();
        other["id"] = json!("r-other");
        other["prompt"] = json!("Other due {{due}}");
        other["last_due"] = json!(0);
        other.as_object_mut().unwrap().remove("last_run");
        list.push(other);
    });
    let cooee = Cooee::start(&dir, &shed.dir);
    recorded(&file, "r-other");
    assert_eq!(recorded(&file, &rid), ran);
    assert_eq!((runs(&cooee, &id).len(), prompts()), (2, 1));
    drop(cooee);

    // A crash between the run and its record: the same key runs again and adopts the message already sent.
    change(&file, |list| {
        list[0]["last_due"] = json!(0);
        list[0].as_object_mut().unwrap().remove("last_run");
    });
    let cooee = Cooee::start(&dir, &shed.dir);
    assert_eq!(recorded(&file, &rid)["last_due"], due);
    let mine = runs(&cooee, &id);
    let mine = mine.iter().filter(|i| i["routine"]["id"] == rid.as_str());
    assert_eq!((mine.count(), prompts()), (1, 1));
}

#[test]
fn a_new_thread_routine_starts_a_thread_marked_with_it_in_the_list() {
    let (dir, shed, cooee, id) = chat("routines-new");
    let ask = json!({ "name": "Daily digest", "when": "30 18 * * *", "prompt": "Digest for {{due}}", "place": "new_thread" });
    let made = call(&shed, &id, "create_routine", &ask);
    assert_eq!(made["isError"], false, "{made}");
    let rid = cooee.json("/routines.json")[0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    drop(cooee);
    let file = dir.join("data/routines.json");
    change(&file, |list| list[0]["made_at"] = json!(ago(2)));
    let cooee = Cooee::start(&dir, &shed.dir);
    let ran = recorded(&file, &rid);
    let started = ran["last_run"]["thread"].as_str().unwrap().to_owned();
    assert_ne!(started, id, "{ran}");
    let rows = cooee.json("/threads");
    let row = |t: &str| {
        rows.as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == t)
            .cloned()
    };
    assert_eq!(row(&started).unwrap()["routine"], "Daily digest", "{rows}");
    assert_eq!(row(&id).unwrap()["routine"], "", "{rows}");
    // A routine's own run can't make routines.
    let again = call(&shed, &started, "create_routine", &ask);
    let said = again["content"][0]["text"].as_str().unwrap();
    assert!(said.starts_with("NOT_HERE"), "{said}");

    // Settings › routines pauses it, then deletes it; the thread it started stays.
    assert_eq!(
        form(&cooee, &format!("/routines/{rid}"), "paused=true"),
        "303"
    );
    assert_eq!(cooee.json("/routines.json")[0]["paused"]["why"], "owner");
    let bad = form(&cooee, &format!("/routines/{rid}"), "when=*%2F5+*+*+*+*");
    assert_eq!(bad, "400");
    assert_eq!(form(&cooee, &format!("/routines/{rid}/delete"), ""), "303");
    let left = cooee.json("/routines.json");
    assert_eq!(left, json!([]));
    assert!(
        cooee
            .json("/threads")
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == started)
    );
}

const SECRET: &str = "It's a Secret to Everybody";
const URL: &str = "https://mac.example:8443/hooks/github";

/// A pull request opened in owner/app, titled `title`.
fn opened(title: &str) -> String {
    json!({ "action": "opened", "repository": { "full_name": "owner/app" },
            "pull_request": { "title": title, "number": 7 } })
    .to_string()
}

/// cooee with `[hooks]` on a free port, GitHub's API at `api` and the webhook secret and token stored: it and the
/// hooks port.
fn hooked(dir: &Path, shed: &FakeShed, api: &str) -> (Cooee, u16) {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let secrets = dir.join("data/secrets");
    fs::create_dir_all(&secrets).unwrap();
    fs::write(secrets.join("GITHUB_WEBHOOK_SECRET"), SECRET).unwrap();
    fs::write(secrets.join("GITHUB_TOKEN"), "gh-test-token").unwrap();
    let extra = format!("[hooks]\nbind = \"127.0.0.1:{port}\"\nurl = {URL:?}\napi = {api:?}\n");
    (Cooee::start_with(dir, &shed.dir, &extra), port)
}

/// HMAC-SHA256 of `body` under [`SECRET`], as GitHub's `X-Hub-Signature-256`.
fn sign(body: &str) -> String {
    let mut key = [0u8; 64];
    key[..SECRET.len()].copy_from_slice(SECRET.as_bytes());
    let pad = |b: u8| key.iter().map(|k| k ^ b).collect::<Vec<_>>();
    let inner = Sha256::new()
        .chain_update(pad(0x36))
        .chain_update(body)
        .finalize();
    let outer = Sha256::new()
        .chain_update(pad(0x5c))
        .chain_update(inner)
        .finalize();
    outer.iter().fold("sha256=".to_owned(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// `POST /hooks/github` on `port` as GitHub sends it: the status.
fn hook(port: u16, guid: &str, signature: &str, body: &str) -> u16 {
    let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        c,
        "POST /hooks/github HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nX-GitHub-Event: pull_request\r\n\
         X-GitHub-Delivery: {guid}\r\nX-Hub-Signature-256: {signature}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut out = String::new();
    c.read_to_string(&mut out).unwrap();
    out[9..12].parse().unwrap()
}

/// The threads' ids.
fn threads(cooee: &Cooee) -> Vec<String> {
    let rows = cooee.json("/threads");
    let ids = rows.as_array().unwrap().iter();
    ids.map(|r| r["id"].as_str().unwrap().to_owned()).collect()
}

#[test]
fn a_github_routine_waits_for_face_id_then_a_signed_delivery_starts_its_thread_once() {
    let dir = scratch("routines-github");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let (cooee, port) = hooked(&dir, &shed, "http://127.0.0.1:9");
    let new = json!({ "agent": "fake-acp", "folder": dir.join("root/project"), "text": "hi", "random_id": "start" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // A schedule saves at once.
    let schedule = json!({ "name": "Monday renewals", "when": "0 9 * * 1", "prompt": "Renewals?" });
    assert_eq!(
        call(&shed, &id, "create_routine", &schedule)["isError"],
        false
    );
    assert_eq!(cooee.json("/routines.json").as_array().unwrap().len(), 1);

    // A GitHub one only raises a card; declined, it is never saved.
    let github = |name: &str| {
        json!({ "name": name, "place": "new_thread", "prompt": "Review {{pull_request.title}} ({{event}})",
                "github": { "repo": "owner/app", "events": ["pull_request.opened"] } })
    };
    for name in ["PR review", "Declined"] {
        let said = call(&shed, &id, "create_routine", &github(name));
        assert_eq!(said["isError"], false, "{said}");
        let text = said["content"][0]["text"].as_str().unwrap();
        assert!(text.starts_with("Proposed."), "{text}");
    }
    assert_eq!(cooee.json("/routines.json").as_array().unwrap().len(), 1);
    let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let card = |name: &str| {
        let mut cards = items
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["form"] == "routine");
        cards
            .find(|i| i["tool"].as_str().unwrap().contains(name))
            .unwrap_or_else(|| panic!("no card for {name}: {items}"))["id"]
            .clone()
    };
    let answer = |item: &Value, option: &str, passkey: Value| {
        let a = json!({ "item": item, "option": option, "passkey": passkey });
        cooee.post(&format!("/threads/{id}/approve"), &a)
    };
    let (status, body) = answer(&card("Declined"), "decline", Value::Null);
    assert_eq!(status, 200, "{body}");
    let (status, body) = answer(&card("PR review"), "approve", Value::Null);
    assert_eq!(
        (status, body.as_str()),
        (403, "Approving this needs Face ID.")
    );
    assert_eq!(cooee.json("/routines.json").as_array().unwrap().len(), 1);
    let phone = Authenticator::new(&cooee);
    phone.save(&cooee);
    let (status, body) = answer(&card("PR review"), "approve", phone.get(&cooee));
    assert_eq!(status, 200, "{body}");
    let list = cooee.json("/routines.json");
    let names: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].clone())
        .collect();
    assert_eq!(
        names,
        [json!("Monday renewals"), json!("PR review")],
        "{list}"
    );
    assert_eq!(list[1]["trigger"]["github"]["repo"], "owner/app");

    // A wrong signature: 401 and no thread. A good one: 202 and a thread whose first message has the title.
    let before = threads(&cooee);
    let body = opened("Fix login");
    assert_eq!(hook(port, "g-1", &sign("{}"), &body), 401);
    thread::sleep(Duration::from_millis(500));
    assert_eq!(threads(&cooee), before);
    assert_eq!(hook(port, "g-1", &sign(&body), &body), 202);
    let started = (0..100)
        .find_map(|_| {
            thread::sleep(Duration::from_millis(100));
            let now = threads(&cooee);
            let mut new = now.into_iter().filter(|t| !before.contains(t));
            new.next()
        })
        .expect("no thread started");
    let items = cooee.json(&format!("/threads/{started}/messages"))["items"].clone();
    let first = items
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["kind"] == "you");
    let text = first.unwrap_or_else(|| panic!("{items}"))["text"].clone();
    assert_eq!(text, "Review Fix login (pull_request.opened)");

    // The same delivery again: 200, and still that one thread.
    assert_eq!(hook(port, "g-1", &sign(&body), &body), 200);
    thread::sleep(Duration::from_millis(500));
    assert_eq!(threads(&cooee).len(), before.len() + 1);
    let routine = &cooee.json("/routines.json")[1];
    assert_eq!(routine["last_run"]["thread"], started.as_str(), "{routine}");
}

/// A fake GitHub API on a free port: owner/app's hook for [`URL`] logs a delivery just made (77) and one from long
/// ago (78). Its address, and each path asked for.
fn fake_github(missed: String) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = format!("http://{}", listener.local_addr().unwrap());
    let asked = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&asked);
    thread::spawn(move || {
        for c in listener.incoming() {
            let mut c = c.unwrap();
            let mut head = Vec::new();
            let mut b = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") && c.read(&mut b).unwrap() == 1 {
                head.push(b[0]);
            }
            let head = String::from_utf8_lossy(&head).into_owned();
            let path = head.split(' ').nth(1).unwrap().to_owned();
            assert!(
                head.contains("Authorization: Bearer gh-test-token"),
                "{head}"
            );
            let hooks = "/repos/owner/app/hooks";
            let body = match path.strip_prefix(hooks).unwrap_or("?") {
                "" => {
                    json!([{ "id": 1, "config": { "url": "https://elsewhere" } }, { "id": 9, "config": { "url": URL } }])
                }
                "/9/deliveries?per_page=100" => json!([
                    { "id": 77, "guid": "g-missed", "delivered_at": "2999-01-01T00:00:00Z", "event": "pull_request" },
                    { "id": 78, "guid": "g-old", "delivered_at": "2000-01-01T00:00:00Z", "event": "pull_request" }]),
                "/9/deliveries/77" => json!({ "guid": "g-missed", "event": "pull_request",
                                              "request": { "payload": serde_json::from_str::<Value>(&missed).unwrap() } }),
                _ => json!({ "message": "Not Found" }),
            };
            log.lock().unwrap().push(path);
            let body = body.to_string();
            let _ = write!(
                c,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    (addr, asked)
}

#[test]
fn the_catch_up_runs_a_delivery_the_hook_missed_once_across_restarts() {
    let dir = scratch("routines-catch-up");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let (api, asked) = fake_github(opened("Missed while away"));
    let (cooee, _) = hooked(&dir, &shed, &api);
    let new = json!({ "agent": "fake-acp", "folder": dir.join("root/project"), "text": "hi", "random_id": "start" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    shed.report(&format!("cooee-{id}"), "done", None);
    drop(cooee);
    assert!(
        asked.lock().unwrap().is_empty(),
        "no routine names a repo yet"
    );

    let routine = json!({ "id": "r-gh", "name": "PR opened", "made_in": id, "made_at": ago(0),
        "trigger": { "github": { "repo": "Owner/App", "events": ["pull_request.opened"] } },
        "place": { "in": "chat", "thread": id }, "prompt": "Look at {{pull_request.title}}" });
    let file = dir.join("data/routines.json");
    fs::write(
        &file,
        json!({ "format": 1, "routines": [routine] }).to_string(),
    )
    .unwrap();
    let detail = "/repos/owner/app/hooks/9/deliveries/77";
    let fetched = || {
        asked
            .lock()
            .unwrap()
            .iter()
            .filter(|p| *p == detail)
            .count()
    };
    for start in 1..=2 {
        let cooee = hooked(&dir, &shed, &api).0;
        let lists = |a: &Mutex<Vec<String>>| {
            a.lock()
                .unwrap()
                .iter()
                .filter(|p| p.contains("per_page"))
                .count()
        };
        for _ in 0..100 {
            if lists(&asked) == start && !runs(&cooee, &id).is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        thread::sleep(Duration::from_millis(300));
        let sent = runs(&cooee, &id);
        assert_eq!(sent.len(), 1, "start {start}: {sent:?}");
        assert_eq!(sent[0]["text"], "Look at Missed while away");
        assert_eq!(fetched(), 1, "start {start}: {:?}", asked.lock().unwrap());
    }
    let paths = asked.lock().unwrap().clone();
    assert!(
        !paths.iter().any(|p| p.ends_with("/78")),
        "an old delivery is left: {paths:?}"
    );
    let book: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(book["seen"][0][0], "g-missed", "{book}");
    assert_eq!(book["routines"][0]["last_run"]["outcome"], "ran", "{book}");
}
