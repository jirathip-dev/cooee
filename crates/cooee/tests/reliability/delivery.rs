use std::{
    fs, thread,
    time::{Duration, Instant},
};

use serde_json::json;

use crate::{
    setup::{id_of, listed, prompts, setup, start, until},
    support::{Cooee, FakeShed, scratch, version},
};

#[test]
fn r16_a_repeated_id_is_one_row_however_often_it_is_refreshed() {
    let dir = scratch("r16-dup");
    let threads = dir.join("data/threads");
    fs::create_dir_all(&threads).unwrap();
    fs::write(
        threads.join("dup.json"),
        r#"{"format":1,"thread":{"id":"dup","title":"Dup","messages":[],"terminal":"cooee-dup"}}"#,
    )
    .unwrap();
    let line = |text: &str| {
        format!("{{\"id\":1,\"at\":1,\"kind\":\"you\",\"text\":\"{text}\",\"random_id\":\"r\"}}\n")
    };
    let lines: String = (0..5).map(|i| line(&format!("v{i}"))).collect();
    fs::write(threads.join("dup.jsonl"), lines).unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    for _ in 0..5 {
        let items = cooee.json("/threads/dup/messages")["items"].clone();
        assert_eq!(items.as_array().unwrap().len(), 1, "{items}");
        assert_eq!(items[0]["text"], "v4");
        assert_eq!(listed(&cooee), ["dup"]);
    }
}

#[test]
fn r20_a_stuck_delivery_never_holds_the_next_one() {
    let s = setup("r20-stuck");
    let (status, body) = s.cooee.post("/threads", &start(&s, "hi", "r20"));
    assert_eq!(status, 200, "{body}");
    let id = id_of(&body);
    let path = format!("/threads/{id}/send");
    s.shed.gate("hang-send", true);
    thread::scope(|scope| {
        let first = scope.spawn(|| {
            s.cooee
                .post(&path, &json!({ "text": "one", "random_id": "a" }))
        });
        until("the first send is stuck in shed", || {
            prompts(&s.shed, &id).len() == 2
        });
        let at = Instant::now();
        let (status, body) = s
            .cooee
            .post(&path, &json!({ "text": "two", "random_id": "b" }));
        let took = at.elapsed();
        assert!(status >= 500, "{status}: {body}");
        assert!(took < Duration::from_secs(30), "{took:?}");
        assert_ne!(first.join().unwrap().0, 200);
    });
    s.shed.gate("hang-send", false);
    let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    assert!(!items.to_string().contains("\"two\""), "{items}");
}

#[test]
fn r21_an_answer_to_a_gone_session_says_not_delivered() {
    let s = setup("r21-gone");
    let (status, body) = s
        .cooee
        .post("/threads", &start(&s, "please approve an edit", "r21"));
    assert_eq!(status, 200, "{body}");
    let id = id_of(&body);
    let approval = || {
        let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
        let items = items.as_array().unwrap().clone();
        items.into_iter().find(|i| i["kind"] == "approval")
    };
    until("the agent asks", || approval().is_some());
    s.shed.vanish(&format!("cooee-{id}"));
    let ask = approval().unwrap();
    let tap = json!({ "item": ask["id"], "option": "allow" });
    let (status, body) = s.cooee.post(&format!("/threads/{id}/approve"), &tap);
    assert_eq!(
        (status, body.as_str()),
        (410, "Not delivered: this thread's agent is gone.")
    );
    let ask = approval().unwrap();
    assert!(
        ask.get("tapped").is_none() && ask.get("chosen").is_none(),
        "{ask}"
    );
}

#[test]
fn r27_a_long_tool_run_is_working_until_its_turn_ends_and_reads_no_screen() {
    let s = setup("r27-state");
    let (status, body) = s.cooee.post("/threads", &start(&s, "hi", "r27"));
    assert_eq!(status, 200, "{body}");
    let id = id_of(&body);
    let messages = format!("/threads/{id}/messages");
    until("the first turn ends", || {
        let turn = &s.cooee.json(&messages)["turn"];
        turn["state"] == "done" && turn["note"] == "end_turn"
    });
    let cursor = s.cooee.json(&messages)["cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut events = s.cooee.events(&id, &cursor, false);
    let run = json!({ "text": "a long tool run", "random_id": "r27b" });
    let (status, body) = s.cooee.post(&format!("/threads/{id}/send"), &run);
    assert_eq!(status, 200, "{body}");
    let seen: Vec<String> = events
        .until(|e| e.event == "state" && e.data["state"] == "idle")
        .into_iter()
        .filter(|e| e.event == "state")
        .map(|e| e.data["state"].as_str().unwrap().to_owned())
        .collect();
    let mut pill: Vec<bool> = seen.iter().map(|s| s == "working").collect();
    pill.dedup();
    assert_eq!(pill, [true, false], "{seen:?}");
    assert_eq!(seen.iter().filter(|s| *s == "working").count(), 5);
    let requests = s.shed.requests.lock().unwrap();
    let screen = requests
        .iter()
        .filter(|(m, _)| m == "read" || m.starts_with("screen."));
    assert_eq!(screen.count(), 0, "{requests:?}");
}
