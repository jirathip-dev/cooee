use std::{fs, path::PathBuf, thread};

use base64::Engine as _;
use blocks::{Frame, Mirror, Synced, Took};
use serde_json::{Value, json};

use crate::{
    setup::{Setup, setup, terminal},
    support::{Cooee, Events, FakeShed, is_item, scratch, version},
};
fn screen_frames(s: &Setup, id: &str) -> (Events, Mirror, String) {
    let key = format!("/screens/chat/{id}");
    let mut events = s.cooee.sse("/stream?v=2", "");
    let hello: blocks::Hello = serde_json::from_value(events.next().data).unwrap();
    let mut m = Mirror::new(2);
    m.hello(&hello).unwrap();
    m.open(&key);
    let sent = m.have();
    let (status, body) = s
        .cooee
        .post(&format!("/stream/{}", hello.sid), &json!({ "open": sent }));
    assert_eq!(status, 200, "{body}");
    let synced: Synced = serde_json::from_str(&body).unwrap();
    assert_eq!(m.synced(&sent, synced), [Took::Replaced]);
    (events, m, key)
}

fn block<'a>(v: &'a Value, id: &str) -> Option<&'a Value> {
    match v {
        Value::Object(o) if o.get("id") == Some(&json!(id)) && o.contains_key("type") => Some(v),
        Value::Object(o) => o.values().find_map(|v| block(v, id)),
        Value::Array(a) => a.iter().find_map(|v| block(v, id)),
        _ => None,
    }
}

fn follow(
    events: &mut Events,
    m: &mut Mirror,
    key: &str,
    f: impl Fn(&Value) -> bool,
) -> Vec<Value> {
    let mut frames = vec![];
    loop {
        let shown = serde_json::to_value(m.screen(key).unwrap()).unwrap();
        if f(&shown) {
            return frames;
        }
        let e = events.next();
        assert_eq!(e.event, "message", "{e:?}");
        let frame: Frame = serde_json::from_value(e.data.clone()).unwrap();
        assert_ne!(m.frame(frame), Took::Gap);
        frames.push(e.data);
    }
}

fn ops<'a>(frames: &'a [Value], op: &'a str) -> impl Iterator<Item = &'a Value> {
    frames
        .iter()
        .flat_map(|f| f["ops"].as_array().unwrap())
        .filter(move |o| o["op"] == op)
}

fn text(prose: &Value) -> String {
    prose["parts"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|p| p["spans"].as_array().unwrap())
        .map(|s| s["text"].as_str().unwrap())
        .collect()
}

