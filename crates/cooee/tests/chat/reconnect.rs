use std::{fs, thread};

use serde_json::{Value, json};

use crate::{
    setup::{setup, terminal},
    support::{Cooee, Sse, is_item, version},
};

#[test]
fn a_reconnect_with_last_event_id_replays_without_gaps_or_duplicates_and_revs_count_up_across_threads()
 {
    let s = setup("reconnect");
    let id = s.create("use a tool");
    let all = s.events(&id).until(|e| is_item(e, "agent"));
    assert!(all.len() > 4, "{all:?}");
    let cursor = s.cooee.json(&format!("/threads/{id}/messages"))["cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let boot = cursor.split('.').next().unwrap();
    for cut in 1..all.len() {
        let mut events = s.cooee.events(&id, &all[cut - 1].id, true);
        let rest = events.until(|e| e.id == all.last().unwrap().id);
        let ids = |v: &[Sse]| v.iter().map(|e| e.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&rest), ids(&all[cut..]), "after {}", all[cut - 1].id);
    }
    for stale in ["0.0".to_owned(), format!("{boot}.99999")] {
        assert_eq!(s.cooee.events(&id, &stale, true).next().event, "reset");
    }
    let other = s.create("hi");
    let theirs = s.events(&other).until(|e| is_item(e, "agent"));
    let rev = |e: &Sse| e.id.rsplit_once('.').unwrap().1.parse::<u64>().unwrap();
    let revs: Vec<u64> = all.iter().chain(&theirs).map(rev).collect();
    assert!(revs.windows(2).all(|w| w[0] < w[1]), "{revs:?}");
}

#[test]
fn a_restarted_cooee_reattaches_to_its_terminals_and_catches_up() {
    let mut s = setup("restart");
    let id = s.create("hang on");
    s.events(&id).until(|e| e.event == "text");
    drop(s.cooee);
    s.shed.input(&terminal(&id), "\x1e\"cancel\"\n");
    s.cooee = Cooee::start(&s.dir, &s.shed.dir);
    let mut events = s.events(&id);
    events.until(|e| is_item(e, "agent") && e.data["stop_reason"] == "cancelled");
    let gets = s
        .shed
        .requests
        .lock()
        .unwrap()
        .iter()
        .filter(|(m, p)| m == "get" && p["name"] == terminal(&id))
        .count();
    assert!(gets >= 1, "re-attached by name");
    assert_eq!(s.send(&id, "again", "r2").0, 200);
    events.until(|e| is_item(e, "agent") && e.data["text"] == "Hello");
    let kinds: Vec<Value> = s.items(&id).iter().map(|i| i["kind"].clone()).collect();
    assert_eq!(kinds, ["you", "tool", "agent", "you", "agent"]);
}

#[test]
fn a_reset_from_a_busy_shed_reads_on_and_drops_no_later_event() {
    let s = setup("shed-reset");
    let id = s.create("hi");
    let mut events = s.events(&id);
    events.until(|e| is_item(e, "agent"));
    let floor = s.shed.forget(1_304_000);
    s.shed.churn();
    let _shed = s.shed.restart(version());
    let stale = || -> usize {
        let requests = s.shed.requests.lock().unwrap();
        let events = requests.iter().filter(|(m, _)| m == "events");
        events
            .filter(|(_, p)| p["since"].as_u64().is_some_and(|n| n < floor))
            .count()
    };
    for _ in 0..500 {
        if stale() > 0 {
            break;
        }
        thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(stale(), 1, "one stale subscribe, then a reset");
    assert_eq!(s.send(&id, "again", "reset-1").0, 200);
    events.until(|e| is_item(e, "agent") && e.data["text"] == "Hello");
    let log = fs::read_to_string(s.dir.join("cooee.err")).unwrap();
    assert!(!log.contains("dropped a shed event"), "{log}");
    assert_eq!(stale(), 1, "read on after the reset");
}

#[test]
fn closing_a_thread_kills_and_removes_its_terminal() {
    let s = setup("close");
    let id = s.create("hi");
    let mut events = s.events(&id);
    events.until(|e| is_item(e, "agent"));
    assert_eq!(
        s.cooee.post(&format!("/threads/{id}/close"), &json!({})).0,
        200
    );
    let calls: Vec<String> = s
        .shed
        .requests
        .lock()
        .unwrap()
        .iter()
        .filter(|(m, p)| (m == "kill" || m == "rm") && p["name"] == terminal(&id))
        .map(|(m, _)| m.clone())
        .collect();
    assert_eq!(calls, ["kill", "rm"]);
    events.until(|e| e.event == "state" && e.data["state"] == "ended");
    assert_eq!(s.send(&id, "more", "r9").0, 400);
    let stored = fs::read_to_string(s.dir.join(format!("data/threads/{id}.json"))).unwrap();
    assert!(stored.contains("\"closed\": true"), "{stored}");
}
