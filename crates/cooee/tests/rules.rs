//! A persona's approval rules (#503, #535): cooee answers each harness request from them, deny over ask over allow,
//! and only an unmatched one reaches the owner as the three-button card; Settings lists, adds and removes them.

mod support;

use std::{fs, path::PathBuf, thread, time::Duration};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, agent, scratch, version};

struct Setup {
    dir: PathBuf,
    folder: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
    thread: Option<String>,
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let folder = dir.join("root/project");
    fs::create_dir_all(&folder).unwrap();
    let toml = format!(
        "name = \"Scout\"\nagent = \"fake-acp\"\nfolder = {:?}\n",
        folder.display().to_string()
    );
    agent(&dir, "scout", &toml, "");
    let folder = folder.canonicalize().unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    Setup {
        dir,
        folder,
        shed,
        cooee,
        thread: None,
    }
}

impl Setup {
    fn items(&self) -> Vec<Value> {
        let id = self.thread.as_deref().unwrap();
        let items = self.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
        items.as_array().unwrap().clone()
    }

    fn until<T>(&self, mut f: impl FnMut(&[Value]) -> Option<T>) -> T {
        for _ in 0..500 {
            if let Some(t) = f(&self.items()) {
                return t;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("never came: {:?}", self.items())
    }

    fn of<'a>(items: &'a [Value], kind: &str) -> Vec<&'a Value> {
        items.iter().filter(|i| i["kind"] == kind).collect()
    }

    fn run(&mut self, command: &str) -> Value {
        let replies = self
            .thread
            .as_ref()
            .map_or(0, |_| Self::of(&self.items(), "agent").len());
        let text = format!("Run `{command}`.");
        let random_id = format!("r-{command}");
        let (status, body) = if let Some(id) = &self.thread {
            let send = json!({ "text": text, "random_id": random_id });
            self.cooee.post(&format!("/threads/{id}/send"), &send)
        } else {
            let new = json!({ "agent": "fake-acp", "profile": "scout", "folder": self.folder,
                              "text": text, "random_id": random_id });
            let (status, body) = self.cooee.post("/threads", &new);
            let id = serde_json::from_str::<Value>(&body).unwrap()["id"].clone();
            self.thread = id.as_str().map(str::to_owned);
            (status, body)
        };
        assert_eq!(status, 200, "{body}");
        self.until(|items| {
            let open = Self::of(items, "approval")
                .into_iter()
                .find(|c| c["chosen"].is_null() && c["tapped"].is_null());
            let reply = Self::of(items, "agent").get(replies).copied();
            open.or(reply).cloned()
        })
    }

    fn tap(&self, card: &Value, option: &str) -> Value {
        let id = self.thread.as_deref().unwrap();
        let replies = Self::of(&self.items(), "agent").len();
        let tap = json!({ "item": card["id"], "option": option });
        let (status, body) = self.cooee.post(&format!("/threads/{id}/approve"), &tap);
        assert_eq!(status, 200, "{body}");
        self.until(|items| Self::of(items, "agent").get(replies).copied().cloned())
    }

    fn rules(&self) -> Vec<Value> {
        let rules = self.cooee.json("/personas/scout/rules");
        rules.as_array().unwrap().clone()
    }

    fn rule(&self, effect: &str, pattern: &str) -> Value {
        let rule = json!({ "effect": effect, "kind": "run", "pattern": pattern });
        let (status, body) = self.cooee.post("/personas/scout/rules", &rule);
        assert_eq!(status, 200, "{body}");
        serde_json::from_str(&body).unwrap()
    }

    fn answers(&self) -> Vec<Value> {
        let terminal = format!("cooee-{}", self.thread.as_deref().unwrap());
        let sent = self.shed.sent(&terminal).into_iter();
        let sent =
            sent.filter_map(|s| serde_json::from_str::<Value>(s.trim_start_matches('\x1e')).ok());
        sent.filter(|v| v.get("approve").is_some()).collect()
    }

    fn last_row(&self) -> Value {
        let items = self.items();
        items.iter().rfind(|i| i["kind"] == "tool").unwrap().clone()
    }
}

