//! A persona's skills (#218) end to end with the fake shed: proposed through cooee's own MCP server with the thread's
//! token, saved only once the owner approves the card, listed by description (never body) in the next session's brief,
//! read on demand by their own persona only, at most 20, and kept and edited over the native app's API.

mod support;

use std::{
    fs,
    io::Write as _,
    path::PathBuf,
    process::{Command, Stdio},
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

struct Setup {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    fs::create_dir_all(dir.join("personas")).unwrap();
    for n in ["Ada", "Bo"] {
        let file = dir.join(format!("personas/{}.toml", n.to_lowercase()));
        fs::write(
            file,
            format!("name = \"{n}\"\nagent = \"fake-acp\"\ncharacter = \"I am {n}.\"\nstyle = \"detailed\"\n"),
        )
        .unwrap();
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

    /// A new thread as `persona`: its id and the first prompt its agent got.
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

    /// `skill` called by thread `id`'s agent through cooee's own MCP server: the tool's text and whether it failed.
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

    /// Thread `id`'s newest skill card.
    fn card(&self, id: &str) -> Value {
        let items = self.cooee.json(&format!("/threads/{id}/messages"));
        let mut cards = items["items"].as_array().unwrap().iter();
        cards
            .rfind(|i| i["form"] == "skill")
            .expect("a card")
            .clone()
    }

    /// The owner taps `option` on card `card` of thread `id`.
    fn answer(&self, id: &str, card: &Value, option: &str) -> Value {
        let tap = json!({ "item": card["id"], "option": option });
        let (status, body) = self.call("POST", &format!("/threads/{id}/approve"), Some(&tap));
        assert_eq!(status, 200, "{body}");
        body
    }

    fn file(&self, persona: &str, name: &str) -> PathBuf {
        self.dir
            .join(format!("data/personas/{persona}/skills/{name}/SKILL.md"))
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

    // Refused: nothing saved.
    let other = json!({ "name": "deploy", "description": "How I deploy.", "body": "Ship it." });
    assert!(!s.skill(&id, &json!({ "propose": other })).1);
    let declined = s.answer(&id, &s.card(&id), "decline");
    assert_eq!(declined["line"], "declined");
    assert!(!s.file("ada", "deploy").exists());

    // Approved: saved as the file, listed by description in the next session's brief, never its body.
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
    // Byte-identical while the skills are unchanged.
    assert_eq!(s.start("ada").1, brief);

    // Its body on demand, to its own persona only.
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

    // An edit shows as a diff, and is saved only once approved.
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

    // Clearing its notes leaves its skills.
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

    // Too long a description, or a name taken, is refused.
    let long = json!({ "name": "long", "description": "x".repeat(121), "body": "" });
    assert_eq!(s.call("POST", "/personas/bo/skills", Some(&long)).0, 400);
    assert_eq!(
        s.call("POST", "/personas/ada/skills", Some(&skill(1))).0,
        400
    );

    // Read, edit (renamed), delete.
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
