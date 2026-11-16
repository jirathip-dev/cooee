//! #151: `POST /threads/{id}/move {machine}` against the fake shed with machine `box` up. Idle, the thread checks box
//! is ready (#150) for it, starts its agent there and closes the old session; the next message reaches box after a
//! hand-over with its last turns, and the transcript stays one thread. Refused while a turn works, or while box isn't
//! ready, and then nothing starts there.

mod support;

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use support::{Cooee, Events, FakeShed, is_item, scratch, version};

fn setup(name: &str) -> (PathBuf, FakeShed, Cooee) {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let machines = json!([{ "name": "box", "target": "box.example" }]);
    fs::write(shed.dir.join("machines.json"), machines.to_string()).unwrap();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    shed.link("box", "up", now.as_secs(), None);
    let script = |path: &Path, body: &str| {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, format!("#!/bin/sh\n{body}")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    };
    script(&dir.join("bin/fake-cli"), "exit 0\n");
    fs::write(dir.join("signed-in"), "").unwrap();
    let flag = dir.join("signed-in");
    script(
        &dir.join("status.sh"),
        &format!("test -f {:?}\n", flag.to_string_lossy()),
    );
    let provider = format!(
        "[[providers]]\nname = \"fake\"\nagent = \"fake-acp\"\nlogin = [{:?}]\nstatus = [{:?}]\ninstall = \"get fake-cli\"\n",
        dir.join("bin/fake-cli").to_string_lossy(),
        dir.join("status.sh").to_string_lossy(),
    );
    let cooee = Cooee::start_with(&dir, &shed.dir, &provider);
    (dir, shed, cooee)
}

