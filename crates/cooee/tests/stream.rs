//! The device stream (#718): `GET /stream` and `POST /stream/{sid}`, followed by blocks' own `Mirror` on
//! `/screens/sessions` through dropped and doubled frames and a restart of the server.

mod support;

use std::{fs, path::Path};

use blocks::{Block, Frame, Have, Hello, Mirror, Synced, Took};
use serde_json::{Value, json};
use support::{Cooee, Events, FakeShed, scratch, version};

const KEY: &str = "/screens/sessions";

fn hello(events: &mut Events) -> Hello {
    let e = events.next();
    assert_eq!(e.event, "hello", "{e:?}");
    serde_json::from_value(e.data).unwrap()
}

fn frame(events: &mut Events) -> Frame {
    let e = events.next();
    assert_eq!(e.event, "message", "{e:?}");
    serde_json::from_value(e.data).unwrap()
}

fn sync(cooee: &Cooee, sid: &str, open: &[Have]) -> Synced {
    let (status, body) = cooee.post(&format!("/stream/{sid}"), &json!({ "open": open }));
    assert_eq!(status, 200, "{body}");
    serde_json::from_str(&body).unwrap()
}

fn shown(m: &Mirror) -> Value {
    serde_json::to_value(m.screen(KEY).unwrap()).unwrap()
}

fn lists(screen: &Value, threads: &[&str]) -> bool {
    let text = screen.to_string();
    threads.iter().all(|t| text.contains(&format!("\"t-{t}\"")))
}

fn follow(cooee: &Cooee, events: &mut Events, m: &mut Mirror, threads: &[&str]) {
    loop {
        let screen = shown(m);
        if lists(&screen, threads) && screen == cooee.json(KEY) {
            return;
        }
        let f = frame(events);
        match m.frame(f.clone()) {
            Took::Patched | Took::Replaced => assert_eq!(m.frame(f), Took::Old, "a doubled frame"),
            Took::Gap => panic!("a gap with no frame dropped"),
            Took::Old => {}
        }
    }
}

fn core_only(v: &Value) -> bool {
    match v {
        Value::Object(o) => {
            let app = o.contains_key("type")
                && matches!(serde_json::from_value(v.clone()), Ok(Block::App(_)));
            !app && o.values().all(core_only)
        }
        Value::Array(a) => a.iter().all(core_only),
        _ => true,
    }
}

fn start(dir: &Path, shed: &FakeShed) -> (Cooee, Events, Hello) {
    let cooee = Cooee::start(dir, &shed.dir);
    let mut events = cooee.sse("/stream?v=2", "");
    let hello = hello(&mut events);
    (cooee, events, hello)
}

fn write_coder_profile(dir: &Path) {
    let profiles = dir.join("profiles");
    fs::create_dir_all(&profiles).unwrap();
    let coder = "name = \"coder\"\nlabel = \"Coder\"\nagent = \"fake-acp\"\n";
    fs::write(profiles.join("coder.toml"), coder).unwrap();
}

