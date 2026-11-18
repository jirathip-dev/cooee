use cooee_core::shed_proto::{ApprovalOption, EventKind, Reported};

use super::*;

pub(super) fn hub(dir: &Path) -> Arc<Hub> {
    hub_on(dir, dir.join("no-shed"))
}

fn hub_on(dir: &Path, bin: PathBuf) -> Arc<Hub> {
    let shed = Shed::new(bin, Some(dir.join("no-shed-dir")));
    let (looks, _) = crate::look::Looks::load(None, None);
    let theme = looks.themes[&looks.default].clone();
    Hub::open(
        (shed, cooee_core::Providers::default()),
        &dir.join("data"),
        (Vec::new(), None),
        theme,
        Vec::new(),
        None,
        30 * 60_000,
    )
    .unwrap()
}

pub(super) fn scratch(name: &str) -> PathBuf {
    let dir = cooee_core::scratch::owned(
        std::env::temp_dir().join(format!("cooee-hub-{name}-{}", std::process::id())),
    );
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(dir.join("data/threads")).unwrap();
    dir
}

pub(super) fn seed(dir: &Path, id: &str, lines: &str) {
    let threads = dir.join("data/threads");
    let meta = json!({ "format": 1, "thread": { "id": id, "title": id, "messages": [], "terminal": format!("cooee-{id}") } });
    fs::write(threads.join(format!("{id}.json")), meta.to_string()).unwrap();
    fs::write(threads.join(format!("{id}.jsonl")), lines).unwrap();
}

fn event(id: &str, seq: u64, kind: EventKind) -> Event {
    Event {
        seq,
        name: format!("cooee-{id}"),
        labels: BTreeMap::new(),
        kind,
        replay: false,
        parent: None,
    }
}

#[test]
fn r6_requests_over_a_long_log_are_within_budget() {
    let dir = scratch("budget");
    let mut lines = String::new();
    for v in 0..30 {
        for id in 1..=1000 {
            let line = json!({ "id": id, "at": id, "seq": v * 1000 + id, "kind": "agent", "text": format!("reply {id} v{v}"), "stop_reason": "end_turn" });
            lines += &line.to_string();
            lines.push('\n');
        }
    }
    seed(&dir, "long", &lines);
    let hub = hub(&dir);
    let at = cooee_core::clock::instant();
    let slice = hub.slice("long", Some(500), 50, 50).unwrap();
    let newest = hub.slice("long", None, 500, 0).unwrap();
    let rows = hub.list();
    hub.thread("long").unwrap();
    let took = at.elapsed();
    assert_eq!((slice.items.len(), slice.skipped_before), (101, 449));
    assert_eq!(
        slice.items[0].body,
        Body::Agent {
            text: "reply 450 v29".into(),
            stop_reason: "end_turn".into()
        }
    );
    assert_eq!((newest.items.len(), rows.len()), (500, 1));
    assert!(took < Duration::from_millis(300), "{took:?}");
}

#[test]
fn r15_a_turn_idle_30_minutes_is_surfaced_as_needs_you() {
    let dir = scratch("stuck");
    seed(&dir, "idle", "");
    let hub = hub(&dir);
    let working = Reported {
        state: "working".to_owned(),
        ..Reported::default()
    };
    hub.apply(event("idle", 1, EventKind::State(Box::new(working))));
    let state = |hub: &Hub| hub.thread("idle").unwrap().1;
    let t0 = now();
    hub.watch(t0 + STUCK - 60_000);
    assert_eq!(state(&hub).state, "working");
    let (_, rx) = hub.subscribe("idle", "").unwrap();
    hub.watch(t0 + STUCK + 1);
    let turn = state(&hub);
    assert_eq!(
        (turn.state.as_str(), turn.note.as_deref()),
        ("blocked", Some("no progress for 30 minutes"))
    );
    hub.watch(t0 + 2 * STUCK);
    assert_eq!(rx.len(), 1);
    assert_eq!(hub.list()[0].state, "blocked");
    hub.apply(event("idle", 2, EventKind::Text { text: "w".into() }));
    assert_eq!(state(&hub).state, "working");
}

