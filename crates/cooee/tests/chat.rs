//! Chat end to end: `cooee serve` against a fake shed server on a Unix socket (tests/support). A new thread, a
//! prompt and its streamed reply, a tool call, an approval, a cancel, a reconnect with `Last-Event-ID`, a restart
//! that re-attaches, closing, and a shed of another version.

mod support;

use std::{fs, path::PathBuf, thread};

use serde_json::{Value, json};
use support::{Cooee, Events, FakeShed, Sse, is_item, scratch, version};

struct Setup {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    Setup { dir, shed, cooee }
}

impl Setup {
    fn folder(&self) -> PathBuf {
        self.dir.join("root/project")
    }

    /// Starts a thread with `text`; returns its id.
    fn create(&self, text: &str) -> String {
        // Each start its own id, as the page makes one: a repeated id adopts the thread it started.
        static STARTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = STARTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let new = json!({ "agent": "fake-acp", "folder": self.folder(), "text": text, "random_id": format!("start-{n}") });
        let (status, body) = self.cooee.post("/threads", &new);
        assert_eq!(status, 200, "{body}");
        serde_json::from_str::<Value>(&body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    /// The thread's events from the first one this run.
    fn events(&self, id: &str) -> Events {
        let cursor = self.cooee.json(&format!("/threads/{id}/messages"))["cursor"]
            .as_str()
            .unwrap()
            .to_owned();
        let boot = cursor.split('.').next().unwrap();
        self.cooee.events(id, &format!("{boot}.0"), false)
    }

    fn send(&self, id: &str, text: &str, random_id: &str) -> (u16, String) {
        self.cooee.post(
            &format!("/threads/{id}/send"),
            &json!({ "text": text, "random_id": random_id }),
        )
    }

    fn items(&self, id: &str) -> Vec<Value> {
        self.cooee.json(&format!("/threads/{id}/messages"))["items"]
            .as_array()
            .unwrap()
            .clone()
    }
}

fn text_of(events: &[Sse]) -> String {
    events
        .iter()
        .filter(|e| e.event == "text")
        .map(|e| e.data["text"].as_str().unwrap())
        .collect()
}

fn terminal(id: &str) -> String {
    format!("cooee-{id}")
}

#[test]
fn new_thread_runs_the_agent_in_an_allowed_folder_and_streams_the_reply() {
    let s = setup("new");
    let outside = json!({ "agent": "fake-acp", "folder": s.dir, "text": "hi", "random_id": "x" });
    let (status, body) = s.cooee.post("/threads", &outside);
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("not inside a configured root"), "{body}");
    assert!(!s.shed.cli_log().contains(" run "), "{}", s.shed.cli_log());

    let id = s.create("Say hello");
    let folder = s.folder().canonicalize().unwrap();
    let run = format!(
        "--dir {} --json run --acp fake-acp --name cooee-{id} --cwd {} --label cooee=1 --label cooee.thread={id}",
        s.shed.dir.display(),
        folder.display()
    );
    assert!(s.shed.cli_log().contains(&run), "{}", s.shed.cli_log());
    assert_eq!(
        s.shed.sent(&terminal(&id)),
        ["\x1e{\"prompt\":\"Say hello\"}\n"]
    );

    let events = s.events(&id).until(|e| is_item(e, "agent"));
    assert!(is_item(&events[0], "you") && events[0].data["text"] == "Say hello");
    assert_eq!(text_of(&events), "Hello");
    let reply = events.last().unwrap();
    assert_eq!(
        (&reply.data["text"], &reply.data["stop_reason"]),
        (&json!("Hello"), &json!("end_turn"))
    );

    let kinds: Vec<Value> = s.items(&id).iter().map(|i| i["kind"].clone()).collect();
    assert_eq!(kinds, ["you", "agent"]);
    let (status, page) = s.cooee.get("/");
    assert_eq!(status, 200);
    assert!(
        page.contains("Say hello") && page.contains("fake-acp"),
        "{page}"
    );
    let (status, page) = s.cooee.get(&format!("/t/{id}"));
    assert!(
        status == 200 && page.contains(&format!("data-thread=\"{id}\"")),
        "{page}"
    );
}

#[test]
fn a_tool_call_is_one_row_updated_in_place_and_a_retried_send_is_stored_once() {
    let s = setup("tool");
    let id = s.create("hi");
    let mut events = s.events(&id);
    events.until(|e| is_item(e, "agent"));
    let (status, first) = s.send(&id, "use a tool", "r1");
    assert_eq!(status, 200, "{first}");
    let (_, again) = s.send(&id, "use a tool", "r1");
    assert_eq!(first, again);
    let prompts = s.shed.sent(&terminal(&id));
    assert_eq!(
        prompts.iter().filter(|t| t.contains("use a tool")).count(),
        1
    );

    let events = events.until(|e| is_item(e, "agent"));
    let tools: Vec<&Sse> = events.iter().filter(|e| is_item(e, "tool")).collect();
    let statuses: Vec<&Value> = tools.iter().map(|e| &e.data["status"]).collect();
    assert_eq!(statuses, ["pending", "in_progress", "completed"]);
    assert!(tools.iter().all(|e| e.data["id"] == tools[0].data["id"]));
    assert!(tools[1].data.get("ended").is_none() && tools[2].data["ended"].is_u64());
    assert_eq!(text_of(&events), "Read it.");
    let kinds: Vec<Value> = s.items(&id).iter().map(|i| i["kind"].clone()).collect();
    assert_eq!(kinds, ["you", "agent", "you", "tool", "agent"]);
}

#[test]
fn an_approval_is_sent_to_shed_once_however_often_it_is_tapped() {
    let s = setup("approve");
    let id = s.create("please approve an edit");
    let mut events = s.events(&id);
    let ask = events.until(|e| is_item(e, "approval")).pop().unwrap();
    let options: Vec<&Value> = ask.data["options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| &o["id"])
        .collect();
    assert_eq!(options, ["allow", "always", "reject"]);
    let item = ask.data["id"].as_u64().unwrap();
    let path = format!("/threads/{id}/approve");
    let tap = json!({ "item": item, "option": "allow" });
    thread::scope(|scope| {
        for _ in 0..3 {
            scope.spawn(|| assert_eq!(s.cooee.post(&path, &tap).0, 200));
        }
    });
    assert_eq!(s.cooee.post(&path, &tap).0, 200);
    let approvals = s.shed.sent(&terminal(&id));
    let approvals: Vec<&String> = approvals
        .iter()
        .filter(|t| t.contains("approve\":"))
        .collect();
    assert_eq!(approvals, ["\x1e{\"approve\":\"allow\"}\n"]);

    let events = events.until(|e| is_item(e, "agent"));
    let answered = events
        .iter()
        .find(|e| is_item(e, "approval") && e.data["chosen"] == "allow");
    assert!(answered.is_some(), "{events:?}");
    let tool = events.iter().rfind(|e| is_item(e, "tool")).unwrap();
    assert_eq!(tool.data["status"], "completed");
    assert_eq!(events.last().unwrap().data["text"], "Edited.");
}

#[test]
fn cancel_ends_the_turn_and_writes_back_the_tool_call_in_progress() {
    let s = setup("cancel");
    let id = s.create("hang on");
    let mut events = s.events(&id);
    events.until(|e| e.event == "text");
    assert_eq!(
        s.cooee.post(&format!("/threads/{id}/cancel"), &json!({})).0,
        200
    );
    assert!(
        s.shed
            .sent(&terminal(&id))
            .contains(&"\x1e\"cancel\"\n".to_owned())
    );
    let events = events.until(|e| e.event == "state" && e.data["state"] == "done");
    let tool = events.iter().rfind(|e| is_item(e, "tool")).unwrap();
    assert_eq!(tool.data["status"], "cancelled");
    assert!(tool.data["ended"].is_u64());
    let reply = events.iter().rfind(|e| is_item(e, "agent")).unwrap();
    assert_eq!(reply.data["stop_reason"], "cancelled");
}

#[test]
fn a_reconnect_with_last_event_id_replays_without_gaps_or_duplicates() {
    let s = setup("reconnect");
    let id = s.create("use a tool");
    let all = s.events(&id).until(|e| is_item(e, "agent"));
    assert!(all.len() > 4, "{all:?}");
    let cursor = s.cooee.json(&format!("/threads/{id}/messages"))["cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let boot = cursor.split('.').next().unwrap();
    for cut in 1..all.len() {
        // A page that saw the first `cut` events and reconnects gets exactly the rest.
        let mut events = s.cooee.events(&id, &all[cut - 1].id, true);
        let rest = events.until(|e| e.id == all.last().unwrap().id);
        let ids = |v: &[Sse]| v.iter().map(|e| e.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&rest), ids(&all[cut..]), "after {}", all[cut - 1].id);
    }
    // A cursor from another run gets a reset: the page reloads its slice.
    for stale in ["0.0".to_owned(), format!("{boot}.99999")] {
        assert_eq!(s.cooee.events(&id, &stale, true).next().event, "reset");
    }
}

#[test]
fn a_restarted_cooee_reattaches_to_its_terminals_and_catches_up() {
    let mut s = setup("restart");
    let id = s.create("hang on");
    s.events(&id).until(|e| e.event == "text");
    drop(s.cooee);
    // While cooee is down the turn is cancelled (as `shed cancel` would).
    s.shed.input(&terminal(&id), "\x1e\"cancel\"\n");
    s.cooee = Cooee::start(&s.dir, &s.shed.dir);
    let mut events = s.events(&id);
    events.until(|e| is_item(e, "agent") && e.data["stop_reason"] == "cancelled");
    let gets = s
        .shed
        .requests
        .lock()
        .unwrap()
        .iter()
        .filter(|(m, p)| m == "get" && p["name"] == terminal(&id))
        .count();
    assert!(gets >= 1, "re-attached by name");
    assert_eq!(s.send(&id, "again", "r2").0, 200);
    events.until(|e| is_item(e, "agent") && e.data["text"] == "Hello");
    let kinds: Vec<Value> = s.items(&id).iter().map(|i| i["kind"].clone()).collect();
    assert_eq!(kinds, ["you", "tool", "agent", "you", "agent"]);
}

#[test]
fn closing_a_thread_kills_and_removes_its_terminal() {
    let s = setup("close");
    let id = s.create("hi");
    let mut events = s.events(&id);
    events.until(|e| is_item(e, "agent"));
    assert_eq!(
        s.cooee.post(&format!("/threads/{id}/close"), &json!({})).0,
        200
    );
    let calls: Vec<String> = s
        .shed
        .requests
        .lock()
        .unwrap()
        .iter()
        .filter(|(m, p)| (m == "kill" || m == "rm") && p["name"] == terminal(&id))
        .map(|(m, _)| m.clone())
        .collect();
    assert_eq!(calls, ["kill", "rm"]);
    events.until(|e| e.event == "state" && e.data["state"] == "ended");
    assert_eq!(s.send(&id, "more", "r9").0, 400);
    let stored = fs::read_to_string(s.dir.join(format!("data/threads/{id}.json"))).unwrap();
    assert!(stored.contains("\"closed\": true"), "{stored}");
}

#[test]
fn a_shed_of_another_version_is_refused_naming_both_versions() {
    let dir = scratch("version");
    let shed = FakeShed::start(&dir.join("shed"), "9.9.9");
    let cooee = Cooee::start(&dir, &shed.dir);
    let new = json!({ "agent": "fake-acp", "folder": dir.join("root/project"), "text": "hi", "random_id": "x" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 502, "{body}");
    assert!(
        body.contains("version mismatch") && body.contains(version()) && body.contains("9.9.9"),
        "{body}"
    );
    assert!(
        !shed.cli_log().contains(" run "),
        "nothing started: {}",
        shed.cli_log()
    );
}

/// The fake shed for the UI tests (web/e2e/serve.sh): serves in `$FAKE_SHED_DIR` until killed.
#[test]
#[ignore = "a server for web/e2e, not a test"]
fn fake_shed_server() {
    let Some(dir) = std::env::var_os("FAKE_SHED_DIR") else {
        return;
    };
    let _shed = FakeShed::start(PathBuf::from(dir).as_path(), version());
    loop {
        thread::park();
    }
}
