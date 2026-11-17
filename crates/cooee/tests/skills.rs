//! A persona's skills (#218) end to end with the fake shed: proposed through cooee's own MCP server with the thread's
//! token, saved only once the owner approves the card, listed by description (never body) in the next session's brief,
//! read on demand by their own persona only, at most 20, and kept and edited over the native app's API. A thread's
//! "Save as skill" and "Make this a routine" (#495) each queue one marked message for its persona, refused with why
//! for a room, a Workflow thread, a thread with no persona and a persona at 20 skills.

mod support;

use std::{
    fs,
    io::Write as _,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, agent, scratch, version};

struct Setup {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    for n in ["Ada", "Bo"] {
        let toml = format!("name = \"{n}\"\nagent = \"fake-acp\"\nstyle = \"detailed\"\n");
        let home = agent(&dir, &n.to_lowercase(), &toml, "");
        fs::write(home.join("CHARACTER.md"), format!("I am {n}.")).unwrap();
    }
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    Setup { dir, shed, cooee }
}

impl Setup {
    fn call(&self, method: &str, path: &str, body: Option<&Value>) -> (u16, Value) {
        let (status, text) = self.cooee.request(method, path, body);
        (
            status,
            serde_json::from_str(&text).unwrap_or(Value::String(text)),
        )
    }

    fn start(&self, persona: &str) -> (String, String) {
        let random = format!("start-{}", self.shed.cli_log().len());
        let new =
            json!({ "agent": "other-acp", "profile": persona, "text": "hi", "random_id": random });
        let (status, body) = self.call("POST", "/threads", Some(&new));
        assert_eq!(status, 200, "{body}");
        let id = body["id"].as_str().unwrap().to_owned();
        let sent = self.shed.sent(&format!("cooee-{id}"));
        let prompt: Value = serde_json::from_str(sent[0].trim_start_matches('\x1e')).unwrap();
        (id, prompt["prompt"].as_str().unwrap().to_owned())
    }

    fn skill(&self, id: &str, args: &Value) -> (String, bool) {
        let file = self.shed.dir.join(format!("mcp-cooee-{id}.json"));
        let servers: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
        let cooee = &servers[0];
        let mut child = Command::new(cooee["command"].as_str().unwrap())
            .args(
                cooee["args"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|a| a.as_str().unwrap()),
            )
            .env("COOEE_MCP", cooee["env"][0]["value"].as_str().unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let call = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                           "params": { "name": "skill", "arguments": args } });
        writeln!(child.stdin.take().unwrap(), "{call}").unwrap();
        let out = child.wait_with_output().unwrap();
        let result = serde_json::from_slice::<Value>(&out.stdout).unwrap()["result"].clone();
        let text = result["content"][0]["text"].as_str().unwrap().to_owned();
        (text, result["isError"] == json!(true))
    }

    fn card(&self, id: &str) -> Value {
        let items = self.cooee.json(&format!("/threads/{id}/messages"));
        let mut cards = items["items"].as_array().unwrap().iter();
        cards
            .rfind(|i| i["form"] == "skill")
            .expect("a card")
            .clone()
    }

    fn answer(&self, id: &str, card: &Value, option: &str) -> Value {
        let tap = json!({ "item": card["id"], "option": option });
        let (status, body) = self.call("POST", &format!("/threads/{id}/approve"), Some(&tap));
        assert_eq!(status, 200, "{body}");
        body
    }

    fn teach(&self, persona: &str, action: &str, thread: &str) -> (u16, Value) {
        let act = json!({ "screen": format!("/screens/persona/{persona}"), "block": "actions",
                          "action": action, "values": { "thread": thread } });
        self.call("POST", "/screens/action", Some(&act))
    }

    fn marked(&self, id: &str, action: &str) -> Vec<Value> {
        let items = self.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
        let items = items.as_array().unwrap().iter();
        items
            .filter(|i| i["kind"] == "you" && i["routine"]["id"] == action)
            .cloned()
            .collect()
    }

    fn file(&self, persona: &str, name: &str) -> PathBuf {
        self.dir
            .join(format!("agents/{persona}/skills/{name}/SKILL.md"))
    }
}