#[test]
fn always_allow_saves_a_rule_the_next_call_shows_no_card_and_removing_it_brings_the_card_back() {
    let mut s = setup("rules-always");
    let card = s.run("cargo test -p export");
    assert_eq!(
        (&card["kind"], &card["action"], &card["tool"]),
        (
            &json!("approval"),
            &json!("run"),
            &json!("cargo test -p export")
        )
    );
    let options: Vec<(&Value, &Value)> = card["options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| (&o["id"], &o["name"]))
        .collect();
    let words = format!("running cargo test … in {}", s.folder.display());
    assert_eq!(
        options,
        [
            (&json!("allow"), &json!("Allow")),
            (&json!("always"), &json!(format!("Always allow {words}"))),
            (&json!("deny"), &json!("Deny"))
        ]
    );
    assert_eq!(s.tap(&card, "always")["text"], "Ran it.");
    let request = card["request"]["id"].clone();
    assert_eq!(
        s.answers(),
        [json!({ "approve": "allow", "request": request })],
        "a one-time allow, never the harness's always"
    );
    let saved = s.rules();
    let ours: Vec<&Value> = saved.iter().filter(|r| r["by"] == "card").collect();
    assert_eq!(ours.len(), 1, "{saved:?}");
    assert_eq!(ours[0]["words"], format!("Allow {words}"));

    let reply = s.run("cargo test -p cli");
    assert_eq!(reply["text"], "Ran it.", "no card: {reply}");
    assert_eq!(Setup::of(&s.items(), "approval").len(), 1);
    assert_eq!(s.last_row()["ruled"], format!("Allow {words}"));
    assert_eq!(s.answers().len(), 2);

    let gone = format!("/personas/scout/rules/{}", ours[0]["id"].as_str().unwrap());
    assert_eq!(s.cooee.request("DELETE", &gone, None).0, 200);
    let card = s.run("cargo test -p web");
    assert_eq!(
        (&card["kind"], &card["tool"]),
        (&json!("approval"), &json!("cargo test -p web"))
    );
    assert_eq!(s.tap(&card, "deny")["text"], "Denied.");
    let last = s.answers().pop().unwrap();
    assert_eq!(
        last,
        json!({ "approve": "reject", "request": card["request"]["id"] })
    );
    fs::remove_dir_all(&s.dir).ok();
}

#[test]
fn a_deny_rule_beats_a_matching_allow_and_answers_with_no_card() {
    let mut s = setup("rules-deny");
    s.rule("allow", "cargo *");
    s.rule("deny", "cargo publish*");
    assert_eq!(s.run("cargo build --release")["text"], "Ran it.");
    let reply = s.run("cargo publish --dry-run");
    assert_eq!(reply["text"], "Denied.", "{reply}");
    assert_eq!(Setup::of(&s.items(), "approval").len(), 0);
    assert_eq!(s.last_row()["ruled"], "Deny running cargo publish*");
    let answers: Vec<Value> = s
        .answers()
        .into_iter()
        .map(|a| a["approve"].clone())
        .collect();
    assert_eq!(answers, [json!("allow"), json!("reject")]);
    fs::remove_dir_all(&s.dir).ok();
}

#[test]
fn a_persona_with_no_rules_file_has_two_removable_starter_rules_and_its_agent_changes_no_rule() {
    let s = setup("rules-starter");
    let file = s.dir.join("agents/scout/rules.toml");
    assert!(!file.exists());
    let words: Vec<Value> = s.rules().iter().map(|r| r["words"].clone()).collect();
    let folder = s.folder.display();
    assert_eq!(
        words,
        [
            json!(format!("Allow reading any file in {folder}")),
            json!(format!("Allow searching any file in {folder}"))
        ]
    );
    let search = s.rules()[1]["id"].as_str().unwrap().to_owned();
    assert_eq!(
        s.cooee
            .request("DELETE", &format!("/personas/scout/rules/{search}"), None)
            .0,
        200
    );
    let left: Vec<Value> = s.rules().iter().map(|r| r["words"].clone()).collect();
    assert_eq!(left, [words[0].clone()]);
    assert_eq!(
        s.cooee
            .request("DELETE", &format!("/personas/scout/rules/{search}"), None)
            .0,
        400,
        "gone once"
    );
    s.rule("allow", "make *");
    let bad = json!({ "effect": "maybe", "kind": "run", "pattern": "x" });
    assert_eq!(s.cooee.post("/personas/scout/rules", &bad).0, 400);

    let home = s.dir.join("agents/scout");
    let files = || {
        let mut all: Vec<(String, Vec<u8>)> = fs::read_dir(&home)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name() != "agent.toml")
            .map(|e| {
                (
                    e.file_name().to_string_lossy().into_owned(),
                    fs::read(e.path()).unwrap(),
                )
            })
            .collect();
        all.sort();
        all
    };
    let (before, listed) = (files(), s.rules());
    let mut persona = s.cooee.json("/personas/scout");
    persona.as_object_mut().unwrap().remove("version");
    persona["agent"] = json!("other-acp");
    let (status, body) = s.cooee.request("PUT", "/personas/scout", Some(&persona));
    assert_eq!(status, 200, "{body}");
    assert_eq!(s.cooee.json("/personas/scout")["agent"], "other-acp");
    let after = files();
    assert!(before.iter().all(|f| after.contains(f)), "{after:?}");
    assert_eq!(s.rules(), listed);
    fs::remove_dir_all(&s.dir).ok();
}

#[test]
fn a_rule_added_in_settings_answers_the_next_request_and_removing_it_brings_the_card_back() {
    let mut s = setup("rules-settings");
    assert_eq!(s.run("make lint")["kind"], "approval");
    let first = Setup::of(&s.items(), "approval")[0].clone();
    s.tap(&first, "allow");
    let rule = s.rule("allow", "make *");
    assert_eq!(
        (&rule["by"], &rule["words"]),
        (&json!("settings"), &json!("Allow running make …"))
    );
    assert_eq!(s.run("make test")["text"], "Ran it.");
    assert_eq!(Setup::of(&s.items(), "approval").len(), 1);
    let gone = format!("/personas/scout/rules/{}", rule["id"].as_str().unwrap());
    assert_eq!(s.cooee.request("DELETE", &gone, None).0, 200);
    let card = s.run("make docs");
    assert_eq!(
        (&card["kind"], &card["tool"]),
        (&json!("approval"), &json!("make docs"))
    );
    fs::remove_dir_all(&s.dir).ok();
}