#[test]
fn a_reply_streams_into_one_message_by_appends_beside_its_tool_row_and_keeps_its_id_when_stored() {
    let s = setup("chat-screen");
    let id = s.create("hang");
    let (mut events, mut m, key) = screen_frames(&s, &id);
    let streaming = |v: &Value| block(v, "m2-text").is_some_and(|p| p["streaming"] == true);
    follow(&mut events, &mut m, &key, |v| {
        streaming(v) && block(v, "i2").is_some_and(|t| t["state"] == "running")
    });
    let shown = serde_json::to_value(m.screen(&key).unwrap()).unwrap();
    let reply = block(&shown, "m2").unwrap();
    assert_eq!(
        (&reply["type"], &reply["role"]),
        (&json!("message"), &json!("agent"))
    );
    let inside: Vec<&Value> = reply["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| &b["id"])
        .collect();
    assert_eq!(
        inside,
        ["i2", "m2-text"],
        "the tool row, then the text last"
    );
    assert_eq!(block(&shown, "i2").unwrap()["summary"], "cargo test");
    assert_eq!(text(block(&shown, "m2-text").unwrap()), "w0");

    s.shed.says(&terminal(&id), "w1 ");
    let grew = follow(&mut events, &mut m, &key, |v| {
        block(v, "m2-text").is_some_and(|p| text(p) == "w0 w1")
    });
    let added: Vec<&Value> = ops(&grew, "append").map(|o| &o["id"]).collect();
    assert_eq!(added, ["m2-text"], "{grew:?}");

    s.shed.replies(&terminal(&id), "w0 w1 ");
    let stored = follow(&mut events, &mut m, &key, |v| {
        !streaming(v)
            && block(v, "m2-text").is_some()
            && block(v, "say").unwrap()["working"].is_null()
    });
    assert_eq!(
        ops(&stored, "insert").count(),
        0,
        "the stored reply is the block that streamed: {stored:?}"
    );
    let shown = serde_json::to_value(m.screen(&key).unwrap()).unwrap();
    assert_eq!(shown, s.cooee.json(&key));

    let png = base64::engine::general_purpose::STANDARD.encode(b"not really a png");
    let files = json!([{ "name": "shot.png", "data": png }]);
    let body = json!({ "text": "echo seen", "random_id": "f1", "files": files });
    let (status, sent) = s.cooee.post(&format!("/threads/{id}/send"), &body);
    assert_eq!(status, 200, "{sent}");
    let you: Value = serde_json::from_str(&sent).unwrap();
    let mine = format!("m{}", you["id"]);
    follow(&mut events, &mut m, &key, |v| block(v, &mine).is_some());
    let shown = serde_json::to_value(m.screen(&key).unwrap()).unwrap();
    let message = block(&shown, &mine).unwrap();
    assert_eq!(
        (&message["role"], &message["name"]),
        (&json!("owner"), &json!("You"))
    );
    let file = &message["files"][0];
    let url = format!("/threads/{id}/files/{}", you["files"][0].as_str().unwrap());
    assert_eq!(
        (&file["tap"]["url"], &file["src"]),
        (&json!(url), &json!(url))
    );
    assert_eq!(s.cooee.get(&url).0, 200, "the attachment opens");
    assert_eq!(s.cooee.get("/screens/chat/nope").0, 404);
}

fn eventually(mut f: impl FnMut() -> bool) {
    for _ in 0..200 {
        if f() {
            return;
        }
        thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("never happened");
}

fn with_pushes(name: &str) -> (Setup, PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch(name);
    let (bin, log) = (dir.join("bin"), dir.join("pushed"));
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&log).unwrap();
    let curl = format!(
        "#!/bin/sh\nperl -ne 'print $1 =~ s/\\\\(.)/$1/gr if /^data-raw = \"(.*)\"$/' >\"{}/$$.json\"\nprintf '\\n200'\n",
        log.display()
    );
    fs::write(bin.join("curl"), curl).unwrap();
    fs::set_permissions(bin.join("curl"), fs::Permissions::from_mode(0o755)).unwrap();
    let key = "MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg/FMQqoOmZU6bop6k\n\
               jwzlkZBiOA5CCW7ngc03CgGpgsKhRANCAASkJsDwiTE2wL6OIdpH5tfnsdyw5UK8\n\
               NOnlJZkFHy9kIEFLJ8uttasmC9v/MsS+Cbb3MJBa25UlVnmKe8xoXskq";
    let p = "PRIVATE KEY-----";
    fs::create_dir_all(dir.join("data")).unwrap();
    let pem = format!("-----BEGIN {p}\n{key}\n-----END {p}\n");
    fs::write(dir.join("data/apns.p8"), pem).unwrap();
    let apns = "[apns]\nkey_id = \"K\"\nteam_id = \"T\"\ntopic = \"dev.cooee.app\"\nenv = \"sandbox\"\nurl = \"https://apns.test\"\n";
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start_env(&dir, &shed.dir, apns, &[("PATH", path.as_str())]);
    let token = json!({ "token": "ab".repeat(32) });
    assert_eq!(cooee.post("/push/apns", &token).0, 200);
    (Setup { dir, shed, cooee }, log)
}

fn pushed(log: &std::path::Path) -> Vec<String> {
    let files = fs::read_dir(log).unwrap();
    let read = files.map(|f| fs::read_to_string(f.unwrap().path()).unwrap());
    let all = read.filter_map(|t| serde_json::from_str::<Value>(&t).ok());
    let mut titles: Vec<String> = all
        .map(|p| p["aps"]["alert"]["title"].as_str().unwrap().to_owned())
        .collect();
    titles.sort();
    titles
}

fn unseen(s: &Setup, id: &str) -> Value {
    let list = s.cooee.json("/threads");
    let row = list.as_array().unwrap().iter().find(|r| r["id"] == id);
    row.unwrap()["unseen"].clone()
}

#[test]
fn a_held_chat_sends_no_push_and_is_marked_read_as_its_frames_go_out() {
    let (s, log) = with_pushes("chat-held");
    let held = s.create("Say hello");
    eventually(|| pushed(&log) == ["Say hello"]);
    assert_eq!(unseen(&s, &held), 1);

    let (mut events, mut m, key) = screen_frames(&s, &held);
    eventually(|| unseen(&s, &held) == 0);
    assert_eq!(s.send(&held, "again", "r2").0, 200);
    follow(&mut events, &mut m, &key, |v| {
        block(v, "m4-text").is_some_and(|p| p["streaming"].is_null())
    });
    eventually(|| unseen(&s, &held) == 0);

    let other = s.create("Hi there");
    eventually(|| pushed(&log).len() >= 2);
    assert_eq!(
        pushed(&log),
        ["Hi there", "Say hello"],
        "no push for the held chat's reply"
    );
    assert_eq!(unseen(&s, &other), 1);
}

fn act(s: &Setup, screen: &str, action: &str, values: &Value) -> (u16, Value) {
    let a = json!({ "screen": screen, "block": "say", "action": action, "values": values });
    let (status, body) = s.cooee.post("/screens/action", &a);
    (status, serde_json::from_str(&body).unwrap())
}

fn mine(s: &Setup, id: &str, text: &str) -> Vec<Value> {
    let items = s.items(id).into_iter();
    items
        .filter(|i| i["kind"] == "you" && i["text"] == text)
        .collect()
}

fn prompted(s: &Setup, id: &str, text: &str) -> usize {
    let sent = s.shed.sent(&terminal(id));
    sent.iter().filter(|t| t.contains(text)).count()
}

#[test]
fn a_send_with_the_same_nonce_twice_is_one_item_and_one_prompt_through_either_door() {
    let s = setup("chat-act-send");
    let id = s.create("hi");
    s.events(&id).until(|e| is_item(e, "agent"));
    let key = format!("/screens/chat/{id}");
    let send = json!({ "text": "echo once", "nonce": "n1" });
    assert_eq!(act(&s, &key, "send", &send), (200, json!({})));
    assert_eq!(act(&s, &key, "send", &send), (200, json!({})));
    let (status, body) = s.send(&id, "echo once", "n1");
    assert_eq!(status, 200, "{body}");
    let stored = mine(&s, &id, "echo once");
    assert_eq!(stored.len(), 1, "{stored:?}");
    assert_eq!(stored[0]["random_id"], "n1");
    assert_eq!(prompted(&s, &id, "echo once"), 1);

    let (status, refused) = act(&s, &key, "send", &json!({ "text": "echo none" }));
    assert_eq!((status, &refused["toast"]["tone"]), (400, &json!("bad")));
    assert_eq!(mine(&s, &id, "echo none"), Vec::<Value>::new());
    assert_eq!(prompted(&s, &id, "echo none"), 0);
}

#[test]
fn a_file_uploads_first_and_the_send_names_its_id() {
    let s = setup("chat-act-file");
    let id = s.create("hi");
    s.events(&id).until(|e| is_item(e, "agent"));
    let upload = |name: &str| {
        let headers = [("Cookie", s.cooee.cookie.as_str()), ("X-Name", name)];
        let (head, body) = s
            .cooee
            .raw("POST", "/screens/files", &headers, "not really a png");
        (head[9..12].to_owned(), body)
    };
    assert_eq!(upload("../escape").0, "400");
    let (status, kept) = upload("shot.png");
    assert_eq!(status, "200", "{kept}");
    let kept: Value = serde_json::from_str(&kept).unwrap();
    assert_eq!(kept["name"], "shot.png");

    let key = format!("/screens/chat/{id}");
    let files = json!([{ "id": kept["id"], "name": "shot.png", "src": null }]);
    let send = json!({ "text": "echo seen", "nonce": "f1", "files": files });
    assert_eq!(act(&s, &key, "send", &send).0, 200);
    assert_eq!(
        act(&s, &key, "send", &send).0,
        200,
        "a retry still finds its file"
    );
    let stored = mine(&s, &id, "echo seen");
    assert_eq!(stored.len(), 1, "{stored:?}");
    let name = stored[0]["files"][0].as_str().unwrap();
    let (status, bytes) = s.cooee.get(&format!("/threads/{id}/files/{name}"));
    assert_eq!((status, bytes.as_str()), (200, "not really a png"));

    let gone = json!({ "text": "echo lost", "nonce": "f2", "files": [{ "id": "1-ab" }] });
    assert_eq!(act(&s, &key, "send", &gone).0, 400);
    assert_eq!(mine(&s, &id, "echo lost"), Vec::<Value>::new());
}

fn turn(s: &Setup, id: &str) -> Value {
    s.cooee.json(&format!("/threads/{id}/messages"))["turn"]["state"].clone()
}

fn queued(s: &Setup, key: &str) -> Vec<Value> {
    let screen = s.cooee.json(key);
    let say = block(&screen, "say").unwrap();
    say["queued"].as_array().cloned().unwrap_or_default()
}

#[test]
fn a_card_queue_steer_and_stop_work_from_the_chat_screen() {
    let s = setup("chat-act-turn");
    let id = s.create("please approve an edit");
    let key = format!("/screens/chat/{id}");
    let mut events = s.events(&id);
    let ask = events.until(|e| is_item(e, "approval")).pop().unwrap();
    let card = json!({ "screen": key, "block": format!("{}-answer", ask.data["id"]), "action": "answer", "values": { "choice": "allow" } });
    for _ in 0..2 {
        assert_eq!(s.cooee.post("/screens/action", &card).0, 200);
    }
    let approvals = s.shed.sent(&terminal(&id));
    let approvals: Vec<&String> = approvals
        .iter()
        .filter(|t| t.contains("approve\":"))
        .collect();
    assert_eq!(approvals, ["\x1e{\"approve\":\"allow\"}\n"]);
    events.until(|e| is_item(e, "agent"));

    let hang = json!({ "text": "hang on", "nonce": "h1" });
    assert_eq!(act(&s, &key, "send", &hang).0, 200);
    eventually(|| turn(&s, &id) == "working");
    let later = json!({ "text": "then this", "nonce": "q1" });
    assert_eq!(act(&s, &key, "queue", &later).0, 200);
    let shown = queued(&s, &key);
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0]["text"], "then this");
    assert_eq!(
        act(&s, &key, "unqueue", &json!({ "q": shown[0]["q"] })).0,
        200
    );
    assert_eq!(queued(&s, &key), Vec::<Value>::new());

    assert_eq!(
        act(
            &s,
            &key,
            "queue",
            &json!({ "text": "steer", "nonce": "q2" })
        )
        .0,
        200
    );
    let q = queued(&s, &key)[0]["q"].clone();
    assert_eq!(act(&s, &key, "steer", &json!({ "q": q })).0, 200);
    assert_eq!(queued(&s, &key), Vec::<Value>::new());
    assert!(
        s.shed
            .sent(&terminal(&id))
            .iter()
            .any(|t| t.contains("\"steer\""))
    );

    assert_eq!(act(&s, &key, "stop", &json!({})).0, 200);
    assert!(
        s.shed
            .sent(&terminal(&id))
            .contains(&"\x1e\"cancel\"\n".to_owned())
    );
    eventually(|| turn(&s, &id) == "done");
    assert_eq!(
        mine(&s, &id, "then this"),
        Vec::<Value>::new(),
        "the unqueued message never went"
    );
}