#[test]
fn a_rooms_turns_stop_at_the_deadline() {
    let dir = scratch("deadline");
    let t0 = now();
    let turn = |m: &str| json!({ "member": m, "reason": "round" });
    let meta = json!({ "format": 1, "thread": { "id": "room", "title": "room", "messages": [], "kind": "room",
        "members": [{ "persona": "ada" }, { "persona": "bo" }], "round": [turn("ada"), turn("bo")], "asked_at": t0 } });
    fs::write(dir.join("data/threads/room.json"), meta.to_string()).unwrap();
    fs::write(dir.join("data/threads/room.jsonl"), "").unwrap();
    let hub = hub(&dir);
    let round = |hub: &Hub| hub.thread("room").unwrap().0.round.len();
    hub.watch(t0 + rooms::DEADLINE - 1);
    assert_eq!(round(&hub), 2);
    let (_, mut rx) = hub.subscribe("room", "").unwrap();
    hub.watch(t0 + rooms::DEADLINE);
    assert_eq!(round(&hub), 0);
    let stop = rx.try_recv().unwrap();
    assert_eq!(
        (stop.event, stop.data),
        ("room-stop", json!({ "reason": "deadline" }).to_string())
    );
    let note = rx.try_recv().unwrap();
    assert!(note.data.contains("10 minutes passed"), "{}", note.data);
    hub.watch(t0 + 2 * rooms::DEADLINE);
    assert!(rx.try_recv().is_err());
}

#[test]
fn a_members_ask_shows_in_its_room_and_pauses_its_deadline() {
    let dir = scratch("room-ask");
    let t0 = now();
    let turn = |m: &str| json!({ "member": m, "reason": "round" });
    let threads = dir.join("data/threads");
    let room = json!({ "format": 1, "thread": { "id": "room", "title": "room", "messages": [], "kind": "room",
        "members": [{ "persona": "ada", "session": "s1" }], "round": [turn("ada")], "asked_at": t0 - 9 * 60_000 } });
    fs::write(threads.join("room.json"), room.to_string()).unwrap();
    fs::write(threads.join("room.jsonl"), "").unwrap();
    let member = json!({ "format": 1, "thread": { "id": "s1", "title": "s1", "messages": [],
        "terminal": "cooee-s1", "room": "room", "profile": "ada" } });
    fs::write(threads.join("s1.json"), member.to_string()).unwrap();
    fs::write(threads.join("s1.jsonl"), "").unwrap();
    let hub = hub(&dir);
    let (_, mut rx) = hub.subscribe("room", "").unwrap();
    let option = ApprovalOption {
        id: "once".into(),
        name: "Allow once".into(),
        kind: "allow_once".into(),
    };
    let (tool, options) = ("fetch sec.gov".to_owned(), vec![option]);
    hub.apply(event(
        "s1",
        1,
        EventKind::ApprovalRequest {
            id: String::new(),
            tool,
            options,
            call: None,
        },
    ));
    let ask = rx.try_recv().unwrap();
    let data: serde_json::Value = serde_json::from_str(&ask.data).unwrap();
    assert_eq!(ask.event, "room-ask");
    assert_eq!(
        (&data["member"], &data["session"]),
        (&json!("ada"), &json!("s1"))
    );
    assert_eq!(data["item"]["block"]["title"], "fetch sec.gov");
    let asks = |hub: &Hub| hub.slice("room", None, 50, 0).unwrap().asks.len();
    assert_eq!(asks(&hub), 1);
    let round = |hub: &Hub| hub.thread("room").unwrap().0.round.len();
    hub.watch(now() + rooms::DEADLINE);
    assert_eq!(round(&hub), 1, "the deadline waits while the ask is open");
    lock(&hub.live("room").unwrap()).asks.paused -= 5 * 60_000;
    hub.apply(event(
        "s1",
        2,
        EventKind::Approved {
            option: "once".into(),
        },
    ));
    let answered = rx.try_recv().unwrap();
    assert!(answered.data.contains("✓ once"), "{}", answered.data);
    assert_eq!(asks(&hub), 0);
    hub.watch(now() + 3 * 60_000);
    assert_eq!(round(&hub), 1, "five paused minutes moved the deadline on");
    hub.watch(now() + 6 * 60_000);
    assert_eq!(round(&hub), 0);
}

#[test]
fn only_a_stop_cancels_a_members_session_as_it_starts() {
    let dir = scratch("seated");
    let turn = |m: &str| json!({ "member": m, "reason": "round" });
    let room = |round: serde_json::Value| {
        json!({ "format": 1, "thread": { "id": "room", "title": "room", "messages": [],
        "kind": "room", "members": [{ "persona": "ada" }, { "persona": "bo" }], "round": round } })
    };
    let session = json!({ "format": 1, "thread": { "id": "s1", "title": "s1", "messages": [], "terminal": "cooee-s1" } });
    fs::write(dir.join("data/threads/s1.json"), session.to_string()).unwrap();
    fs::write(dir.join("data/threads/s1.jsonl"), "").unwrap();
    fs::write(dir.join("data/threads/room.jsonl"), "").unwrap();
    let seated = |hub: &Hub| hub.thread("room").unwrap().0.members[0].session.clone();

    fs::write(
        dir.join("data/threads/room.json"),
        room(json!([turn("bo")])).to_string(),
    )
    .unwrap();
    let hub = hub(&dir);
    hub.started("room", "ada", Ok("s1".to_owned())).unwrap();
    assert_eq!(seated(&hub), "s1");

    fs::write(
        dir.join("data/threads/room.json"),
        room(json!([turn("ada"), turn("bo")])).to_string(),
    )
    .unwrap();
    let hub = self::hub(&dir);
    hub.cancel("room").unwrap();
    let cancelled = hub.started("room", "ada", Ok("s1".to_owned()));
    assert!(matches!(cancelled, Err(HubError::Shed(_))), "{cancelled:?}");
    hub.started("room", "ada", Ok("s1".to_owned())).unwrap();
}

