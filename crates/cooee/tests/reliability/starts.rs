use std::{process::Command, thread, time::Duration};

use serde_json::json;

use crate::{
    setup::{approval, id_of, listed, post_raw, prompts, runs, setup, start, until},
    support::{Cooee, FakeShed, scratch, version},
};

#[test]
fn r9_an_ask_after_the_terminal_is_replaced_is_answered_in_the_new_session() {
    let s = setup("r9-replaced");
    let new = json!({ "agent": "fake-acp", "text": "hi", "random_id": "r9" });
    let (status, body) = s.cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = id_of(&body);
    let old = format!("cooee-{id}");
    let folder = json!({ "folder": s.dir.join("root/project").canonicalize().unwrap() });
    let (status, body) = s.cooee.post(&format!("/threads/{id}/folder"), &folder);
    assert_eq!(status, 200, "{body}");
    let log = s.shed.cli_log();
    let replaced = log.lines().find_map(|l| {
        let name = l.split("--name ").nth(1)?.split(' ').next()?;
        name.starts_with(&format!("{old}-"))
            .then(|| name.to_owned())
    });
    let replaced = replaced.unwrap_or_else(|| panic!("no new terminal: {log}"));

    let ask = json!({ "text": "please approve an edit", "random_id": "r9b" });
    let (status, body) = s.cooee.post(&format!("/threads/{id}/send"), &ask);
    assert_eq!(status, 200, "{body}");
    until("the new session asks", || approval(&s.cooee, &id).is_some());
    let item = approval(&s.cooee, &id).unwrap()["id"].clone();
    let tap = json!({ "item": item, "option": "allow" });
    let (status, body) = s.cooee.post(&format!("/threads/{id}/approve"), &tap);
    assert_eq!(status, 200, "{body}");
    let answer = "\x1e{\"approve\":\"allow\"}\n".to_owned();
    assert!(s.shed.sent(&replaced).contains(&answer), "{replaced}");
    assert!(!s.shed.sent(&old).contains(&answer), "{old}");
    until("the new session's edit lands", || {
        approval(&s.cooee, &id).unwrap()["chosen"] == "allow"
    });
}

#[test]
fn r10_an_ask_made_during_the_start_is_listed_and_answerable() {
    let s = setup("r10-early-ask");
    s.shed.gate("hold-run", true);
    thread::scope(|scope| {
        let created = scope.spawn(|| s.cooee.post("/threads", &start(&s, "hi", "r10")));
        until("the start runs", || runs(&s.shed) == 1);
        let log = s.shed.cli_log();
        let name = log
            .split("--name ")
            .nth(1)
            .unwrap()
            .split(' ')
            .next()
            .unwrap();
        s.shed.ask(name);
        s.shed.gate("hold-run", false);
        let (status, body) = created.join().unwrap();
        assert_eq!(status, 200, "{body}");
        let id = id_of(&body);
        let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
        let ask = items
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["kind"] == "approval")
            .unwrap_or_else(|| panic!("the ask is missing: {items}"));
        assert_eq!(ask["tool"], "Edit early.rs");
        let tap = json!({ "item": ask["id"], "option": "allow" });
        let (status, body) = s.cooee.post(&format!("/threads/{id}/approve"), &tap);
        assert_eq!(status, 200, "{body}");
        let sent = s.shed.sent(&format!("cooee-{id}"));
        assert!(
            sent.contains(&"\x1e{\"approve\":\"allow\"}\n".to_owned()),
            "{sent:?}"
        );
    });
}

#[test]
fn r11_a_slow_start_is_adopted_once() {
    let s = setup("r11-slow-start");
    s.shed.gate("ready-timeout", true);
    let (status, body) = s.cooee.post("/threads", &start(&s, "hi", "r11"));
    assert_eq!(status, 502, "{body}");
    let first = listed(&s.cooee);
    assert_eq!(first.len(), 1, "the slow start is listed: {first:?}");
    s.shed.gate("ready-timeout", false);
    let (status, body) = s.cooee.post("/threads", &start(&s, "hi", "r11"));
    assert_eq!(status, 200, "{body}");
    assert_eq!(id_of(&body), first[0]);
    assert_eq!(runs(&s.shed), 1);
    assert_eq!(prompts(&s.shed, &first[0]).len(), 1);
    assert_eq!(listed(&s.cooee), first);

    let (status, body) = s.cooee.post("/threads", &start(&s, "hello", "r11b"));
    assert_eq!(status, 200, "{body}");
    s.shed.gate("run-fails", false);
    let id = id_of(&body);
    assert_eq!(runs(&s.shed), 2);
    assert_eq!(prompts(&s.shed, &id).len(), 1);
    assert_eq!(listed(&s.cooee).len(), 2);
}

