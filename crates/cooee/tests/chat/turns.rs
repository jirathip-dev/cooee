use std::{fs, thread};

use serde_json::{Value, json};

use crate::{
    setup::{notes, restarts, setup, terminal, text_of},
    support::{Cooee, Sse, is_item, others_go_on, version},
};

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
    assert!(
        matches!(text_of(&events).as_str(), "" | "Hello"),
        "{events:?}"
    );
    let reply = events.last().unwrap();
    assert_eq!(
        (&reply.data["text"], &reply.data["stop_reason"]),
        (&json!("Hello"), &json!("end_turn"))
    );

    let kinds: Vec<Value> = s.items(&id).iter().map(|i| i["kind"].clone()).collect();
    assert_eq!(kinds, ["you", "agent"]);
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_cooee"))
        .arg("--config")
        .arg(&s.cooee.config)
        .args(["export", &id])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let lines: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines, s.items(&id));
    let list = s.cooee.json("/threads");
    let row = list
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == id.as_str());
    assert_eq!(row.map(|r| &r["last"]), Some(&json!("Hello")), "{list}");
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
    let (status, body) = s.send(&id, "untagged", "");
    assert_eq!(status, 400, "{body}");
    let prompts = s.shed.sent(&terminal(&id));
    assert_eq!(
        prompts.iter().filter(|t| t.contains("use a tool")).count(),
        1
    );
    assert!(
        !prompts.iter().any(|t| t.contains("untagged")),
        "{prompts:?}"
    );

    let events = events.until(|e| is_item(e, "agent"));
    let tools: Vec<&Sse> = events.iter().filter(|e| is_item(e, "tool")).collect();
    let statuses: Vec<&Value> = tools.iter().map(|e| &e.data["status"]).collect();
    assert_eq!(statuses, ["pending", "in_progress", "completed"]);
    assert!(tools.iter().all(|e| e.data["id"] == tools[0].data["id"]));
    let rows: Vec<(&Value, &Value)> = tools
        .iter()
        .map(|e| (&e.data["action"], &e.data["title"]))
        .collect();
    assert!(
        rows.iter()
            .all(|r| *r == (&json!("read"), &json!("src/lib.rs"))),
        "{rows:?}"
    );
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
fn deny_and_say_rejects_the_call_and_the_agent_gets_the_text_once_under_the_denial() {
    let s = setup("deny-say");
    let id = s.create("please approve an edit");
    let mut events = s.events(&id);
    let ask = events.until(|e| is_item(e, "approval")).pop().unwrap();
    let choice = &ask.data["block"]["blocks"][0];
    let last = choice["options"].as_array().unwrap().last().unwrap();
    assert_eq!(
        (&last["value"], &last["label"]),
        (&json!("say"), &json!("Deny and say…"))
    );
    let item = ask.data["id"].as_u64().unwrap();
    let path = format!("/threads/{id}/approve");
    let say = json!({ "item": item, "option": "say", "value": "fake says: Use a branch instead." });
    thread::scope(|scope| {
        for _ in 0..3 {
            scope.spawn(|| assert_eq!(s.cooee.post(&path, &say).0, 200));
        }
    });
    events.until(|e| is_item(e, "agent") && e.data["text"] == "Use a branch instead.");
    assert_eq!(s.cooee.post(&path, &say).0, 200);
    let allow = json!({ "item": item, "option": "allow" });
    assert_eq!(s.cooee.post(&path, &allow).0, 200);
    let sent = s.shed.sent(&terminal(&id));
    let approvals: Vec<&String> = sent.iter().filter(|t| t.contains("approve\":")).collect();
    assert_eq!(approvals, ["\x1e{\"approve\":\"reject\"}\n"]);
    let said = sent.iter().filter(|t| t.contains("Use a branch")).count();
    assert_eq!(said, 1, "{sent:?}");

    let items = s.items(&id);
    let kinds: Vec<&Value> = items.iter().map(|i| &i["kind"]).collect();
    assert_eq!(kinds, ["you", "tool", "approval", "agent", "agent"]);
    let card = items.iter().find(|i| i["kind"] == "approval").unwrap();
    assert_eq!(card["chosen"], "reject");
    let lines: Vec<&Value> = card["block"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| &b["markdown"])
        .collect();
    assert_eq!(
        lines,
        ["✓ reject · browser", "fake says: Use a branch instead."]
    );
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
fn a_message_to_a_stopped_agent_is_refused_never_stored_as_sent() {
    let s = setup("stopped");
    let id = s.create("exit now");
    s.events(&id)
        .until(|e| e.event == "state" && e.data["state"] == "ended");
    let (status, body) = s.send(&id, "are you there", "r1");
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("stopped"), "{body}");
    let mine = s.items(&id);
    assert!(
        !mine.iter().any(|i| i["text"] == "are you there"),
        "{mine:?}"
    );
}

#[test]
fn a_hanging_shed_approve_never_holds_another_thread() {
    let s = setup("approve-hang");
    let id = s.create("please approve an edit");
    let ask = s
        .events(&id)
        .until(|e| is_item(e, "approval"))
        .pop()
        .unwrap();
    let other = s.create("Say hi");
    s.events(&other).until(|e| is_item(e, "agent"));
    let gate = format!("hang-send-{}", terminal(&id));
    s.shed.gate(&gate, true);
    let tap = json!({ "item": ask.data["id"], "option": "allow" });
    let path = format!("/threads/{id}/approve");
    thread::scope(|scope| {
        let tapped = scope.spawn(|| s.cooee.post(&path, &tap));
        let approving = || {
            s.shed
                .sent(&terminal(&id))
                .iter()
                .any(|t| t.contains("approve"))
        };
        while !approving() {
            thread::sleep(std::time::Duration::from_millis(20));
        }
        others_go_on(&s.cooee, &other);
        assert!(!tapped.is_finished(), "the approve still hangs in shed");
        s.shed.gate(&gate, false);
        let (code, body) = tapped.join().unwrap();
        assert_eq!(code, 200, "{body}");
    });
}