#[test]
fn the_agent_list_is_refreshed_at_most_once_a_minute() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("agents");
    let (bin, runs) = (dir.join("shed"), dir.join("runs"));
    let cli = format!(
        "#!/bin/sh\necho run >>{runs:?}\n[ -e {:?} ] && exit 1\necho '[{{\"id\":\"a\",\"version\":\"1\"}}]'\n",
        dir.join("fails")
    );
    fs::write(&bin, cli).unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
    let hub = hub_on(&dir, bin);
    let count = || fs::read_to_string(&runs).unwrap().lines().count();
    let t0 = now();
    for i in 0..10 {
        hub.refresh(t0 + i * 6_000, false);
        assert_eq!(hub.agents().unwrap()[0].id, "a");
    }
    assert_eq!(count(), 1);
    hub.refresh(t0 + AGENTS_FOR, false);
    assert_eq!(count(), 2);
    fs::write(dir.join("fails"), "").unwrap();
    hub.refresh(t0 + 2 * AGENTS_FOR, false);
    assert_eq!((count(), hub.agents().unwrap()[0].id.as_str()), (3, "a"));
}

#[test]
fn prune_drops_only_the_workflow_sessions_shed_no_longer_lists() {
    let dir = scratch("prune");
    seed(&dir, "mine", "");
    let threads = dir.join("data/threads");
    for id in ["gone", "new"] {
        let meta = json!({ "format": 1, "thread": { "id": id, "title": id, "messages": [], "terminal": format!("cooee-{id}"), "workflow": { "source": "tool" } } });
        fs::write(threads.join(format!("{id}.json")), meta.to_string()).unwrap();
    }
    let hub = hub(&dir);
    let at = now();
    cooee_core::clock::sleep(Duration::from_millis(2));
    hub.apply(event("new", 1, EventKind::Text { text: "w".into() }));
    hub.prune(&[], at);
    let ids: Vec<String> = hub.list().into_iter().map(|r| r.id).collect();
    assert_eq!(ids.len(), 2, "{ids:?}");
    assert!(ids.contains(&"mine".to_owned()) && ids.contains(&"new".to_owned()));
    assert!(!threads.join("gone.json").exists());
}

#[test]
fn an_idle_threads_chat_unloads_and_reloads_intact() {
    let dir = scratch("unload");
    let item = |id: u64, text: &str| {
        json!({ "id": id, "at": id, "seq": id, "kind": "agent", "text": text, "stop_reason": "end_turn" })
            .to_string()
            + "\n"
    };
    seed(&dir, "a", &(item(1, "one") + &item(2, "two")));
    let hub = hub(&dir);
    let unloaded = |hub: &Hub| lock(&lock(&hub.threads)["a"]).unloaded.is_some();
    let before = hub.slice("a", None, 10, 0).unwrap();
    assert!(!unloaded(&hub));
    let done = Box::new(Reported {
        state: "done".to_owned(),
        ..Reported::default()
    });
    let t0 = now();
    hub.apply(event("a", 3, EventKind::State(done.clone())));
    hub.apply(event("a", 4, EventKind::State(done)));
    let replayed = |hub: &Hub, cursor: &str| match hub.subscribe("a", cursor).unwrap().0 {
        Resume::Replay(outs) => Some(outs.iter().map(|o| o.seq).collect::<Vec<_>>()),
        Resume::Reset => None,
    };

    hub.unload(t0 + REPLAY - 1_000);
    assert!(!unloaded(&hub));
    assert_eq!(replayed(&hub, &before.cursor), Some(vec![1, 2]));

    let row = json!(hub.list());
    hub.unload(t0 + IDLE + REPLAY);
    assert!(unloaded(&hub));
    assert_eq!(json!(hub.list()), row);
    let after = hub.slice("a", None, 10, 0).unwrap();
    assert!(!unloaded(&hub));
    assert_eq!(json!(after.items), json!(before.items));
    assert_eq!(after.turn.state, "done");
    assert_eq!(replayed(&hub, &before.cursor), None);
    assert_eq!(replayed(&hub, &after.cursor), Some(vec![]));
}