fn idle_thread(dir: &Path, cooee: &Cooee) -> (String, Events) {
    let new = json!({ "agent": "fake-acp", "folder": dir.join("root/project"), "text": "echo hello",
                      "random_id": "start" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let deadline = Instant::now() + Duration::from_secs(10);
    let messages = loop {
        let messages = cooee.json(&format!("/threads/{id}/messages"));
        let items = messages["items"].as_array().unwrap();
        if items
            .iter()
            .any(|i| i["kind"] == "agent" && i["text"] == "hello")
        {
            break messages;
        }
        assert!(Instant::now() < deadline, "no reply: {messages}");
        thread::sleep(Duration::from_millis(20));
    };
    let ev = cooee.events(&id, messages["cursor"].as_str().unwrap(), true);
    (id, ev)
}

fn move_to(cooee: &Cooee, id: &str, machine: &str) -> (u16, String) {
    cooee.post(
        &format!("/threads/{id}/move"),
        &json!({ "machine": machine }),
    )
}

fn kinds(cooee: &Cooee, id: &str) -> Vec<String> {
    let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let items = items.as_array().unwrap().iter();
    items
        .map(|i| i["kind"].as_str().unwrap().to_owned())
        .collect()
}

fn row(cooee: &Cooee, id: &str) -> Value {
    let list = cooee.json("/threads");
    let rows = list.as_array().unwrap().iter();
    rows.into_iter().find(|r| r["id"] == id).cloned().unwrap()
}

fn checks_on_box(shed: &FakeShed) -> usize {
    let requests = shed.requests.lock().unwrap();
    let runs = requests
        .iter()
        .filter(|(m, p)| m == "run" && p["machine"] == "box");
    runs.count()
}

fn sessions_on_box(shed: &FakeShed) -> Vec<String> {
    let log = shed.cli_log();
    let runs = log.lines().filter(|l| l.contains("--on box run --acp"));
    runs.map(|l| {
        l.split("--name ")
            .nth(1)
            .unwrap()
            .split(' ')
            .next()
            .unwrap()
            .to_owned()
    })
    .collect()
}

#[test]
fn an_idle_thread_moves_after_box_is_checked_and_the_next_message_starts_there_with_a_hand_over() {
    let (dir, shed, cooee) = setup("move-idle");
    let (id, mut ev) = idle_thread(&dir, &cooee);

    let (status, body) = move_to(&cooee, &id, "box");
    assert_eq!(status, 200, "{body}");
    let note: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        (&note["kind"], &note["text"]),
        (&json!("note"), &json!("moved to box"))
    );
    assert_ne!(checks_on_box(&shed), 0, "box was checked");
    let started = sessions_on_box(&shed);
    assert_eq!(started.len(), 1, "{started:?}");
    let lane = format!("box/{}", started[0]);
    assert!(lane.starts_with(&format!("box/cooee-{id}-")), "{lane}");
    assert_eq!(
        shed.calls("kill", &format!("cooee-{id}")).len(),
        1,
        "old session closed"
    );
    assert_eq!(row(&cooee, &id)["machine"], "box");

    let send = |text: &str| {
        let body = json!({ "text": text, "random_id": text });
        cooee.post(&format!("/threads/{id}/send"), &body)
    };
    assert_eq!(send("echo there").0, 200);
    ev.until(|e| is_item(e, "agent") && e.data["text"] == "Hello");
    assert_eq!(send("echo again").0, 200);
    ev.until(|e| is_item(e, "agent") && e.data["text"] == "again");
    let prompts = shed.sent(&lane);
    assert_eq!(prompts.len(), 2, "{prompts:?}");
    let project = dir.join("root/project").canonicalize().unwrap();
    let handover = &prompts[0];
    assert!(
        handover.contains(&format!(
            "moved from {} to {} on box",
            project.display(),
            project.display()
        )) && handover.contains("Owner: echo hello")
            && handover.contains("You: hello")
            && handover.contains("echo there"),
        "{handover}"
    );
    assert_eq!(prompts[1], "\x1e{\"prompt\":\"echo again\"}\n");
    assert_eq!(
        shed.sent(&format!("cooee-{id}")).len(),
        1,
        "nothing more to the old session"
    );
    assert_eq!(
        kinds(&cooee, &id),
        ["you", "agent", "note", "you", "agent", "you", "agent"]
    );
}

#[test]
fn a_move_is_refused_while_a_turn_works_and_goes_once_it_is_stopped() {
    let (dir, shed, cooee) = setup("move-working");
    let (id, mut ev) = idle_thread(&dir, &cooee);
    let body = json!({ "text": "hang on", "random_id": "hang" });
    assert_eq!(cooee.post(&format!("/threads/{id}/send"), &body).0, 200);
    ev.until(|e| e.event == "text");

    let (status, said) = move_to(&cooee, &id, "box");
    assert_eq!(
        (status, said.as_str()),
        (400, "a turn is under way: stop it first")
    );
    assert_eq!(sessions_on_box(&shed), Vec::<String>::new());
    assert_eq!(row(&cooee, &id)["machine"], "");

    assert_eq!(
        cooee.post(&format!("/threads/{id}/cancel"), &json!({})).0,
        200
    );
    ev.until(|e| is_item(e, "agent") && e.data["stop_reason"] == "cancelled");
    let (status, body) = move_to(&cooee, &id, "box");
    assert_eq!(status, 200, "{body}");
    assert_eq!(sessions_on_box(&shed).len(), 1);
}

#[test]
fn a_move_to_a_machine_not_ready_is_refused_with_its_failed_checks_and_starts_nothing_there() {
    let (dir, shed, cooee) = setup("move-unready");
    let (id, _ev) = idle_thread(&dir, &cooee);
    fs::remove_file(dir.join("signed-in")).unwrap();

    let (status, said) = move_to(&cooee, &id, "box");
    assert_eq!(status, 400, "{said}");
    assert_eq!(
        said,
        format!(
            "box isn't ready:\n{id}: fake account default signed in: no\n     fix: sign in to fake on box"
        )
    );
    assert_ne!(checks_on_box(&shed), 0);
    assert_eq!(sessions_on_box(&shed), Vec::<String>::new());
    assert_eq!(shed.calls("kill", &format!("cooee-{id}")).len(), 0);
    assert_eq!(kinds(&cooee, &id), ["you", "agent"]);
    assert_eq!(row(&cooee, &id)["machine"], "");
}
