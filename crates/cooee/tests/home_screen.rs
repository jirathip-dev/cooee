//! Core's home as blocks (#311): `GET /screens/home` is the app's bundled home with `source: "core"` set by the
//! server and the same threads and personas `GET /threads` and `GET /personas` answer.

mod support;

use std::{
    fs,
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, is_item, scratch, version};

const BUNDLED: &str = include_str!("../../../apple/Sources/CooeeUI/Default/screens/home.json");

#[test]
fn home_is_the_bundled_screen_filled_with_todays_threads_and_personas() {
    let dir = scratch("home-screen");
    let profiles = dir.join("profiles");
    fs::create_dir_all(&profiles).unwrap();
    fs::write(
        profiles.join("coder.toml"),
        "name = \"coder\"\nlabel = \"Coder\"\nagent = \"fake-acp\"\n",
    )
    .unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let new = json!({ "agent": "fake-acp", "profile": "coder", "text": "hi", "random_id": "r1" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");

    let home = cooee.json("/screens/home");
    let bundled: Value = serde_json::from_str(BUNDLED).unwrap();
    assert_eq!(home["source"], "core");
    assert!(
        bundled.get("source").is_none(),
        "the server sets the source, not the file"
    );
    for key in ["type", "id", "title", "blocks"] {
        assert_eq!(home[key], bundled[key], "{key}");
    }
    let mut personas = cooee.json("/personas");
    personas[0]["topics"] = json!(1);
    personas[0]["unread"] = home["personas"][0]["unread"].clone();
    assert_eq!(home["personas"], personas);
    let ids = |rows: &Value| -> Vec<Value> {
        rows.as_array()
            .unwrap()
            .iter()
            .map(|r| r["id"].clone())
            .collect()
    };
    let threads = cooee.json("/threads");
    assert_eq!(ids(&home["threads"]), ids(&threads));
    assert_eq!(ids(&threads).len(), 1);
    assert_eq!(home["personas"][0]["id"], "coder");
}

#[test]
fn an_approval_in_a_background_thread_reaches_a_connected_home_within_a_second() {
    let dir = scratch("home-events");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let mut home = cooee.sse("/home/events", "");
    assert_eq!(home.next().event, "changed", "a connect says changed once");
    let folder = dir.join("root/project");
    let new = json!({ "agent": "fake-acp", "folder": folder, "text": "please approve an edit", "random_id": "r1" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"].clone();
    let id = id.as_str().unwrap();
    let cursor = cooee.json(&format!("/threads/{id}/messages"))["cursor"].clone();
    let boot = cursor.as_str().unwrap().split('.').next().unwrap();
    let mut thread = cooee.events(id, &format!("{boot}.0"), false);
    thread.until(|e| is_item(e, "approval"));
    let asked = Instant::now();
    let waiting = |cooee: &Cooee| cooee.json("/threads")[0]["last"] == "Edit src/lib.rs";
    while !waiting(&cooee) {
        assert_eq!(home.next().event, "changed");
    }
    assert!(
        asked.elapsed() < Duration::from_secs(1),
        "{:?}",
        asked.elapsed()
    );
}