#[test]
fn an_approved_skill_is_saved_and_briefed_by_description_and_a_refused_one_is_not() {
    let s = setup("skills-propose");
    let (id, prompt) = s.start("ada");
    assert_eq!(prompt, "I am Ada.\n\nhi", "no skills, no index");

    let plan = json!({ "name": "release", "description": "How I cut a release.",
                       "body": "1. Tag it.\n2. Write the notes." });
    let (said, failed) = s.skill(&id, &json!({ "propose": plan }));
    assert!(!failed && said.starts_with("Proposed."), "{said}");
    let card = s.card(&id);
    assert_eq!(
        card["detail"],
        "+ ---\n+ name: release\n+ description: How I cut a release.\n+ ---\n+ 1. Tag it.\n+ 2. Write the notes.\n"
    );
    assert!(
        !s.file("ada", "release").exists(),
        "unanswered: nothing saved"
    );

    let other = json!({ "name": "deploy", "description": "How I deploy.", "body": "Ship it." });
    assert!(!s.skill(&id, &json!({ "propose": other })).1);
    let declined = s.answer(&id, &s.card(&id), "decline");
    assert_eq!(declined["line"], "declined");
    assert!(!s.file("ada", "deploy").exists());

    let approved = s.answer(&id, &card, "approve");
    assert_eq!(approved["line"], "saved skill release");
    assert_eq!(
        fs::read_to_string(s.file("ada", "release")).unwrap(),
        "---\nname: release\ndescription: How I cut a release.\n---\n1. Tag it.\n2. Write the notes.\n"
    );
    let (next, brief) = s.start("ada");
    assert_eq!(
        brief,
        "I am Ada.\n\nYour skills (read one with cooee's skill tool):\n- release: How I cut a release.\n\nhi"
    );
    assert_eq!(s.start("ada").1, brief);

    assert_eq!(
        s.skill(&next, &json!({ "read": "release" })),
        ("1. Tag it.\n2. Write the notes.".to_owned(), false)
    );
    let (bo, _) = s.start("bo");
    let (refused, failed) = s.skill(&bo, &json!({ "read": "release" }));
    assert!(failed, "{refused}");
    assert_eq!(refused, "no skill release for bo");
    let (refused, failed) = s.skill(&bo, &json!({ "read": "../../ada/skills/release" }));
    assert!(
        failed && refused.contains("isn't a skill name"),
        "{refused}"
    );

    let edit = json!({ "name": "release", "description": "How I cut a release.",
                       "body": "1. Tag it.\n2. Write the notes.\n3. Post them.", "replaces": "release" });
    assert!(!s.skill(&next, &json!({ "propose": edit })).1);
    let card = s.card(&next);
    assert!(
        card["detail"]
            .as_str()
            .unwrap()
            .ends_with("  2. Write the notes.\n+ 3. Post them.\n"),
        "{card}"
    );
    assert!(
        card["tool"]
            .as_str()
            .unwrap()
            .starts_with("edit ada's skill release")
    );
    assert!(
        !fs::read_to_string(s.file("ada", "release"))
            .unwrap()
            .contains("Post")
    );
    s.answer(&next, &card, "approve");
    assert!(
        fs::read_to_string(s.file("ada", "release"))
            .unwrap()
            .contains("3. Post them.")
    );

    assert_eq!(s.call("DELETE", "/personas/ada/memory", None).0, 200);
    assert!(s.file("ada", "release").exists());
}

#[test]
fn the_app_keeps_skills_and_the_twenty_first_is_refused() {
    let s = setup("skills-api");
    let skill = |n: usize| json!({ "name": format!("s{n:02}"), "description": format!("Skill {n}."), "body": "Do it." });
    for n in 1..=20 {
        let (status, body) = s.call("POST", "/personas/ada/skills", Some(&skill(n)));
        assert_eq!(status, 200, "{body}");
    }
    let (status, body) = s.call("POST", "/personas/ada/skills", Some(&skill(21)));
    assert_eq!(
        (status, body),
        (400, json!("a persona keeps at most 20 skills"))
    );
    let (id, brief) = s.start("ada");
    assert_eq!(brief.matches("\n- s").count(), 20, "{brief}");
    let (said, failed) = s.skill(&id, &json!({ "propose": skill(21) }));
    assert!(failed, "{said}");
    assert_eq!(said, "a persona keeps at most 20 skills");

    let long = json!({ "name": "long", "description": "x".repeat(121), "body": "" });
    assert_eq!(s.call("POST", "/personas/bo/skills", Some(&long)).0, 400);
    assert_eq!(
        s.call("POST", "/personas/ada/skills", Some(&skill(1))).0,
        400
    );

    assert_eq!(s.cooee.json("/personas/ada/skills/s01")["body"], "Do it.");
    let renamed = json!({ "name": "first", "description": "The first.", "body": "Do it first." });
    let (status, body) = s.call("PUT", "/personas/ada/skills/s01", Some(&renamed));
    assert_eq!((status, &body["name"]), (200, &json!("first")), "{body}");
    assert!(!s.file("ada", "s01").exists());
    let list = s.cooee.json("/personas/ada/skills");
    let names: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 20);
    assert_eq!(names[0], "first");
    assert_eq!(s.call("DELETE", "/personas/ada/skills/first", None).0, 200);
    assert_eq!(s.call("GET", "/personas/ada/skills/first", None).0, 400);
    assert_eq!(s.cooee.json("/personas/bo/skills"), json!([]));
}