#[test]
fn a_send_to_a_gone_agent_starts_a_fresh_one_in_the_chat_and_delivers() {
    let s = setup("gone-send");
    let id = s.create("hi");
    let mut events = s.events(&id);
    events.until(|e| is_item(e, "agent"));
    s.shed.vanish(&terminal(&id));
    let (status, body) = s.send(&id, "echo back", "g1");
    assert_eq!(status, 200, "{body}");
    events.until(|e| is_item(e, "agent") && e.data["text"] == "back");
    let again = restarts(&s);
    assert_eq!(again.len(), 1, "{again:?}");
    assert!(!again[0].contains("--resume"), "{again:?}");
    assert_eq!(notes(&s, &id), 1, "one line says it started fresh");
}

#[test]
fn a_gone_agent_that_reported_its_session_resumes_it_on_send() {
    let s = setup("gone-resume");
    fs::write(s.shed.dir.join("session.json"), r#"{"id":"sess-1"}"#).unwrap();
    let id = s.create("hi");
    let mut events = s.events(&id);
    events.until(|e| is_item(e, "agent"));
    s.shed.vanish(&terminal(&id));
    let (status, body) = s.send(&id, "echo back", "g1");
    assert_eq!(status, 200, "{body}");
    events.until(|e| is_item(e, "agent") && e.data["text"] == "back");
    let again = restarts(&s);
    assert_eq!(again.len(), 1, "{again:?}");
    assert!(again[0].contains("--resume sess-1"), "{again:?}");
    assert_eq!(notes(&s, &id), 0, "resumed: no fresh-start line");
}

#[test]
fn restart_on_the_gone_line_starts_the_agent_and_leaves_ended() {
    let mut s = setup("gone-restart");
    let id = s.create("hi");
    let mut events = s.events(&id);
    events.until(|e| is_item(e, "agent"));
    s.shed.vanish(&terminal(&id));
    let _shed = s.shed.restart(version());
    events.until(|e| e.event == "state" && e.data["state"] == "ended");
    let path = format!("/threads/{id}/restart");
    let (status, body) = s.cooee.post(&path, &json!({}));
    assert_eq!(status, 200, "{body}");
    events.until(|e| e.event == "state" && e.data["state"] != "ended");
    assert_eq!(restarts(&s).len(), 1);
    assert_eq!(s.cooee.post(&path, &json!({})).0, 200);
    assert_eq!(restarts(&s).len(), 1, "a second tap starts nothing");
    let (status, body) = s.send(&id, "echo back", "g1");
    assert_eq!(status, 200, "{body}");
    events.until(|e| is_item(e, "agent") && e.data["text"] == "back");
    drop(events);
    drop(std::mem::replace(
        &mut s.cooee,
        Cooee::start(&s.dir, &s.shed.dir),
    ));
    let mut events = s.events(&id);
    let (status, body) = s.send(&id, "echo again", "g2");
    assert_eq!(status, 200, "{body}");
    events.until(|e| is_item(e, "agent") && e.data["text"] == "again");
}

fn gone_scratch_chat(
    s: &crate::setup::Setup,
    rid: &str,
) -> (String, std::path::PathBuf, crate::support::Events) {
    let new = json!({ "agent": "fake-acp", "text": "hi", "random_id": rid });
    let (status, body) = s.cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let own = s.dir.join("data/scratch").join(&id);
    let mut events = s.events(&id);
    events.until(|e| is_item(e, "agent"));
    s.shed.vanish(&terminal(&id));
    fs::remove_dir_all(&own).unwrap();
    (id, own, events)
}

#[test]
fn a_gone_chat_whose_scratch_folder_was_deleted_restarts_on_send_and_delivers() {
    let s = setup("gone-scratch-send");
    let (id, own, mut events) = gone_scratch_chat(&s, "s0");
    let (status, body) = s.send(&id, "echo back", "g1");
    assert_eq!(status, 200, "{body}");
    events.until(|e| is_item(e, "agent") && e.data["text"] == "back");
    assert!(own.is_dir(), "{}", own.display());
}

#[test]
fn restart_on_the_gone_line_recreates_a_deleted_scratch_folder() {
    let s = setup("gone-scratch-tap");
    let (id, _, mut events) = gone_scratch_chat(&s, "s1");
    let (status, body) = s.cooee.post(&format!("/threads/{id}/restart"), &json!({}));
    assert_eq!(status, 200, "{body}");
    assert_eq!(restarts(&s).len(), 1);
    let (status, body) = s.send(&id, "echo back", "g1");
    assert_eq!(status, 200, "{body}");
    events.until(|e| is_item(e, "agent") && e.data["text"] == "back");
}

#[test]
fn a_restart_that_fails_says_why_in_the_chat() {
    let s = setup("gone-restart-fails");
    let (id, _, _) = gone_scratch_chat(&s, "s2");
    fs::write(
        s.shed.dir.join("run-fails"),
        "SPAWN_FAILED: pty: no tty left",
    )
    .unwrap();
    let (status, body) = s.cooee.post(&format!("/threads/{id}/restart"), &json!({}));
    assert_ne!(status, 200, "{body}");
    let said = s.items(&id);
    let note = said.iter().rev().find(|i| i["kind"] == "note").unwrap();
    assert!(
        note["text"].as_str().unwrap().contains("no tty left"),
        "{note}"
    );
}
