//! Rooms (#154) end to end against the fake shed: a room of personas is made, members are added and taken out (a
//! taken-out member's session is closed), each member's own session starts on its first turn and is given the
//! others' final replies, attributed, and nothing else (G3); at most 5 members (G6); the conductor joins, a lane
//! thread doesn't (G5).

mod support;

use std::{fs, path::Path, thread, time::Duration};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

/// cooee with personas Ada, Bo, Cy, Di and Ed, and the conductor profile (read as a persona).
fn setup(name: &str) -> (std::path::PathBuf, FakeShed, Cooee) {
    let dir = scratch(name);
    fs::create_dir_all(dir.join("personas")).unwrap();
    for n in ["Ada", "Bo", "Cy", "Di", "Ed"] {
        let file = dir.join(format!("personas/{}.toml", n.to_lowercase()));
        fs::write(file, format!("name = \"{n}\"\nagent = \"fake-acp\"\n")).unwrap();
    }
    fs::create_dir_all(dir.join("profiles")).unwrap();
    fs::write(
        dir.join("profiles/conductor.toml"),
        "name = \"conductor\"\nlabel = \"Conductor\"\nagent = \"fake-acp\"\n",
    )
    .unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    (dir, shed, cooee)
}

fn post(cooee: &Cooee, path: &str, body: &Value) -> (u16, Value) {
    let (status, text) = cooee.post(path, body);
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

fn room(cooee: &Cooee, members: &[&str]) -> String {
    let new = json!({ "members": members, "random_id": format!("room-{members:?}") });
    let (status, body) = post(cooee, "/rooms", &new);
    assert_eq!(status, 200, "{body}");
    body["id"].as_str().unwrap().to_owned()
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

/// The room's items once `n` member replies are in.
fn replies(cooee: &Cooee, id: &str, n: usize) -> Vec<Value> {
    let mut items = Vec::new();
    eventually(|| {
        let all = cooee.json(&format!("/threads/{id}/messages"));
        items.clone_from(all["items"].as_array().unwrap());
        items.iter().filter(|i| i["kind"] == "agent").count() == n
    });
    items
}

/// The `shed run` line of member `persona`'s session, if it started.
fn run_line(dir: &Path, persona: &str) -> Option<String> {
    let log = fs::read_to_string(dir.join("shed/cli.log")).unwrap_or_default();
    let member = format!("cooee.member={persona}");
    log.lines().find(|l| l.contains(&member)).map(str::to_owned)
}

/// The prompts member `persona`'s session was given, in order.
fn prompts(dir: &Path, shed: &FakeShed, persona: &str) -> Vec<String> {
    let line = run_line(dir, persona).expect("its session started");
    let terminal = line
        .split([' ', '='])
        .find(|w| w.starts_with("cooee-t"))
        .unwrap();
    let sent = shed.sent(terminal);
    sent.iter()
        .map(|s| {
            let p: Value = serde_json::from_str(s.trim_start_matches('\x1e')).unwrap();
            p["prompt"].as_str().unwrap().to_owned()
        })
        .collect()
}

fn send(cooee: &Cooee, id: &str, text: &str) {
    let body = json!({ "text": text, "random_id": text });
    let (status, item) = post(cooee, &format!("/threads/{id}/send"), &body);
    assert_eq!((status, &item["speaker"]), (200, &json!("owner")), "{item}");
}

#[test]
fn each_member_takes_a_turn_in_its_own_session_and_hears_the_others_attributed() {
    let (dir, shed, cooee) = setup("room-talk");
    let id = room(&cooee, &["ada", "bo"]);
    let rows = cooee.json("/threads");
    assert_eq!(rows[0]["members"], json!(["ada", "bo"]), "{rows}");
    assert!(run_line(&dir, "ada").is_none(), "no session before a turn");

    send(&cooee, &id, "hi all");
    let items = replies(&cooee, &id, 2);
    let said: Vec<(&Value, &Value)> = items.iter().map(|i| (&i["speaker"], &i["text"])).collect();
    assert_eq!(
        said,
        [
            (&json!("owner"), &json!("hi all")),
            (&json!("ada"), &json!("Hello")),
            (&json!("bo"), &json!("Hello"))
        ]
    );
    let line = run_line(&dir, "ada").unwrap();
    assert!(line.contains(&format!("cooee.thread={id}")), "{line}");
    assert!(!prompts(&dir, &shed, "ada")[0].contains("[Bo]"));
    assert!(
        prompts(&dir, &shed, "bo")[0].ends_with("[Owner]: hi all\n\n[Ada]: Hello"),
        "{:?}",
        prompts(&dir, &shed, "bo")
    );

    // The next turn gives each only what it hasn't had, never its own.
    send(&cooee, &id, "and again");
    replies(&cooee, &id, 4);
    let ada = prompts(&dir, &shed, "ada");
    assert_eq!(ada[1], "[Bo]: Hello\n\n[Owner]: and again", "{ada:?}");
    // The members' sessions are the room's, never listed apart.
    assert_eq!(cooee.json("/threads").as_array().unwrap().len(), 1);
}

#[test]
fn members_are_given_final_replies_only() {
    let (dir, shed, cooee) = setup("room-final");
    let id = room(&cooee, &["ada", "bo"]);
    send(&cooee, &id, "use a tool");
    let items = replies(&cooee, &id, 2);
    assert!(items.iter().all(|i| i["kind"] != "tool"), "{items:?}");
    let bo = prompts(&dir, &shed, "bo").remove(0);
    assert!(bo.ends_with("[Ada]: Read it."), "{bo}");
    // Ada's tool ran in her own session; Bo's was given her reply alone.
    assert!(!bo.contains("Read src/lib.rs"), "{bo}");
}

#[test]
fn members_join_and_leave_up_to_five_the_conductor_but_never_a_lane() {
    let (dir, shed, cooee) = setup("room-members");
    let six = json!({ "members": ["ada", "bo", "cy", "di", "ed", "conductor"], "random_id": "r6" });
    let (status, why) = post(&cooee, "/rooms", &six);
    assert_eq!(status, 400, "{why}");
    assert!(why.to_string().contains("2 to 5 members"), "{why}");

    let id = room(&cooee, &["ada", "bo"]);
    let path = format!("/threads/{id}/members");
    let (status, members) = post(&cooee, &path, &json!({ "persona": "conductor" }));
    assert_eq!((status, members), (200, json!(["ada", "bo", "conductor"])));
    assert_eq!(post(&cooee, &path, &json!({ "persona": "cy" })).0, 200);
    // Twice is once.
    let (_, members) = post(&cooee, &path, &json!({ "persona": "cy" }));
    assert_eq!(members, json!(["ada", "bo", "conductor", "cy"]));
    assert_eq!(post(&cooee, &path, &json!({ "persona": "di" })).0, 200);
    let (status, why) = post(&cooee, &path, &json!({ "persona": "ed" }));
    assert!(
        status == 400 && why.to_string().contains("at most 5"),
        "{why}"
    );
    assert_eq!(post(&cooee, &path, &json!({ "persona": "nobody" })).0, 400);

    // A lane thread can't join.
    let labels = [("source", "drover"), ("group", "cooee"), ("item", "#33")];
    shed.lane("drover-cooee-33-impl", &labels);
    let lane = "w-drover-cooee-33-impl";
    eventually(|| cooee.json("/threads").to_string().contains(lane));
    let (status, why) = post(&cooee, &path, &json!({ "persona": "ed", "remove": true }));
    assert_eq!(status, 200, "{why}");
    let (_, why) = post(&cooee, &path, &json!({ "persona": "di", "remove": true }));
    assert_eq!(why, json!(["ada", "bo", "conductor", "cy"]));
    let (status, why) = post(&cooee, &path, &json!({ "persona": lane }));
    assert!(
        status == 400 && why.to_string().contains("lane thread"),
        "{why}"
    );
    let new = json!({ "members": ["ada", lane], "random_id": "lane" });
    assert_eq!(post(&cooee, "/rooms", &new).0, 400);

    // Taken out: its session is closed, and it takes no more turns.
    send(&cooee, &id, "hi all");
    replies(&cooee, &id, 4);
    let bo = run_line(&dir, "bo").unwrap();
    let bo = bo
        .split([' ', '='])
        .find(|w| w.starts_with("cooee-t"))
        .unwrap()
        .to_owned();
    let (_, members) = post(&cooee, &path, &json!({ "persona": "bo", "remove": true }));
    assert_eq!(members, json!(["ada", "conductor", "cy"]));
    eventually(|| !shed.calls("kill", &bo).is_empty());
    send(&cooee, &id, "once more");
    replies(&cooee, &id, 7);
    assert_eq!(prompts(&dir, &shed, "bo").len(), 1);
}
