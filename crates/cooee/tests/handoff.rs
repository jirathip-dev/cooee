//! Handoff asks (#164, W5) end to end against the fake shed: the agent calls cooee's `handoff` tool in its turn and
//! the turn stops with a card; while it waits nothing the owner sends or queued reaches the agent; the owner's hand
//! back resumes it with their note.

mod support;

use std::{io::Write as _, thread, time::Duration};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, agent, scratch, version};

struct Setup {
    shed: FakeShed,
    cooee: Cooee,
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    agent(&dir, "ada", "name = \"Ada\"\nagent = \"fake-acp\"\n", "");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    Setup { shed, cooee }
}

fn eventually(mut f: impl FnMut() -> bool) {
    for _ in 0..200 {
        if f() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("never happened");
}

impl Setup {
    fn start(&self, text: &str) -> String {
        let new =
            json!({ "agent": "fake-acp", "profile": "ada", "text": text, "random_id": "first" });
        let (status, body) = self.cooee.post("/threads", &new);
        assert_eq!(status, 200, "{body}");
        let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        eventually(|| self.replies(&id).len() == 1);
        id
    }

    fn slice(&self, id: &str) -> Value {
        self.cooee.json(&format!("/threads/{id}/messages"))
    }

    fn of_kind(&self, id: &str, kind: &str) -> Vec<Value> {
        let all = self.slice(id)["items"].as_array().unwrap().clone();
        all.into_iter().filter(|i| i["kind"] == kind).collect()
    }

    fn replies(&self, id: &str) -> Vec<Value> {
        self.of_kind(id, "agent")
    }

    fn card(&self, id: &str) -> (Value, Value, Value) {
        let cards = self.of_kind(id, "handoff");
        assert_eq!(cards.len(), 1, "{cards:?}");
        let c = &cards[0];
        (c["text"].clone(), c["outcome"].clone(), c["note"].clone())
    }

    fn post(&self, id: &str, verb: &str, text: &str, random_id: &str) -> (u16, String) {
        let body = json!({ "text": text, "random_id": random_id });
        self.cooee.post(&format!("/threads/{id}/{verb}"), &body)
    }

    fn sent(&self, id: &str) -> Vec<String> {
        let framed = self.shed.sent(&format!("cooee-{id}"));
        let prompt = |f: &String| -> String {
            let v: Value = serde_json::from_str(f.trim_start_matches('\u{1e}')).unwrap();
            v["prompt"].as_str().unwrap_or_default().to_owned()
        };
        framed.iter().map(prompt).collect()
    }

    fn handoff(&self, id: &str, what: &str) -> String {
        let file = self.shed.dir.join(format!("mcp-cooee-{id}.json"));
        let servers: Value = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
        let cooee = &servers[0];
        let mut child = std::process::Command::new(cooee["command"].as_str().unwrap())
            .args(
                cooee["args"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|a| a.as_str().unwrap()),
            )
            .env("COOEE_MCP", cooee["env"][0]["value"].as_str().unwrap())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let call = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                           "params": { "name": "handoff", "arguments": { "what": what } } });
        writeln!(child.stdin.take().unwrap(), "{call}").unwrap();
        let out = child.wait_with_output().unwrap();
        let result = serde_json::from_slice::<Value>(&out.stdout).unwrap()["result"].clone();
        assert_eq!(result["isError"], json!(false), "{result}");
        result["content"][0]["text"].as_str().unwrap().to_owned()
    }
}

#[test]
fn the_turn_stops_with_a_handoff_and_hand_back_resumes_it_with_the_note_only() {
    let s = setup("handoff-turn");
    let id = s.start("hand off: Sign in to linkedin.com");
    let said = s.replies(&id)[0]["text"].as_str().unwrap().to_owned();
    assert!(said.starts_with("Handed off: end your turn now."), "{said}");
    let waiting = (
        json!("Sign in to linkedin.com"),
        json!("waiting"),
        Value::Null,
    );
    assert_eq!(s.card(&id), waiting);

    for verb in ["send", "queue"] {
        let (status, why) = s.post(&id, verb, "my code is 424242", verb);
        assert!(
            status != 200 && why.contains("HANDOFF"),
            "{verb}: {status} {why}"
        );
    }
    assert_eq!(s.slice(&id)["queue"], json!([]));

    let (status, body) = s.post(&id, "handback", "Signed in, go on", "back");
    assert_eq!(status, 200, "{body}");
    eventually(|| s.replies(&id).len() == 2);
    let back = "[The owner handed back: Sign in to linkedin.com; their note]:\n\nSigned in, go on";
    assert_eq!(s.sent(&id).last().map(String::as_str), Some(back));
    let closed = (
        json!("Sign in to linkedin.com"),
        json!("handed_back"),
        json!("Signed in, go on"),
    );
    assert_eq!(s.card(&id), closed);
    assert!(
        s.sent(&id).iter().all(|p| !p.contains("424242")),
        "{:?}",
        s.sent(&id)
    );

    let again = s.post(&id, "handback", "Signed in, go on", "back");
    let first: Value = serde_json::from_str(&body).unwrap();
    let repeat: Value = serde_json::from_str(&again.1).unwrap();
    assert_eq!((again.0, &repeat["id"]), (200, &first["id"]));
    let (status, why) = s.post(&id, "handback", "again", "back-2");
    assert!(
        status != 200 && why.contains("NO_HANDOFF"),
        "{status} {why}"
    );
    assert_eq!(s.post(&id, "send", "thanks", "after").0, 200);
}

#[test]
fn a_message_queued_before_the_handoff_waits_until_the_owner_hands_back() {
    let s = setup("handoff-queue");
    let id = s.start("hi");
    assert_eq!(s.post(&id, "send", "hang on", "hang").0, 200);
    eventually(|| s.slice(&id)["turn"]["state"] == "working");
    assert_eq!(s.post(&id, "queue", "queued earlier", "q").0, 200);
    assert!(
        s.handoff(&id, "Run gh auth login")
            .starts_with("Handed off")
    );
    let (status, why) = s.post(&id, "steer", "q", "q");
    assert!(status != 200 && why.contains("HANDOFF"), "{status} {why}");

    assert_eq!(
        s.cooee.post(&format!("/threads/{id}/cancel"), &json!({})).0,
        200
    );
    eventually(|| s.replies(&id).len() == 2);
    assert_eq!(s.slice(&id)["queue"].as_array().unwrap().len(), 1);
    assert!(s.sent(&id).iter().all(|p| !p.contains("queued earlier")));

    assert_eq!(s.post(&id, "handback", "", "back").0, 200);
    eventually(|| s.sent(&id).iter().any(|p| p == "queued earlier"));
    let sent = s.sent(&id);
    let note = "[The owner handed back: Run gh auth login; their note]:\n\nDone.";
    assert_eq!(sent[sent.len() - 2..], [note, "queued earlier"]);
}