#[test]
fn older_widens_a_held_chat_by_50_items_for_its_holders_to_500() {
    let dir = scratch("chat-act-older");
    let threads = dir.join("data/threads");
    fs::create_dir_all(&threads).unwrap();
    let meta = json!({ "format": 1, "thread": { "id": "long", "title": "long", "messages": [], "terminal": "cooee-long" } });
    fs::write(threads.join("long.json"), meta.to_string()).unwrap();
    let items: String = (1..=600)
        .map(|k| json!({ "id": k, "at": k, "kind": "you", "text": format!("n{k}"), "random_id": format!("r{k}") }).to_string() + "\n")
        .collect();
    fs::write(threads.join("long.jsonl"), items).unwrap();
    drop(cooee_core::Store::open(&dir.join("data")).unwrap());
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let s = Setup { dir, shed, cooee };

    let (mut events, mut m, key) = screen_frames(&s, "long");
    let count = |v: &Value| block(v, "log").unwrap()["blocks"].as_array().unwrap().len();
    let shown = serde_json::to_value(m.screen(&key).unwrap()).unwrap();
    assert_eq!(count(&shown), 50);
    for n in 2..=10 {
        assert!(
            block(
                &serde_json::to_value(m.screen(&key).unwrap()).unwrap(),
                "older"
            )
            .is_some()
        );
        assert_eq!(act(&s, &key, "older", &json!({})), (200, json!({})));
        follow(&mut events, &mut m, &key, |v| count(v) == 50 * n);
    }
    let shown = serde_json::to_value(m.screen(&key).unwrap()).unwrap();
    assert!(block(&shown, "older").is_none(), "500 is the widest");
    assert!(block(&shown, "m101").is_some() && block(&shown, "m100").is_none());
    assert_eq!(act(&s, &key, "older", &json!({})).0, 200);
    assert_eq!(count(&s.cooee.json(&key)), 50, "GET stays the newest 50");
}
