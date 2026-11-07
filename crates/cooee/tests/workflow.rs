//! Workflow (#33) end to end: ACP sessions another tool started in the fake shed, known only by their labels, listed
//! by `GET /threads` (drover's Workflow tab shows them) but never among the agents (#86); each opens as a normal chat
//! (streamed reply, a reply and [stop] that reach its terminal); one goes once the tool removes it.

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

/// The Workflow sessions `GET /threads` lists, waiting until there are `n`.
fn workflow(cooee: &Cooee, n: usize) -> Vec<Value> {
    for _ in 0..250 {
        let rows = cooee.json("/threads");
        let rows: Vec<Value> = rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["workflow"].is_object())
            .cloned()
            .collect();
        if rows.len() == n {
            return rows;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("no {n} workflow sessions: {}", cooee.json("/threads"));
}

fn id_of(row: &Value) -> String {
    row["id"].as_str().unwrap().to_owned()
}

#[test]
fn a_lane_is_listed_with_its_labels_but_never_among_the_agents() {
    let dir = scratch("workflow-list");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let folder = dir.join("root/project");
    let new =
        json!({ "agent": "fake-acp", "folder": folder, "text": "My own chat", "random_id": "s1" });
    let (_, body) = cooee.post("/threads", &new);
    let own = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    shed.lane("drover-cooee-33-impl", &labels("#33", "impl", "2"));

    let rows = workflow(&cooee, 1);
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
        !page.contains("cooee #33")
            && !page.contains(&format!("/t/{}", id_of(&rows[0])))
            && page.contains(&format!(r#"href="/t/{own}""#)),
        "{page}"
    );
}

#[test]
fn a_lane_opens_as_a_chat_that_streams_takes_a_reply_and_stops() {
    let dir = scratch("workflow-chat");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let lane = "drover-cooee-33-impl";
    shed.lane(lane, &labels("#33", "impl", "2"));
    let id = id_of(&workflow(&cooee, 1)[0]);

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
fn a_lane_goes_once_its_tool_removes_it() {
    let dir = scratch("workflow-done");
    let shed = FakeShed::start(&dir.join("shed"), version());
    // Running before cooee starts: listed from shed's terminals.
    shed.lane("lane-rev", &labels("#31", "rev", "1"));
    let mut cooee = Cooee::start(&dir, &shed.dir);
    shed.lane("lane-impl", &labels("#32", "impl", "1"));
    shed.lane("lane-fix", &labels("#33", "fix", "3"));
    workflow(&cooee, 3);

    // The tool removes it: gone from the list (here at the restart's look at shed's terminals).
    shed.vanish("lane-fix");
    drop(cooee);
    cooee = Cooee::start(&dir, &shed.dir);
    let mut roles: Vec<String> = workflow(&cooee, 2)
        .iter()
        .map(|r| r["workflow"]["role"].as_str().unwrap().to_owned())
        .collect();
    roles.sort();
    assert_eq!(roles, ["impl", "rev"]);
}
