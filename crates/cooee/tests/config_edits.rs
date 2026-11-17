//! Config saves are surgical and never overwrite a hand edit (#360), over the API with the fake shed: an `agent.toml`
//! with comments changes in the one line of the key saved, a save naming the version read before a hand edit is a
//! 409 that leaves the edit, a hand-edited `INSTRUCTIONS.md` reaches the next chat with no restart, and a new note
//! adds one line to `memory.md`.

mod support;

use std::{fs, path::PathBuf};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, agent, scratch, version};

struct Setup {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
}

const TOML: &str = "# The house coder.\nname = \"Coder\" # shown on Home\n\n# Which ACP agent runs it.\n\
                    agent = \"fake-acp\"\nmodel = \"small\"\n\n[env]\n# review or write\nMODE = \"review\"\n";

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    agent(&dir, "coder", TOML, "Follow the house rules.\n");
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

    fn file(&self, name: &str) -> PathBuf {
        self.dir.join("agents/coder").join(name)
    }

    fn prompt(&self) -> String {
        let random = format!("start-{}", self.shed.cli_log().len());
        let new =
            json!({ "agent": "other-acp", "profile": "coder", "text": "hi", "random_id": random });
        let (status, body) = self.call("POST", "/threads", Some(&new));
        assert_eq!(status, 200, "{body}");
        let id = body["id"].as_str().unwrap();
        let sent = self.shed.sent(&format!("cooee-{id}"));
        let prompt: Value = serde_json::from_str(sent[0].trim_start_matches('\x1e')).unwrap();
        prompt["prompt"].as_str().unwrap().to_owned()
    }
}

fn changed(old: &str, new: &str) -> Vec<String> {
    let (a, b): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
    let gone = a.iter().filter(|l| !b.contains(l)).map(|l| format!("-{l}"));
    let came = b.iter().filter(|l| !a.contains(l)).map(|l| format!("+{l}"));
    gone.chain(came).collect()
}

#[test]
fn saving_the_model_changes_only_its_line_and_keeps_the_comments() {
    let s = setup("edits-model");
    let mut p = s.cooee.json("/personas/coder");
    assert!(p["version"].as_str().is_some_and(|v| !v.is_empty()), "{p}");
    p["model"] = json!("large");
    let (status, saved) = s.call("PUT", "/personas/coder", Some(&p));
    assert_eq!(status, 200, "{saved}");
    let now = fs::read_to_string(s.file("agent.toml")).unwrap();
    assert_eq!(
        changed(TOML, &now),
        ["-model = \"small\"", "+model = \"large\""]
    );
    assert_eq!(now, TOML.replace("small", "large"));
    assert_eq!(
        fs::read_to_string(s.file("INSTRUCTIONS.md")).unwrap(),
        "Follow the house rules.\n"
    );
    assert_ne!(saved["version"], p["version"]);
}

#[test]
fn a_save_after_a_hand_edit_is_refused_and_the_edit_stays() {
    let s = setup("edits-stale");
    let mut p = s.cooee.json("/personas/coder");
    let edited = TOML.replace("small", "medium");
    fs::write(s.file("agent.toml"), &edited).unwrap();
    p["model"] = json!("large");
    let (status, body) = s.call("PUT", "/personas/coder", Some(&p));
    assert_eq!(status, 409, "{body}");
    assert!(
        body.as_str().unwrap().contains("changed on disk, reload"),
        "{body}"
    );
    assert_eq!(fs::read_to_string(s.file("agent.toml")).unwrap(), edited);

    p["instructions"] = json!("Mine.");
    let screen = json!({ "screen": "/screens/persona/coder", "action": "save", "values": p });
    let (status, body) = s.call("POST", "/screens/action", Some(&screen));
    assert_eq!(status, 409, "{body}");
    assert_eq!(
        fs::read_to_string(s.file("INSTRUCTIONS.md")).unwrap(),
        "Follow the house rules.\n"
    );

    let fresh = s.cooee.json("/personas/coder");
    assert_eq!(fresh["model"], "medium");
    p["version"] = fresh["version"].clone();
    assert_eq!(s.call("PUT", "/personas/coder", Some(&p)).0, 200);
    assert_eq!(
        fs::read_to_string(s.file("INSTRUCTIONS.md")).unwrap(),
        "Mine."
    );
}

#[test]
fn a_hand_edited_instructions_file_reaches_the_next_chat() {
    let s = setup("edits-instructions");
    assert!(s.prompt().contains("Follow the house rules."));
    fs::write(s.file("INSTRUCTIONS.md"), "Answer in haiku.\n").unwrap();
    let prompt = s.prompt();
    assert!(prompt.contains("Answer in haiku."), "{prompt}");
    assert!(!prompt.contains("house rules"), "{prompt}");
}

#[test]
fn a_new_note_adds_exactly_one_line_to_memory() {
    let s = setup("edits-memory");
    let old = "# Coder's notes\n\n- [ab] Tea, no sugar. <!-- by:owner at:1 -->\nKept by hand.\n\
               - [cd] Lunch at one.\n";
    fs::write(s.file("memory.md"), old).unwrap();
    let (status, body) = s.call(
        "POST",
        "/personas/coder/memory",
        Some(&json!({ "text": "Call me Sam." })),
    );
    assert_eq!(status, 200, "{body}");
    let id = body["id"].as_str().unwrap();
    let now = fs::read_to_string(s.file("memory.md")).unwrap();
    assert!(now.starts_with(old), "{now}");
    let added = changed(old, &now);
    assert_eq!(added.len(), 1, "{added:?}");
    assert!(
        added[0].starts_with(&format!("+- [{id}] Call me Sam. <!-- by:owner at:")),
        "{added:?}"
    );
}