#[test]
fn each_thread_action_queues_one_marked_message_and_its_skill_saves_only_on_approve() {
    let s = setup("skills-teach");
    let screen = s.cooee.json("/screens/persona/ada");
    let blocks = screen["blocks"].as_array().unwrap();
    let actions = blocks.iter().find(|b| b["type"] == "actions").unwrap();
    assert_eq!(
        actions["rows"],
        json!([{ "label": "Save as skill", "action": "skill" },
               { "label": "Make this a routine", "action": "routine" }])
    );
    let (id, _) = s.start("ada");
    for (action, name, asks) in [
        (
            "skill",
            "Save as skill",
            "Draft a skill from the steps of this thread",
        ),
        (
            "routine",
            "Make this a routine",
            "Propose a routine that repeats",
        ),
    ] {
        let (status, queue) = s.teach("ada", action, &id);
        assert_eq!(status, 200, "{queue}");
        let queued = queue.as_array().unwrap().iter();
        let mut mine: Vec<Value> = queued
            .filter(|q| q["routine"]["id"] == action)
            .cloned()
            .collect();
        mine.extend(s.marked(&id, action));
        assert_eq!(mine.len(), 1, "{mine:?}");
        assert_eq!(mine[0]["routine"]["name"], name);
        assert!(
            mine[0]["text"].as_str().unwrap().starts_with(asks),
            "{mine:?}"
        );
    }
    let you = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let owner = you
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "you" && i["routine"].is_null());
    assert_eq!(owner.count(), 1, "only the owner's own hi is unmarked");

    let plan = json!({ "name": "triage", "description": "How I triage.", "body": "1. Read it." });
    assert!(!s.skill(&id, &json!({ "propose": plan })).1);
    assert!(
        !s.file("ada", "triage").exists(),
        "unanswered: nothing saved"
    );
    s.answer(&id, &s.card(&id), "approve");
    assert!(s.file("ada", "triage").exists());
}

#[test]
fn a_thread_action_is_refused_with_why() {
    let s = setup("skills-teach-refused");
    let room = json!({ "members": ["ada", "bo"], "random_id": "room" });
    let (status, room) = s.call("POST", "/rooms", Some(&room));
    assert_eq!(status, 200, "{room}");
    let room = room["id"].as_str().unwrap();
    assert_eq!(
        s.teach("ada", "skill", room),
        (400, json!("a room has no one persona to ask"))
    );

    let labels = [
        ("source", "drover"),
        ("group", "cooee"),
        ("item", "#33"),
        ("role", "impl"),
        ("round", "1"),
        ("url", "https://github.com/o/cooee/pull/60"),
    ];
    s.shed.lane("drover-cooee-33-impl", &labels);
    let lane = (0..250)
        .find_map(|_| {
            let rows = s.cooee.json("/threads");
            let row = rows
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["workflow"].is_object())
                .cloned();
            if row.is_none() {
                thread::sleep(Duration::from_millis(20));
            }
            row
        })
        .expect("a Workflow thread");
    assert_eq!(
        s.teach("ada", "routine", lane["id"].as_str().unwrap()),
        (400, json!("a Workflow thread is the Workflow's to keep"))
    );

    let plain = json!({ "agent": "fake-acp", "folder": s.dir.join("root/project"), "text": "hi", "random_id": "plain" });
    let (status, plain) = s.call("POST", "/threads", Some(&plain));
    assert_eq!(status, 200, "{plain}");
    assert_eq!(
        s.teach("ada", "skill", plain["id"].as_str().unwrap()),
        (400, json!("the thread has no persona"))
    );

    let (id, _) = s.start("ada");
    for n in 1..=20 {
        let skill =
            json!({ "name": format!("s{n:02}"), "description": "A skill.", "body": "Do it." });
        assert_eq!(s.call("POST", "/personas/ada/skills", Some(&skill)).0, 200);
    }
    assert_eq!(
        s.teach("ada", "skill", &id),
        (400, json!("Ada keeps 20 skills already"))
    );
    assert_eq!(s.marked(&id, "skill"), Vec::<Value>::new());
    assert_eq!(s.teach("ada", "routine", &id).0, 200);
}