fn chat(cooee: &Cooee, text: &str) -> String {
    let new = json!({ "agent": "fake-acp", "profile": "coder", "text": text, "random_id": text });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn assert_first_syncs_and_unknown_keys(cooee: &Cooee, m: &mut Mirror, sid: &str) {
    let sent = m.have();
    let synced = sync(cooee, sid, &sent);
    assert!(
        synced.keys[0].screen.is_some(),
        "a first sync gets the whole screen"
    );
    assert_eq!(m.synced(&sent, synced), [Took::Replaced]);
    assert_eq!(shown(m), cooee.json(KEY));

    let sent = m.have();
    let synced = sync(cooee, sid, &sent);
    let same = &synced.keys[0];
    assert_eq!(
        (same.rev, &same.ops, &same.screen),
        (sent[0].rev, &None, &None)
    );
    assert_eq!(m.synced(&sent, synced), [Took::Old]);

    let nope = [Have {
        k: "/screens/nope".into(),
        rev: None,
    }];
    assert_eq!(sync(cooee, sid, &nope).keys[0].gone, Some(404));
    let (status, _) = cooee.post("/stream/no-such-sid", &json!({ "open": sent }));
    assert_eq!(status, 404);
}

#[test]
fn a_mirror_follows_the_sessions_list_through_drops_doubles_and_a_restart() {
    let dir = scratch("stream-sessions");
    write_coder_profile(&dir);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let (cooee, mut events, first) = start(&dir, &shed);
    let mut m = Mirror::new(2);
    m.hello(&first).unwrap();
    m.open(KEY);

    assert_first_syncs_and_unknown_keys(&cooee, &mut m, &first.sid);

    let one = chat(&cooee, "one");
    follow(&cooee, &mut events, &mut m, &[&one]);

    let two = chat(&cooee, "two");
    let dropped = frame(&mut events);
    let three = chat(&cooee, "three");
    let next = frame(&mut events);
    assert!(next.rev > dropped.rev);
    assert_eq!(m.frame(next), Took::Gap, "the frame after a dropped one");
    let sent = m.have();
    let synced = sync(&cooee, &first.sid, &sent);
    assert!(
        synced.keys[0].ops.is_some(),
        "a resync inside the ring gets its ops"
    );
    assert!(synced.keys[0].screen.is_none());
    assert_eq!(m.synced(&sent, synced), [Took::Patched]);
    follow(&cooee, &mut events, &mut m, &[&one, &two, &three]);

    drop(events);
    drop(cooee);
    let (cooee, mut events, again) = start(&dir, &shed);
    assert_ne!(again.epoch, first.epoch, "a restart is a new epoch");
    m.hello(&again).unwrap();
    let sent = m.have();
    assert_eq!(sent[0].rev, None, "a new epoch forgets the rev held");
    let synced = sync(&cooee, &again.sid, &sent);
    assert!(
        synced.keys[0].screen.is_some(),
        "after a restart the screen comes whole"
    );
    assert_eq!(m.synced(&sent, synced), [Took::Replaced]);
    follow(&cooee, &mut events, &mut m, &[&one, &two, &three]);
}

#[test]
fn the_sessions_screen_is_core_kinds_grouped_by_persona_with_the_rows_of_get_threads() {
    let dir = scratch("sessions-screen");
    write_coder_profile(&dir);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let one = chat(&cooee, "one");

    let screen = cooee.json(KEY);
    assert_eq!(
        (&screen["id"], &screen["source"]),
        (&json!("sessions"), &json!("core"))
    );
    let section = &screen["blocks"][0];
    assert_eq!(
        (&section["type"], &section["header"]),
        (&json!("section"), &json!("Coder"))
    );
    let rows = section["blocks"].as_array().unwrap();
    let thread = &cooee.json("/threads")[0];
    assert_eq!(rows[0]["id"], format!("t-{one}"));
    assert_eq!(rows[0]["title"], thread["title"]);
    assert_eq!(rows[0]["tap"]["open"], format!("/screens/chat/{one}"));
    assert_eq!(rows[1]["tap"]["open"], "/screens/chat?persona=coder");
    assert!(core_only(&screen), "{screen}");
    assert_eq!(
        cooee.json("/screens/sessions?archived=1")["blocks"],
        json!([])
    );
}

fn ask_to_pair(cooee: &Cooee) -> String {
    let ask = json!({ "screen": "/screens/pair", "action": "ask" });
    let (head, _) = cooee.raw(
        "POST",
        "/screens/pair/action",
        &[("Content-Type", "application/json")],
        &ask.to_string(),
    );
    let claim = head
        .lines()
        .find_map(|l| l.strip_prefix("set-cookie: cooee_claim="))
        .unwrap();
    format!("cooee_claim={}", claim.split(';').next().unwrap())
}

#[test]
fn a_stream_without_a_device_opens_only_the_pair_screen() {
    const PAIR: &str = "/screens/pair";
    let dir = scratch("stream-pair");
    fs::create_dir_all(dir.join("profiles")).unwrap();
    let coder = "name = \"coder\"\nlabel = \"Coder\"\nagent = \"fake-acp\"\n";
    fs::write(dir.join("profiles/coder.toml"), coder).unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let mut cooee = Cooee::start(&dir, &shed.dir);
    let thread = format!("/screens/chat/{}", chat(&cooee, "one"));
    let paired = std::mem::take(&mut cooee.cookie);
    cooee.cookie = ask_to_pair(&cooee);

    let mut first = cooee.sse("/stream?v=2", "");
    let hello1 = hello(&mut first);
    let mut events = cooee.sse("/stream?v=2", "");
    let e = first.next();
    assert_eq!(
        (e.event.as_str(), &e.data),
        ("bye", &json!({ "why": "evicted" })),
        "a waiting request holds one stream"
    );
    let (status, _) = cooee.post(&format!("/stream/{}", hello1.sid), &json!({ "open": [] }));
    assert_eq!(status, 404, "the evicted stream is gone");
    let hello = hello(&mut events);
    let mut m = Mirror::new(2);
    m.hello(&hello).unwrap();
    m.open(PAIR);
    m.open(KEY);
    m.open(&thread);
    let sent = m.have();
    let synced = sync(&cooee, &hello.sid, &sent);
    let gone: Vec<_> = synced.keys.iter().map(|k| (k.k.as_str(), k.gone)).collect();
    assert_eq!(
        gone,
        [(thread.as_str(), Some(404)), (PAIR, None), (KEY, Some(404))]
    );
    m.synced(&sent, synced);
    let pair = |m: &Mirror| serde_json::to_value(m.screen(PAIR).unwrap()).unwrap();
    assert_eq!(pair(&m)["blocks"][3]["action"], "cancel");

    let claim = std::mem::replace(&mut cooee.cookie, paired);
    let id = cooee.json("/pair/asks")[0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    cooee.cookie = claim;
    cooee.cli(&["pair", "approve", &id]);
    while pair(&m)["blocks"][1]["action"] != "collect" {
        let f = frame(&mut events);
        assert_ne!(m.frame(f), Took::Gap);
    }
    assert_eq!(
        pair(&m),
        cooee.json(PAIR),
        "the stream shows what GET shows"
    );

    let collect = json!({ "screen": PAIR, "action": "collect" });
    assert_eq!(cooee.post("/screens/pair/action", &collect).0, 200);
    let bye = events.until(|e| e.event == "bye");
    assert_eq!(bye.last().unwrap().data, json!({ "why": "handover" }));
    let (status, _) = cooee.post(&format!("/stream/{}", hello.sid), &json!({ "open": [] }));
    assert_eq!(status, 401, "a collected claim reaches no stream");
}
