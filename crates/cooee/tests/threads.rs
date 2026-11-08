//! Threads and messages (#43, herdsman#553 rows 15–29) end to end against the fake shed: the list's unread count and
//! archive, renaming, files sent with a message (stored, served safely, their paths given to the agent), and the
//! queue: held during a turn, sent as it ends, steered in or taken back.

mod support;

use std::thread;
use std::time::Duration;

use serde_json::{Value, json};
use support::{Cooee, FakeShed, is_item, scratch, version};

fn setup(name: &str) -> (std::path::PathBuf, FakeShed, Cooee) {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    (dir, shed, cooee)
}

fn create(cooee: &Cooee, dir: &std::path::Path, text: &str) -> String {
    let new = json!({ "agent": "fake-acp", "folder": dir.join("root/project"), "text": text, "random_id": format!("start-{text}") });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// Standard base64, as a browser's `readAsDataURL` gives it.
fn b64(bytes: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..4 {
            out.push(if i <= c.len() {
                A[(n >> (18 - 6 * i) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    out
}

/// Polls `f` for up to 10 s.
fn eventually(mut f: impl FnMut() -> bool) {
    for _ in 0..200 {
        if f() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("never happened");
}

fn row(cooee: &Cooee, id: &str) -> (Value, bool) {
    let list = cooee.json("/threads");
    let r = list
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == id)
        .cloned()
        .expect("listed");
    let archived = r["archived"] == json!(true);
    (r, archived)
}

#[test]
fn the_list_counts_unseen_replies_and_archives_apart() {
    let (dir, _shed, cooee) = setup("t-list");
    let id = create(&cooee, &dir, "Say hello");
    eventually(|| row(&cooee, &id).0["unseen"] == 1);
    let (r, archived) = row(&cooee, &id);
    assert_eq!((r["unread"].clone(), archived), (json!(true), false));
    let last = cooee.json(&format!("/threads/{id}/messages"))["items"][1]["id"].clone();
    assert_eq!(
        cooee
            .post(&format!("/threads/{id}/read"), &json!({ "item": last }))
            .0,
        200
    );
    let (r, _) = row(&cooee, &id);
    assert_eq!(
        (r["unread"].clone(), r["unseen"].clone()),
        (json!(false), json!(0))
    );

    let archive = format!("/threads/{id}/archive");
    assert_eq!(cooee.post(&archive, &json!({ "on": true })).0, 200);
    assert!(row(&cooee, &id).1);
    let (_, page) = cooee.get("/");
    assert!(page.contains(&format!("data-restore=\"{id}\"")), "{page}");
    assert_eq!(cooee.post(&archive, &json!({ "on": false })).0, 200);
    assert!(!row(&cooee, &id).1);
}

#[test]
fn a_thread_is_renamed_and_keeps_its_name_over_a_restart() {
    let (dir, shed, cooee) = setup("t-title");
    let id = create(&cooee, &dir, "hi there\nmore");
    assert_eq!(row(&cooee, &id).0["title"], "hi there");
    let path = format!("/threads/{id}/title");
    assert_eq!(cooee.post(&path, &json!({ "title": "  " })).0, 400);
    assert_eq!(cooee.post(&path, &json!({ "title": " the build " })).0, 200);
    drop(cooee);
    let cooee = Cooee::start(&dir, &shed.dir);
    assert_eq!(row(&cooee, &id).0["title"], "the build");
}

#[test]
fn files_are_stored_served_as_images_only_and_named_to_the_agent() {
    let (dir, shed, cooee) = setup("t-files");
    let id = create(&cooee, &dir, "Say hello");
    // Its bytes as text: the test client reads bodies as UTF-8.
    let png = b"not really a png".to_vec();
    let files = json!([
        { "name": "../../photo one.png", "data": b64(&png) },
        { "name": "page.html", "data": b64(b"<script>alert(1)</script>") },
    ]);
    let body = json!({ "text": "two files", "random_id": "f1", "files": files });
    let (status, item) = cooee.post(&format!("/threads/{id}/send"), &body);
    assert_eq!(status, 200, "{item}");
    let item: Value = serde_json::from_str(&item).unwrap();
    let names: Vec<&str> = item["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 2);
    assert!(
        names[0].ends_with("-0-photo_one.png") && names[1].ends_with("-1-page.html"),
        "{names:?}"
    );
    assert_eq!(item["text"], "two files");

    let (head, _) = cooee.raw(
        "GET",
        &format!("/threads/{id}/files/{}", names[0]),
        &[("Cookie", &cooee.cookie)],
        "",
    );
    assert!(
        head.contains("content-type: image/png")
            && head.contains("content-security-policy: sandbox"),
        "{head}"
    );
    let (head, _) = cooee.raw(
        "GET",
        &format!("/threads/{id}/files/{}", names[1]),
        &[("Cookie", &cooee.cookie)],
        "",
    );
    assert!(
        head.contains("application/octet-stream")
            && head.contains("content-disposition: attachment"),
        "{head}"
    );
    assert_eq!(
        cooee.get(&format!("/threads/{id}/files/..%2F{id}.json")).0,
        404
    );
    assert_eq!(cooee.get(&format!("/threads/{id}/files/nope.png")).0, 404);

    let prompt = shed.sent(&format!("cooee-{id}")).pop().unwrap();
    let stored = dir.join("data/files").join(&id).join(names[0]);
    assert_eq!(std::fs::read(&stored).unwrap(), png);
    assert!(
        prompt.contains("two files\\n\\nAttached files:\\n- ")
            && prompt.contains(&stored.display().to_string()),
        "{prompt}"
    );
}

#[test]
fn a_message_queued_during_a_turn_goes_as_it_ends_and_one_steered_goes_at_once() {
    let (dir, shed, cooee) = setup("t-queue");
    let id = create(&cooee, &dir, "Say hello");
    let terminal = format!("cooee-{id}");
    eventually(|| row(&cooee, &id).0["state"] == "done");
    // Between turns a queued message goes at once.
    let q = |text: &str, rid: &str| {
        let (status, body) = cooee.post(
            &format!("/threads/{id}/queue"),
            &json!({ "text": text, "random_id": rid }),
        );
        assert_eq!(status, 200, "{body}");
        serde_json::from_str::<Value>(&body).unwrap()
    };
    assert_eq!(q("hang on", "q0"), json!([]));
    eventually(|| row(&cooee, &id).0["state"] == "working");
    let cursor = cooee.json(&format!("/threads/{id}/messages"))["cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut events = cooee.events(&id, &cursor, false);

    assert_eq!(q("then this", "q1")[0]["text"], "then this");
    assert_eq!(q("steer", "q2").as_array().unwrap().len(), 2);
    // Again with the same id: kept once.
    assert_eq!(q("steer", "q2").as_array().unwrap().len(), 2);
    let queued = events.until(|e| e.event == "queue");
    assert_eq!(queued.last().unwrap().data[0]["random_id"], "q1");
    assert!(!shed.sent(&terminal).iter().any(|t| t.contains("then this")));

    let (status, left) = cooee.post(
        &format!("/threads/{id}/steer"),
        &json!({ "random_id": "q2" }),
    );
    assert_eq!(status, 200, "{left}");
    assert_eq!(
        serde_json::from_str::<Value>(&left)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(shed.sent(&terminal).iter().any(|t| t.contains("\"steer\"")));
    assert_eq!(
        cooee
            .post(
                &format!("/threads/{id}/unqueue"),
                &json!({ "random_id": "q2" })
            )
            .0,
        400
    );
    assert_eq!(
        cooee.json(&format!("/threads/{id}/messages"))["queue"][0]["text"],
        "then this"
    );

    // The turn ends: the queued message goes, as the owner's next row.
    assert_eq!(
        cooee.post(&format!("/threads/{id}/cancel"), &json!({})).0,
        200
    );
    let sent = events.until(|e| is_item(e, "you") && e.data["random_id"] == "q1");
    assert_eq!(sent.last().unwrap().data["text"], "then this");
    events.until(|e| is_item(e, "agent") && e.data["text"] == "Hello");
    assert_eq!(
        cooee.json(&format!("/threads/{id}/messages"))["queue"],
        json!([])
    );

    // Taken back: never sent.
    eventually(|| row(&cooee, &id).0["state"] == "done");
    q("hang on", "q3");
    eventually(|| row(&cooee, &id).0["state"] == "working");
    q("not this", "q4");
    assert_eq!(
        cooee
            .post(
                &format!("/threads/{id}/unqueue"),
                &json!({ "random_id": "q4" })
            )
            .1,
        "[]"
    );
    cooee.post(&format!("/threads/{id}/cancel"), &json!({}));
    eventually(|| row(&cooee, &id).0["state"] == "done");
    assert!(!shed.sent(&terminal).iter().any(|t| t.contains("not this")));
}
