//! Workflow (#33) end to end: ACP sessions another tool started in the fake shed, known only by their labels, listed
//! by `GET /threads` (drover's Workflow tab shows them) but never among the agents (#86); each opens as a normal chat
//! (streamed reply, a reply and [stop] that reach its terminal); one goes once the tool removes it.

mod support;

use std::{thread, time::Duration};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, is_item, scratch, version};

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
    let list = cooee.json("/threads");
    let mine = list.as_array().unwrap().iter().find(|r| r["id"] == own);
    let mine = mine.expect("the own chat is listed");
    assert!(
        mine["title"] == "My own chat" && !mine["workflow"].is_object(),
        "{mine}"
    );
    assert_ne!(id_of(&rows[0]), own);
}

#[test]
fn a_lane_opens_as_a_chat_that_streams_takes_a_reply_and_stops() {
    let dir = scratch("workflow-chat");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let lane = "drover-cooee-33-impl";
    shed.lane(lane, &labels("#33", "impl", "2"));
    let id = id_of(&workflow(&cooee, 1)[0]);

    let w = &workflow(&cooee, 1)[0]["workflow"];
    assert_eq!(
        (&w["role"], &w["round"], &w["url"]),
        (
            &json!("impl"),
            &json!("2"),
            &json!("https://github.com/o/cooee/pull/60")
        )
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
}

#[test]
fn a_lane_goes_once_its_tool_removes_it() {
    let dir = scratch("workflow-done");
    let shed = FakeShed::start(&dir.join("shed"), version());
    shed.lane("lane-rev", &labels("#31", "rev", "1"));
    let mut cooee = Cooee::start(&dir, &shed.dir);
    shed.lane("lane-impl", &labels("#32", "impl", "1"));
    shed.lane("lane-fix", &labels("#33", "fix", "3"));
    workflow(&cooee, 3);

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

fn row(cooee: &Cooee, id: &str, until: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..250 {
        let rows = cooee.json("/threads");
        let found = rows.as_array().unwrap().iter().find(|r| r["id"] == id);
        if let Some(r) = found.filter(|r| until(r)) {
            return r.clone();
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("no such row {id}: {}", cooee.json("/threads"));
}

fn lane_id(cooee: &Cooee, item: &str) -> String {
    let rows = cooee.json("/threads");
    let found = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["workflow"]["item"] == item);
    id_of(found.unwrap())
}

#[test]
fn a_parked_dead_or_offline_lane_lists_its_state_and_reason() {
    let dir = scratch("workflow-states");
    let shed = FakeShed::start(&dir.join("shed"), version());
    shed.link("m1", "up", 0, None);
    let mut cooee = Cooee::start(&dir, &shed.dir);
    shed.lane("lane-park", &labels("#1", "impl", "1"));
    shed.lane("lane-dead", &labels("#2", "impl", "1"));
    shed.lane("m1/lane-off", &labels("#3", "impl", "1"));
    workflow(&cooee, 3);

    shed.link("m1", "down", 0, Some("refused"));
    drop(cooee);
    cooee = Cooee::start(&dir, &shed.dir);
    let off = row(&cooee, &lane_id(&cooee, "#3"), |r| r["state"] == "offline");
    assert_eq!(off["workflow"]["item"], "#3");
    shed.report("lane-park", "parked", Some("waiting on #12"));
    shed.end("lane-dead");
    let parked = row(&cooee, &lane_id(&cooee, "#1"), |r| r["state"] == "parked");
    assert_eq!(parked["note"], "waiting on #12");
    let dead = row(&cooee, &lane_id(&cooee, "#2"), |r| r["state"] == "ended");
    assert_eq!(dead["note"], "The agent exited (1).");

    shed.link("m1", "up", 0, None);
    drop(cooee);
    cooee = Cooee::start(&dir, &shed.dir);
    let back = row(&cooee, &lane_id(&cooee, "#3"), |r| r["state"] != "offline");
    assert!(back.get("note").is_none(), "{back}");
    assert_eq!(workflow(&cooee, 3).len(), 3);
}

#[test]
fn only_a_dead_or_unclaimed_lane_closes_once_and_is_archived() {
    let dir = scratch("workflow-close");
    let build = dir.join("data/plugins/drover/builds/c0ffee12345");
    std::fs::create_dir_all(&build).unwrap();
    let manifest = "name = \"drover\"\nversion = \"0.1.0\"\nbuild = \"true\"\nrun = \"true\"\nclaims = [\"drover\"]\n\n\
                    [[places]]\ntype = \"menu-row\"\nlabel = \"Queue\"\nscreen = \"/screens/queue\"\n\n\
                    [permissions]\nplaces = [\"menu-row\"]\n";
    std::fs::write(build.join("cooee-plugin.toml"), manifest).unwrap();
    let lock = json!([{ "name": "drover", "url": "https://example.com/drover", "commit": "c0ffee12345",
                        "version": "0.1.0", "listed": true, "tabs": [], "port": 1, "enabled": false }]);
    std::fs::write(dir.join("data/plugins.lock"), lock.to_string()).unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let (lane, stray) = ("drover-cooee-9-impl", "stray-1");
    shed.lane(lane, &labels("#9", "impl", "1"));
    let mut other = labels("#10", "impl", "1");
    other[0] = ("source", "other");
    shed.lane(stray, &other);
    workflow(&cooee, 2);
    let (id, foreign) = (lane_id(&cooee, "#9"), lane_id(&cooee, "#10"));
    let close = |id: &str| cooee.post(&format!("/threads/{id}/close"), &json!({})).0;

    assert_eq!(close(&id), 400);
    assert_eq!(shed.calls("kill", lane), Vec::<Value>::new());

    shed.end(lane);
    row(&cooee, &id, |r| r["state"] == "ended");
    assert_eq!((close(&id), close(&id)), (200, 200));
    assert_eq!(shed.calls("kill", lane).len(), 1);
    assert_eq!(shed.calls("rm", lane).len(), 1);
    let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let ended = items
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "ended");
    assert_eq!(ended.count(), 1);
    assert_eq!(row(&cooee, &id, |_| true)["archived"], true);

    assert_eq!(close(&foreign), 200);
    assert_eq!(shed.calls("kill", stray).len(), 1);
    assert_eq!(row(&cooee, &foreign, |_| true)["archived"], true);
}