#[test]
fn r14_a_crash_between_start_and_send_is_recovered_once() {
    let dir = scratch("r14-crash");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let body = json!({ "agent": "fake-acp", "folder": dir.join("root/project"), "text": "hi", "random_id": "r14" });
    shed.gate("hold-ready", true);
    let (addr, cookie, b) = (cooee.addr.clone(), cooee.cookie.clone(), body.clone());
    let lost = thread::spawn(move || post_raw(&addr, &cookie, "/threads", &b));
    until("the agent started", || runs(&shed) == 1);
    let killed = Command::new("kill")
        .args(["-9", &cooee.child.id().to_string()])
        .status();
    assert!(killed.unwrap().success());
    lost.join().ok();
    drop(cooee);
    shed.gate("hold-ready", false);

    let cooee = Cooee::start(&dir, &shed.dir);
    let ids = listed(&cooee);
    assert_eq!(ids.len(), 1, "{ids:?}");
    let (status, answer) = cooee.post("/threads", &body);
    assert_eq!(status, 200, "{answer}");
    assert_eq!(id_of(&answer), ids[0]);
    assert_eq!(runs(&shed), 1);
    assert_eq!(prompts(&shed, &ids[0]).len(), 1);
    assert_eq!(listed(&cooee), ids);
}

#[test]
fn a_background_start_answers_before_the_agent_runs_and_sends_once() {
    let s = setup("start-later");
    s.shed.gate("hold-run", true);
    let mut new = start(&s, "hi there", "later");
    new["background"] = json!(true);
    let (status, body) = s.cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = id_of(&body);
    let slice = s.cooee.json(&format!("/threads/{id}/messages"));
    assert_eq!(slice["turn"]["state"], "connecting", "{slice}");
    assert_eq!(slice["items"], json!([]), "{slice}");
    assert_eq!(
        slice["pending"],
        json!({ "text": "hi there", "random_id": "later" })
    );
    assert_eq!(listed(&s.cooee), std::slice::from_ref(&id));
    let (_, again) = s.cooee.post("/threads", &new);
    assert_eq!(id_of(&again), id);
    s.shed.gate("hold-run", false);
    until("the first message went", || {
        prompts(&s.shed, &id).len() == 1
    });
    until("it is an item", || {
        let slice = s.cooee.json(&format!("/threads/{id}/messages"));
        slice["items"][0]["text"] == "hi there"
            && slice.get("pending").is_none()
            && slice["turn"]["state"] != "connecting"
    });
    assert_eq!((runs(&s.shed), prompts(&s.shed, &id).len()), (1, 1));
}

#[test]
fn a_failed_background_start_says_why_and_a_retry_sends_once() {
    let s = setup("start-later-fails");
    s.shed.gate("ready-timeout", true);
    let mut new = start(&s, "hi", "fails");
    new["background"] = json!(true);
    let (status, body) = s.cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = id_of(&body);
    let messages = format!("/threads/{id}/messages");
    until("the start failed", || {
        s.cooee.json(&messages)["turn"]["state"] == "unconnected"
    });
    let slice = s.cooee.json(&messages);
    let why = slice["turn"]["note"].as_str().unwrap();
    assert!(
        why.starts_with("fake stopped: ") && !why.contains("fake-acp"),
        "{slice}"
    );
    assert_eq!(
        slice["pending"]["text"], "hi",
        "the message is kept: {slice}"
    );
    assert_eq!(listed(&s.cooee), std::slice::from_ref(&id));
    assert_eq!(prompts(&s.shed, &id), Vec::<String>::new());

    s.shed.gate("ready-timeout", false);
    let (status, body) = s.cooee.post(&format!("/threads/{id}/retry"), &json!({}));
    assert_eq!(status, 200, "{body}");
    until("the retry sent it", || prompts(&s.shed, &id).len() == 1);
    until("it is an item", || {
        s.cooee.json(&messages)["items"][0]["text"] == "hi"
    });
    s.cooee.post(&format!("/threads/{id}/retry"), &json!({}));
    thread::sleep(Duration::from_millis(200));
    assert_eq!((runs(&s.shed), prompts(&s.shed, &id).len()), (1, 1));
}

#[test]
fn a_start_with_no_folder_runs_in_its_own_scratch_folder() {
    let s = setup("scratch");
    let new =
        json!({ "agent": "fake-acp", "text": "hi", "random_id": "scratch", "background": true });
    let (status, body) = s.cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = id_of(&body);
    until("the first message went", || {
        prompts(&s.shed, &id).len() == 1
    });
    let dir = s.dir.join("data/scratch").join(&id);
    assert!(dir.is_dir(), "{}", dir.display());
    let cwd = format!("--cwd {}", dir.display());
    assert!(s.shed.cli_log().contains(&cwd), "{}", s.shed.cli_log());
    let list = s.cooee.json("/threads");
    assert_eq!(list[0]["id"], id.as_str());
    assert_eq!(list[0]["folder"], "", "{list}");
    assert!(!list.to_string().contains("data/scratch"), "{list}");
}
