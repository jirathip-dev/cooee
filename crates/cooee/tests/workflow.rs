//! Workflow (#33) end to end: ACP sessions another tool started in the fake shed, known only by their labels, on
//! the one thread list under the All · Chats · Workflow chips; each opens as a normal chat (streamed reply, a reply
//! and [stop] that reach its terminal); finished ones fold under "done today" and go once the tool removes them.

mod support;

use std::{thread, time::Duration};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, is_item, scratch, version};

/// A lane's labels, as a tool sets them (`role` and `round` vary).
fn labels<'a>(item: &'a str, role: &'a str, round: &'a str) -> [(&'a str, &'a str); 6] {
    [
        ("source", "drover"),
        ("group", "cooee"),
        ("item", item),
        ("role", role),
        ("round", round),
        ("url", "https://github.com/o/cooee/pull/60"),
    ]
}

/// The workflow cards' rows and folded rows of `GET /threads`, waiting until there are `n` sessions.
fn workflow(cooee: &Cooee, n: usize) -> (Vec<Value>, Vec<Value>) {
    for _ in 0..250 {
        let cards = cooee.json("/threads");
        let (mut rows, mut done) = (Vec::new(), Vec::new());
        for c in cards
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["done"].is_array())
        {
            rows.extend(c["rows"].as_array().unwrap().iter().cloned());
            done.extend(c["done"].as_array().unwrap().iter().cloned());
        }
        if rows.len() + done.len() == n {
            return (rows, done);
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("no {n} workflow sessions: {}", cooee.json("/threads"));
}

fn id_of(row: &Value) -> String {
    row["id"].as_str().unwrap().to_owned()
}

/// Taps chip `list` as the page's form does; the cookie it sets goes with every later request.
fn chip(cooee: &mut Cooee, list: &str) {
    let pairing = cooee.cookie.split(';').next().unwrap().to_owned();
    let form = [
        ("Content-Type", "application/x-www-form-urlencoded"),
        ("Cookie", pairing.as_str()),
    ];
    let (head, _) = cooee.raw("POST", "/list", &form, &format!("list={list}"));
    assert!(
        head.starts_with("HTTP/1.0 303") || head.starts_with("HTTP/1.1 303"),
        "{head}"
    );
    let set = head
        .lines()
        .find_map(|l| l.strip_prefix("set-cookie: "))
        .unwrap();
    let kept = set.split(';').next().unwrap();
    cooee.cookie = format!("{pairing}; {kept}");
}

#[test]
fn a_lane_is_listed_with_its_labels_and_the_chips_filter_the_one_list() {
    let dir = scratch("workflow-chips");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let mut cooee = Cooee::start(&dir, &shed.dir);
    let folder = dir.join("root/project");
    let new =
        json!({ "agent": "fake-acp", "folder": folder, "text": "My own chat", "random_id": "s1" });
    assert_eq!(cooee.post("/threads", &new).0, 200);
    shed.lane("drover-cooee-33-impl", &labels("#33", "impl", "2"));

    let (rows, _) = workflow(&cooee, 1);
    let w = &rows[0]["workflow"];
    assert_eq!(
        (
            &rows[0]["title"],
            &w["group"],
            &w["item"],
            &w["role"],
            &w["round"]
        ),
        (
            &json!("cooee #33"),
            &json!("cooee"),
            &json!("#33"),
            &json!("impl"),
            &json!("2")
        )
    );
    let (_, page) = cooee.get("/");
    assert!(
        page.contains("cooee #33") && page.contains("impl · round 2"),
        "{page}"
    );
    assert!(page.contains("My own chat"), "{page}");
    assert!(
        page.contains(r#"value="all" aria-checked="true""#),
        "{page}"
    );

    chip(&mut cooee, "workflow");
    let (_, page) = cooee.get("/");
    assert!(
        page.contains("cooee #33") && !page.contains("My own chat"),
        "{page}"
    );
    assert!(
        page.contains(r#"value="workflow" aria-checked="true""#),
        "{page}"
    );
    let cards = cooee.json("/threads");
    assert!(
        cards
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["done"].is_array()),
        "{cards}"
    );

    chip(&mut cooee, "chats");
    let (_, page) = cooee.get("/");
    assert!(
        !page.contains("cooee #33") && page.contains("My own chat"),
        "{page}"
    );
    let cards = cooee.json("/threads");
    assert!(
        cards
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["done"].is_null()),
        "{cards}"
    );
}

#[test]
fn a_lane_opens_as_a_chat_that_streams_takes_a_reply_and_stops() {
    let dir = scratch("workflow-chat");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let lane = "drover-cooee-33-impl";
    shed.lane(lane, &labels("#33", "impl", "2"));
    let id = id_of(&workflow(&cooee, 1).0[0]);

    let (status, page) = cooee.get(&format!("/t/{id}"));
    assert_eq!(status, 200);
    assert!(page.contains("impl · round 2"), "{page}");
    assert!(
        page.contains(r#"href="https://github.com/o/cooee/pull/60""#),
        "{page}"
    );
    assert!(
        page.contains(r#"id="composer""#) && page.contains(r#"class="stop""#),
        "{page}"
    );

    let cursor = cooee.json(&format!("/threads/{id}/messages"))["cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut events = cooee.events(&id, &cursor, false);
    let reply = json!({ "text": "Say hello", "random_id": "r1" });
    assert_eq!(cooee.post(&format!("/threads/{id}/send"), &reply).0, 200);
    assert_eq!(shed.sent(lane), ["\x1e{\"prompt\":\"Say hello\"}\n"]);
    let got = events.until(|e| is_item(e, "agent"));
    let text: String = got
        .iter()
        .filter(|e| e.event == "text")
        .map(|e| e.data["text"].as_str().unwrap())
        .collect();
    assert_eq!(text, "Hello");
    // The tool's own prompt shows as a message too; the owner's reply shows once.
    shed.input(lane, "\x1e{\"prompt\":\"Fix the bug\"}\n");
    events.until(|e| is_item(e, "agent"));
    let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let you: Vec<&Value> = items
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "you")
        .collect();
    let said: Vec<&str> = you.iter().map(|i| i["text"].as_str().unwrap()).collect();
    assert_eq!(said, ["Say hello", "Fix the bug"]);

    let hang = json!({ "text": "hang on", "random_id": "r2" });
    assert_eq!(cooee.post(&format!("/threads/{id}/send"), &hang).0, 200);
    events.until(|e| e.event == "text");
    assert_eq!(
        cooee.post(&format!("/threads/{id}/cancel"), &json!({})).0,
        200
    );
    assert!(shed.sent(lane).contains(&"\x1e\"cancel\"\n".to_owned()));
    events.until(|e| is_item(e, "agent") && e.data["stop_reason"] == "cancelled");

    // Its tool closes it, not the owner.
    let (status, _) = cooee.post(&format!("/threads/{id}/close"), &json!({}));
    assert_eq!(status, 400);
    assert_eq!(shed.calls("kill", lane), Vec::<Value>::new());
}

#[test]
fn finished_lanes_fold_under_done_today_and_go_once_their_tool_removes_them() {
    let dir = scratch("workflow-done");
    let shed = FakeShed::start(&dir.join("shed"), version());
    // Running before cooee starts: listed from shed's terminals.
    shed.lane("lane-rev", &labels("#31", "rev", "1"));
    shed.report("lane-rev", "blocked", None);
    let mut cooee = Cooee::start(&dir, &shed.dir);
    shed.lane("lane-impl", &labels("#32", "impl", "1"));
    shed.lane("lane-fix", &labels("#33", "fix", "3"));
    workflow(&cooee, 3);
    shed.report("lane-impl", "working", None);
    let fix = workflow(&cooee, 3)
        .0
        .iter()
        .find(|r| r["workflow"]["role"] == "fix")
        .map(id_of)
        .unwrap();
    let hello = json!({ "text": "Say hello", "random_id": "r1" });
    assert_eq!(cooee.post(&format!("/threads/{fix}/send"), &hello).0, 200);

    let roles = |rows: &[Value]| -> Vec<String> {
        let role = |r: &Value| r["workflow"]["role"].as_str().unwrap().to_owned();
        rows.iter().map(role).collect()
    };
    let mut folded = (Vec::new(), Vec::new());
    for _ in 0..250 {
        folded = workflow(&cooee, 3);
        if folded.1.len() == 1 {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    // Needs-you first, then working; the finished one folded.
    assert_eq!(roles(&folded.0), ["rev", "impl"]);
    assert_eq!(roles(&folded.1), ["fix"]);
    let (_, page) = cooee.get("/");
    let fold = page.find("1 done today").expect("the fold");
    let details = page[..fold].rfind("<details>").unwrap();
    assert!(page[details..].find("fix · round 3").is_some(), "{page}");

    // The tool removes it: gone from the list (here at the restart's look at shed's terminals).
    shed.vanish("lane-fix");
    drop(cooee);
    cooee = Cooee::start(&dir, &shed.dir);
    let (rows, done) = workflow(&cooee, 2);
    assert_eq!(
        (roles(&rows), done.len()),
        (vec!["rev".to_owned(), "impl".to_owned()], 0)
    );
}
