//! #149: a thread on another machine starts and runs there, against the fake shed's remote (`--on <m> run`, its
//! terminals `<m>/<name>`, refused `NoLink` while the link is down): reply, approval and cancel go to `<m>/<name>`;
//! an offline or unknown machine refuses the start or the send with when it was last seen; a restart finds the
//! session again.

mod support;

use std::{
    fs,
    path::Path,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use support::{Cooee, Events, FakeShed, is_item, scratch, version};

fn setup(name: &str) -> (std::path::PathBuf, FakeShed, Cooee) {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let machines = json!([{ "name": "box", "target": "box.example" }]);
    fs::write(shed.dir.join("machines.json"), machines.to_string()).unwrap();
    shed.link("box", "up", unix_now(), None);
    let cooee = Cooee::start(&dir, &shed.dir);
    (dir, shed, cooee)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn until(what: &str, f: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !f() {
        assert!(Instant::now() < deadline, "never: {what}");
        thread::sleep(Duration::from_millis(20));
    }
}

fn start(dir: &Path, machine: &str, text: &str, background: bool) -> Value {
    json!({ "agent": "fake-acp", "folder": dir.join("root/project"), "text": text, "random_id": format!("start-{text}"),
            "machine": machine, "background": background })
}

fn id_of(body: &str) -> String {
    serde_json::from_str::<Value>(body).unwrap()["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{body}"))
        .to_owned()
}

fn events(cooee: &Cooee, id: &str) -> Events {
    let cursor = cooee.json(&format!("/threads/{id}/messages"))["cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    cooee.events(id, &cursor, false)
}

fn row(cooee: &Cooee, id: &str) -> Value {
    let list = cooee.json("/threads");
    let rows = list.as_array().unwrap().iter();
    rows.into_iter().find(|r| r["id"] == id).cloned().unwrap()
}

#[test]
fn a_thread_on_a_machine_starts_replies_approves_and_cancels_there_and_is_found_after_a_restart() {
    let (dir, shed, cooee) = setup("remote-run");
    let (status, body) = cooee.post("/threads", &start(&dir, "box", "hi", false));
    assert_eq!(status, 200, "{body}");
    let id = id_of(&body);
    let lane = format!("box/cooee-{id}");

    let log = shed.cli_log();
    let run = log.lines().rfind(|l| l.contains(" run ")).unwrap();
    assert!(
        run.contains(&format!("--on box run --acp fake-acp --name cooee-{id} ")),
        "{run}"
    );
    assert_eq!(shed.calls("wait", &lane).len(), 1);
    assert_eq!(shed.sent(&lane), ["\x1e{\"prompt\":\"hi\"}\n"]);
    let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    assert_eq!(
        (&items[0]["kind"], &items[0]["text"]),
        (&json!("note"), &json!("runs on box")),
        "{items}"
    );
    assert_eq!(row(&cooee, &id)["machine"], "box");

    let mut ev = events(&cooee, &id);
    let send = |text: &str| {
        let body = json!({ "text": text, "random_id": text });
        cooee.post(&format!("/threads/{id}/send"), &body)
    };
    assert_eq!(send("echo there").0, 200);
    let got = ev.until(|e| is_item(e, "agent") && e.data["text"] == "there");
    assert!(!got.is_empty());

    assert_eq!(send("please approve an edit").0, 200);
    let ask = ev.until(|e| is_item(e, "approval")).pop().unwrap();
    let tap = json!({ "item": ask.data["id"], "option": "allow" });
    let path = format!("/threads/{id}/approve");
    assert_eq!(cooee.post(&path, &tap).0, 200);
    assert!(
        shed.sent(&lane)
            .contains(&"\x1e{\"approve\":\"allow\"}\n".to_owned())
    );
    ev.until(|e| is_item(e, "agent") && e.data["text"] == "Edited.");

    assert_eq!(send("hang on").0, 200);
    ev.until(|e| e.event == "text");
    let cancel = format!("/threads/{id}/cancel");
    assert_eq!(cooee.post(&cancel, &json!({})).0, 200);
    assert!(shed.sent(&lane).contains(&"\x1e\"cancel\"\n".to_owned()));
    ev.until(|e| is_item(e, "agent") && e.data["stop_reason"] == "cancelled");

    drop(ev);
    drop(cooee);
    let gets = shed.calls("get", &lane).len();
    let cooee = Cooee::start(&dir, &shed.dir);
    until("box's session is found again", || {
        shed.calls("get", &lane).len() > gets
    });
    assert_eq!(row(&cooee, &id)["machine"], "box");
    let mut ev = events(&cooee, &id);
    let body = json!({ "text": "echo again", "random_id": "again" });
    let again = cooee.post(&format!("/threads/{id}/send"), &body);
    assert_eq!(again.0, 200, "{}", again.1);
    ev.until(|e| is_item(e, "agent") && e.data["text"] == "again");
    assert_eq!(
        shed.cli_log().matches(" run ").count(),
        1,
        "never started twice"
    );
}

#[test]
fn an_offline_or_unknown_machine_refuses_the_start_and_the_send_with_when_it_was_last_seen() {
    let (dir, shed, cooee) = setup("remote-offline");
    let (status, body) = cooee.post("/threads", &start(&dir, "box", "hi", false));
    assert_eq!(status, 200, "{body}");
    let id = id_of(&body);
    let lane = format!("box/cooee-{id}");

    let down = Some("ssh: connect to host box.example: Connection refused");
    shed.link("box", "down", unix_now() - 12 * 60, down);
    let why = "box is offline, last seen 12 min ago: ssh: connect to host box.example: Connection refused";

    let body = json!({ "text": "echo more", "random_id": "more" });
    let (status, said) = cooee.post(&format!("/threads/{id}/send"), &body);
    assert_eq!((status, said.as_str()), (400, why));
    assert_eq!(shed.sent(&lane).len(), 1);

    let runs = shed.cli_log().matches(" run ").count();
    let (status, said) = cooee.post("/threads", &start(&dir, "box", "later", false));
    assert_eq!((status, said.as_str()), (400, why));
    let (status, body) = cooee.post("/threads", &start(&dir, "box", "bg", true));
    assert_eq!(status, 200, "{body}");
    let messages = format!("/threads/{}/messages", id_of(&body));
    until("the start failed", || {
        cooee.json(&messages)["turn"]["state"] == "unconnected"
    });
    let turn = cooee.json(&messages)["turn"].clone();
    assert_eq!(turn["note"], format!("fake stopped: {why}"), "{turn}");
    assert_eq!(shed.cli_log().matches(" run ").count(), runs);

    drop(cooee);
    let cooee = Cooee::start(&dir, &shed.dir);
    let (status, body) = cooee.post("/threads", &start(&dir, "", "here", false));
    assert_eq!(status, 200, "{body}");
    let here = id_of(&body);
    let messages = format!("/threads/{here}/messages");
    until("the local thread's reply", || {
        let items = cooee.json(&messages)["items"].clone();
        items
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["kind"] == "agent")
    });

    fs::write(
        shed.dir.join("machines.json"),
        json!([{ "name": "box" }, { "name": "gone" }]).to_string(),
    )
    .unwrap();
    let (status, said) = cooee.post("/threads", &start(&dir, "gone", "x", false));
    assert_eq!(
        (status, said.as_str()),
        (400, "no machine gone: add it with shed machine add")
    );
}
