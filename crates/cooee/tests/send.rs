//! `cooee send` (#82) against `cooee serve` and the fake agent: each exit code from its case, and `--json`'s schema
//! as docs/scripting.md gives it.

mod support;

use std::{fs, path::PathBuf, process::Output};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, is_item, scratch, version};

struct Setup {
    dir: PathBuf,
    _shed: FakeShed,
    cooee: Cooee,
    config: PathBuf,
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let port = cooee.addr.rsplit(':').next().unwrap();
    let toml = fs::read_to_string(&cooee.config).unwrap();
    let config = dir.join("cli.toml");
    fs::write(&config, toml.replace("port = 0", &format!("port = {port}"))).unwrap();
    Setup {
        dir,
        _shed: shed,
        cooee,
        config,
    }
}

impl Setup {
    fn thread(&self) -> String {
        let folder = self.dir.join("root/project");
        let new =
            json!({ "agent": "fake-acp", "folder": folder, "text": "hi", "random_id": "start" });
        let (status, body) = self.cooee.post("/threads", &new);
        assert_eq!(status, 200, "{body}");
        let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let cursor = self.cooee.json(&format!("/threads/{id}/messages"))["cursor"]
            .as_str()
            .unwrap()
            .to_owned();
        let boot = cursor.split('.').next().unwrap();
        self.cooee
            .events(&id, &format!("{boot}.0"), false)
            .until(|e| is_item(e, "agent"));
        id
    }

    fn send(&self, args: &[&str]) -> Output {
        std::process::Command::new(env!("CARGO_BIN_EXE_cooee"))
            .arg("--config")
            .arg(&self.config)
            .arg("send")
            .args(args)
            .output()
            .unwrap()
    }

    fn json(&self, args: &[&str], code: i32) -> Value {
        let out = self.send(args);
        assert_eq!(out.status.code(), Some(code), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
}

#[test]
fn a_finished_turn_exits_0_with_its_reply_and_the_cli_pairs_once() {
    let s = setup("send-done");
    let id = s.thread();
    let done = s.json(&[&id, "echo scripted", "--wait", "--json"], 0);
    assert_eq!(
        done,
        json!({ "thread": id, "state": "done", "reply": "scripted", "stop_reason": "end_turn", "approval": null })
    );
    let out = s.send(&[&id, "echo plain", "--wait"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "plain\n");
    let sent = s.json(&[&id, "echo later", "--json"], 0);
    assert_eq!(
        sent,
        json!({ "thread": id, "state": "sent", "reply": null, "stop_reason": null, "approval": null })
    );
    let devices = fs::read_to_string(s.dir.join("data/devices.json")).unwrap();
    assert_eq!(devices.matches("\"cooee CLI\"").count(), 1, "{devices}");
}

#[test]
fn a_pending_approval_exits_3_with_its_options() {
    let s = setup("send-approval");
    let id = s.thread();
    let asked = s.json(&[&id, "please approve an edit", "--wait", "--json"], 3);
    assert_eq!(asked["state"], "approval", "{asked}");
    assert_eq!(asked["reply"], Value::Null);
    assert_eq!(
        asked["approval"]["options"],
        json!(["allow", "always", "reject"])
    );
    let item = asked["approval"]["item"].as_u64().unwrap();
    let tap = json!({ "item": item, "option": "allow" });
    let (status, body) = s.cooee.post(&format!("/threads/{id}/approve"), &tap);
    assert_eq!(status, 200, "{body}");
}

#[test]
fn a_turn_that_runs_past_the_timeout_exits_124() {
    let s = setup("send-timeout");
    let id = s.thread();
    let late = s.json(&[&id, "hang", "--wait", "--timeout", "1", "--json"], 124);
    assert_eq!(
        late,
        json!({ "thread": id, "state": "timeout", "reply": null, "stop_reason": null, "approval": null })
    );
}

#[test]
fn an_ended_session_or_unknown_thread_exits_1_and_bad_words_exit_2() {
    let s = setup("send-error");
    let id = s.thread();
    let ended = s.json(&[&id, "exit now", "--wait", "--json"], 1);
    assert_eq!(ended["state"], "ended", "{ended}");
    let missing = s.send(&["nosuchthread", "hi", "--wait"]);
    assert_eq!(missing.status.code(), Some(1), "{missing:?}");
    for args in [
        &[id.as_str()][..],
        &[&id, "hi", "--timeout", "soon"],
        &["../threads", "hi"],
    ] {
        assert_eq!(s.send(args).status.code(), Some(2), "{args:?}");
    }
}

#[test]
fn new_starts_a_chat_with_the_persona_and_waits_for_its_reply() {
    let s = setup("send-new");
    let persona =
        json!({ "name": "Eval", "agent": "fake-acp", "folder": "scratch", "style": "detailed" });
    let (status, body) = s.cooee.post("/personas", &persona);
    assert_eq!(status, 200, "{body}");
    let done = s.json(&["--new", "eval", "echo scripted", "--wait", "--json"], 0);
    assert_eq!(
        (&done["state"], &done["reply"]),
        (&json!("done"), &json!("scripted")),
        "{done}"
    );
    let id = done["thread"].as_str().unwrap();
    let rows = s.cooee.json("/threads?persona=eval");
    let ids: Vec<&str> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, [id]);
    let missing = s.send(&["--new", "nobody", "hi", "--wait"]);
    assert_eq!(missing.status.code(), Some(1), "{missing:?}");
}
