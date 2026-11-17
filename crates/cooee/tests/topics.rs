//! A persona's topics (#507): `GET /threads?persona=` rows with title, folder, unread, working state and archived
//! flag, newest first; Home's topic and unread counts; the auto title cut from the first message; the brief's topic.

mod support;

use std::{fs, path::Path, thread, time::Duration};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

fn eventually(mut f: impl FnMut() -> bool) {
    for _ in 0..200 {
        if f() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("never happened");
}

fn setup(name: &str) -> (std::path::PathBuf, FakeShed, Cooee) {
    let dir = scratch(name);
    let profiles = dir.join("profiles");
    fs::create_dir_all(&profiles).unwrap();
    fs::write(
        profiles.join("coder.toml"),
        "name = \"coder\"\nlabel = \"Coder\"\nagent = \"fake-acp\"\n",
    )
    .unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    (dir, shed, cooee)
}

fn create(cooee: &Cooee, folder: &Path, profile: &str, text: &str) -> String {
    let new = json!({ "agent": "fake-acp", "profile": profile, "folder": folder, "text": text, "random_id": text });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn topics(cooee: &Cooee) -> Vec<Value> {
    cooee
        .json("/threads?persona=coder")
        .as_array()
        .unwrap()
        .clone()
}

fn topic(cooee: &Cooee, id: &str) -> Value {
    let rows = topics(cooee);
    rows.into_iter().find(|r| r["id"] == id).expect("listed")
}

#[test]
fn a_personas_topics_carry_their_fields_and_home_counts_the_open_ones() {
    let (dir, _shed, cooee) = setup("topics-list");
    let folder = dir.join("root/project");
    let plain = create(&cooee, &folder, "", "Say hello from a plain chat");
    let read = create(&cooee, &folder, "coder", "Say hello, then read");
    eventually(|| topic(&cooee, &read)["unseen"] == 1);
    let last = cooee.json(&format!("/threads/{read}/messages"))["items"][1]["id"].clone();
    let mark = format!("/threads/{read}/read");
    assert_eq!(cooee.post(&mark, &json!({ "item": last })).0, 200);
    let archived = create(&cooee, &folder, "coder", "Say hello, then archive");
    eventually(|| topic(&cooee, &archived)["unseen"] == 1);
    let archive = format!("/threads/{archived}/archive");
    assert_eq!(cooee.post(&archive, &json!({ "on": true })).0, 200);
    let unread = create(&cooee, &folder, "coder", "Say hello and stay unread");
    eventually(|| topic(&cooee, &unread)["unseen"] == 1);
    let working = create(&cooee, &folder, "coder", "hang on");
    let said = |id: &str| {
        let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
        items
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["kind"] == "tool")
    };
    eventually(|| topic(&cooee, &working)["state"] == "working" && said(&working));

    let rows = topics(&cooee);
    let ids: Vec<&str> = rows.iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert!(!ids.contains(&plain.as_str()), "{ids:?}");
    assert_eq!(ids.len(), 4, "{ids:?}");
    let at: Vec<u64> = rows.iter().map(|r| r["at"].as_u64().unwrap()).collect();
    assert!(at.windows(2).all(|w| w[0] >= w[1]), "{at:?}");
    let field = |id: &str| {
        let r = topic(&cooee, id);
        (
            r["title"].clone(),
            r["folder"].clone(),
            r["unseen"].clone(),
            r["state"].clone(),
            r["archived"].clone(),
        )
    };
    let f = json!(fs::canonicalize(&folder).unwrap());
    assert_eq!(
        field(&read),
        (
            json!("Say hello, then read"),
            f.clone(),
            json!(0),
            json!("done"),
            json!(false)
        )
    );
    assert_eq!(
        field(&archived),
        (
            json!("Say hello, then archive"),
            f.clone(),
            json!(1),
            json!("done"),
            json!(true)
        )
    );
    assert_eq!(
        field(&unread),
        (
            json!("Say hello and stay unread"),
            f.clone(),
            json!(1),
            json!("done"),
            json!(false)
        )
    );
    assert_eq!(
        field(&working),
        (
            json!("hang on"),
            f,
            json!(1),
            json!("working"),
            json!(false)
        )
    );

    let home = cooee.json("/screens/home");
    let coder = &home["personas"][0];
    assert_eq!(coder["id"], "coder");
    assert_eq!(
        (coder["topics"].clone(), coder["unread"].clone()),
        (json!(3), json!(2))
    );
}

#[test]
fn a_topic_is_titled_by_its_first_words_and_a_set_title_stays() {
    let (dir, shed, cooee) = setup("topics-title");
    let folder = dir.join("root/project");
    let id = create(
        &cooee,
        &folder,
        "coder",
        "Say hello to everyone in the whole team\nand more",
    );
    assert_eq!(topic(&cooee, &id)["title"], "Say hello to everyone in the");
    let terminal = format!("cooee-{id}");
    eventually(|| topic(&cooee, &id)["state"] == "done");
    let first = shed.sent(&terminal)[0].clone();
    let line = format!(
        "This topic: Say hello to everyone in the (folder {}).",
        fs::canonicalize(&folder).unwrap().display()
    );
    assert!(first.contains(&line), "{first}");

    let path = format!("/threads/{id}/title");
    assert_eq!(cooee.post(&path, &json!({ "title": "Greetings" })).0, 200);
    let again = json!({ "text": "Say hello again", "random_id": "again" });
    let (status, body) = cooee.post(&format!("/threads/{id}/send"), &again);
    assert_eq!(status, 200, "{body}");
    eventually(|| shed.sent(&terminal).len() == 2);
    assert_eq!(topic(&cooee, &id)["title"], "Greetings");
    let sent = shed.sent(&terminal);
    assert_eq!(sent[0], first);
    assert!(!sent[1].contains("This topic"), "{}", sent[1]);
}
