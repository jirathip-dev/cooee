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
    let (login, status) = (dir.join("bin/fake-cli"), dir.join("status.sh"));
    shed.record(&json!({ "login": login, "status": status, "install": "get fake-cli" }));
    let provider = "[[providers]]\nname = \"fake\"\nagent = \"fake-acp\"\n";
    let cooee = Cooee::start_with(&dir, &shed.dir, provider);
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
        .filter(|(m, p)| ["exec", "file.read"].contains(&m.as_str()) && p["machine"] == "box");
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

fn replies(cooee: &Cooee, id: &str, n: usize) -> Vec<Value> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
        let items = items.as_array().unwrap().clone();
        let done = items.iter().filter(|i| i["kind"] == "agent").count() == n;
        if done || Instant::now() > deadline {
            assert!(done, "{items:?}");
            return items;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn files(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut all = Vec::new();
    for e in fs::read_dir(dir).unwrap().map(Result::unwrap) {
        let path = e.path();
        if path.is_dir() {
            all.extend(files(&path));
        } else {
            all.push((path.clone(), fs::read(&path).unwrap()));
        }
    }
    all.sort();
    all
}

#[test]
fn a_switch_starts_the_other_agent_with_the_brief_and_a_handoff_in_the_same_thread_and_keeps_the_persona()
 {
    let dir = scratch("switch-agent");
    let (shed, cooee, id) = ada_with_a_skill_memory_and_two_agents(&dir);
    let messages = replies(&cooee, &id, 1);
    let reply = messages[1]["text"].as_str().unwrap().to_owned();
    let persona = || {
        let shared = ["config.toml", "routines.toml", "data/connectors.json"];
        let shared = shared.map(|f| fs::read(dir.join(f)).ok());
        (files(&dir.join("agents")), shared)
    };
    let kept = persona();
    let switch =
        |agent: &str| cooee.post(&format!("/threads/{id}/agent"), &json!({ "agent": agent }));

    assert_eq!(switch("nope-acp"), (400, "no agent nope-acp".to_owned()));
    assert_eq!(
        switch("fake-acp"),
        (400, "it already runs on fake-acp".to_owned())
    );
    let (status, body) = switch("other-acp");
    assert_eq!(status, 200, "{body}");
    let note: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(note["text"], "now on other-acp · later turns run there");
    let log = shed.cli_log();
    let runs: Vec<&str> = log
        .lines()
        .filter(|l| l.contains("run --acp other-acp"))
        .collect();
    assert_eq!(runs.len(), 1, "{log}");
    let lane = runs[0]
        .split("--name ")
        .nth(1)
        .unwrap()
        .split(' ')
        .next()
        .unwrap()
        .to_owned();
    assert_eq!(
        shed.calls("kill", &format!("cooee-{id}")).len(),
        1,
        "old session closed"
    );
    assert_the_next_turn_carries_the_handoff_in_the_same_thread(&cooee, &shed, &id, &lane, &reply);
    assert_eq!(kinds(&cooee, &id), ["you", "agent", "note", "you", "agent"]);
    assert_eq!(kept, persona());
}

fn ada_with_a_skill_memory_and_two_agents(dir: &Path) -> (FakeShed, Cooee, String) {
    let home = support::agent(
        dir,
        "ada",
        "name = \"Ada\"\nagent = \"fake-acp\"\n",
        "The code word is PLOVER-7.",
    );
    fs::create_dir_all(home.join("skills/greet")).unwrap();
    fs::write(home.join("skills/greet/SKILL.md"), "Say hi.").unwrap();
    fs::write(home.join("memory.md"), "- likes tea\n").unwrap();
    fs::write(dir.join("routines.toml"), "").unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let agents = json!([
        { "id": "fake-acp", "version": "1.0.0", "distribution": "npx", "cmd": ["fake-acp"] },
        { "id": "other-acp", "version": "1.0.0", "distribution": "npx", "cmd": ["fake-acp"] },
    ]);
    fs::write(shed.dir.join("agents.json"), agents.to_string()).unwrap();
    let cooee = support::Cooee::start(dir, &shed.dir);
    let new = json!({ "agent": "fake-acp", "profile": "ada", "text": "echo hello", "random_id": "start" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    (shed, cooee, id)
}

fn assert_the_next_turn_carries_the_handoff_in_the_same_thread(
    cooee: &Cooee,
    shed: &FakeShed,
    id: &str,
    lane: &str,
    reply: &str,
) {
    let body = json!({ "text": "echo there", "random_id": "there" });
    assert_eq!(cooee.post(&format!("/threads/{id}/send"), &body).0, 200);
    replies(cooee, id, 2);
    let prompts = shed.sent(lane);
    assert_eq!(prompts.len(), 1, "{prompts:?}");
    let first = &prompts[0];
    assert!(
        first.contains("The code word is PLOVER-7.")
            && first.contains("Its first message: echo hello")
            && first.contains(&format!("reply: {reply}"))
            && first.contains(&format!("Older turns: your history tool, thread {id}"))
            && first.contains("echo there"),
        "{first}"
    );
    let list = cooee.json("/threads");
    let rows: Vec<&Value> = list
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["profile"] == "ada")
        .collect();
    assert_eq!(rows.len(), 1, "{list}");
    assert_eq!(
        (&rows[0]["id"], &rows[0]["agent"]),
        (&json!(id), &json!("other-acp"))
    );
}

fn card(cooee: &Cooee, id: &str, n: usize) -> Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
        let cards: Vec<Value> = items
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["kind"] == "approval" && i["form"] == "fallback")
            .cloned()
            .collect();
        if cards.len() >= n || Instant::now() > deadline {
            assert_eq!(cards.len(), n, "{items}");
            return cards[n - 1].clone();
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn options(card: &Value) -> Vec<(String, String)> {
    let all = card["options"].as_array().unwrap().iter();
    all.map(|o| {
        let s = |k: &str| o[k].as_str().unwrap().to_owned();
        (s("id"), s("name"))
    })
    .collect()
}

fn chained() -> (FakeShed, Cooee, String) {
    let dir = scratch("fallback");
    support::agent(
        &dir,
        "ada",
        "name = \"Ada\"\nagent = \"fake-acp\"\n\n[[fallback]]\nagent = \"other-acp\"\n",
        "Be brief.",
    );
    let shed = FakeShed::start(&dir.join("shed"), version());
    let agents = json!([
        { "id": "fake-acp", "version": "1.0.0", "distribution": "npx", "cmd": ["fake-acp"] },
        { "id": "other-acp", "version": "1.0.0", "distribution": "npx", "cmd": ["fake-acp"] },
    ]);
    fs::write(shed.dir.join("agents.json"), agents.to_string()).unwrap();
    let cooee = support::Cooee::start(&dir, &shed.dir);
    let new = json!({ "agent": "fake-acp", "profile": "ada", "text": "echo hello", "random_id": "start" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    replies(&cooee, &id, 1);
    (shed, cooee, id)
}

fn lane(shed: &FakeShed, agent: &str) -> String {
    let log = shed.cli_log();
    let run = log
        .lines()
        .find(|l| l.contains(&format!("run --acp {agent}")));
    let name = run.unwrap().split("--name ").nth(1).unwrap();
    name.split(' ').next().unwrap().to_owned()
}

#[test]
fn a_limit_asks_for_the_next_agent_of_the_chain_and_its_pick_takes_the_turn_with_a_handoff_until_every_one_is_capped()
 {
    let (shed, cooee, id) = chained();
    let first = format!("cooee-{id}");
    assert_an_error_is_no_limit(&shed, &cooee, &id, &first);
    shed.limit(&first);
    let asked = card(&cooee, &id, 1);
    assert_eq!(asked["tool"], "fake-acp capped until 3pm, continue on");
    let pair = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    let picks = [pair("other-acp", "other-acp"), pair("wait", "wait")];
    assert_eq!(options(&asked), picks);
    let tap = |item: &Value, option: &str| {
        let body = json!({ "item": item["id"], "option": option });
        cooee.post(&format!("/threads/{id}/approve"), &body)
    };
    let (status, body) = tap(&asked, "other-acp");
    assert_eq!(status, 200, "{body}");
    let items = replies(&cooee, &id, 3);
    let lane = lane(&shed, "other-acp");
    let go_on = "Go on with my last message: the agent before you hit its usage limit.";
    assert_the_pick_gets_a_handoff_to_go_on(&shed, &lane, go_on);
    let shown: Vec<&Value> = items
        .iter()
        .map(|i| i.get("text").unwrap_or(&i["chosen"]))
        .collect();
    assert_eq!(
        shown[2..6],
        [
            &json!("You've hit your limit · resets 3pm"),
            &json!("other-acp"),
            &json!("now on other-acp · later turns run there"),
            &json!(go_on),
        ]
    );
    assert_eq!(row(&cooee, &id)["agent"], "other-acp");
    assert_eq!(tap(&asked, "other-acp").0, 200);
    assert_eq!(shed.cli_log().matches("run --acp other-acp").count(), 1);

    shed.report(&lane, "rate-limited", Some("9am"));
    let out = card(&cooee, &id, 2);
    let all = "every agent in this chat's chain is capped; fake-acp resets first, 3pm";
    assert_eq!(out["tool"], format!("other-acp capped until 9am: {all}"));
    let picks = [
        pair("fake-acp", "fake-acp (capped until 3pm)"),
        pair("wait", "wait"),
    ];
    assert_eq!(options(&out), picks);
    assert_eq!(tap(&out, "wait").0, 200);
    let waited = card(&cooee, &id, 2);
    assert_eq!(
        (&waited["chosen"], &waited["line"]),
        (&json!("wait"), &json!("waiting for its reset"))
    );
    assert_eq!(row(&cooee, &id)["agent"], "other-acp");
}

fn assert_an_error_is_no_limit(shed: &FakeShed, cooee: &Cooee, id: &str, first: &str) {
    shed.report(first, "error", Some("boom"));
    let deadline = Instant::now() + Duration::from_secs(10);
    while row(cooee, id)["state"] != "error" {
        assert!(Instant::now() < deadline, "{}", row(cooee, id));
        thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(kinds(cooee, id), ["you", "agent"], "an error is no limit");
}

fn assert_the_pick_gets_a_handoff_to_go_on(shed: &FakeShed, lane: &str, go_on: &str) {
    let prompts = shed.sent(lane);
    assert_eq!(prompts.len(), 1, "{prompts:?}");
    let said = [
        "Be brief.",
        "Its first message: echo hello",
        "reply: You've hit your limit · resets 3pm",
        go_on,
    ];
    assert!(
        said.iter().all(|s| prompts[0].contains(s)),
        "{}",
        prompts[0]
    );
}
