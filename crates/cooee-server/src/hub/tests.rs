use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

use cooee_core::shed_proto::{ApprovalOption, Event, EventKind, Reported};
use cooee_core::{Body, Chat, Config, Shed, Thread};
use serde_json::json;
use tokio::sync::broadcast;

use super::live::{IDLE, Live, REPLAY};
use super::*;

pub(super) fn hub(dir: &Path) -> Arc<Hub> {
    hub_on(dir, &dir.join("no-shed-dir"))
}

fn hub_on(dir: &Path, shed_dir: &Path) -> Arc<Hub> {
    let shed = Shed::new(Some(shed_dir));
    let (looks, _) = crate::look::Looks::load(None, None);
    let theme = looks.themes[&looks.default].clone();
    Hub::open(Seed {
        shed,
        providers: cooee_core::Providers::default(),
        data: &dir.join("data"),
        config: &Config::default(),
        theme,
        plugins: Vec::new(),
    })
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
    use cooee_core::shed_proto::{ErrorCode, Method, RpcError};
    let dir = scratch("agents");
    let server = shed_client::fake::Server::start().unwrap();
    let installed = json!({ "id": "a", "version": "1", "distribution": "npx", "cmd": [] });
    server.answer(
        Method::Agents,
        json!([{ "id": "a", "installed": installed }]),
    );
    let hub = hub_on(&dir, server.socket().parent().unwrap());
    let count = || {
        let seen = server.seen().into_iter();
        seen.filter(|(m, _)| *m == Method::Agents).count()
    };
    assert_eq!(hub.agents().unwrap()[0].id, "a");
    let t0 = now();
    for i in 0..10 {
        hub.refresh(t0 + i * 6_000, false);
        assert_eq!(hub.agents().unwrap()[0].id, "a");
    }
    assert_eq!(count(), 1);
    hub.refresh(t0 + AGENTS_FOR, false);
    assert_eq!(count(), 2);
    server.fail(Method::Agents, RpcError::new(ErrorCode::Internal, "down"));
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
fn a_thread_opened_right_after_start_has_its_items_and_its_row_matches() {
    let dir = scratch("lazy");
    let option = |id: &str| json!({ "id": id, "name": id, "kind": "allow_once" });
    let lines = [
        json!({ "id": 1, "at": 1, "seq": 1, "kind": "you", "text": "go", "random_id": "r1" }),
        json!({ "id": 2, "at": 2, "seq": 2, "kind": "agent", "text": "ok", "stop_reason": "end_turn" }),
        json!({ "id": 3, "at": 3, "seq": 3, "kind": "you", "text": "more", "random_id": "r2" }),
        json!({ "id": 4, "at": 4, "seq": 4, "kind": "approval", "tool": "Edit a.rs", "options": [option("allow")] }),
        json!({ "id": 5, "at": 5, "seq": 5, "kind": "tool", "call": "c", "title": "ls", "status": "completed" }),
        json!({ "id": 6, "at": 6, "seq": 6, "kind": "agent", "text": "first\nlast line", "stop_reason": "end_turn" }),
    ];
    let jsonl = lines.map(|l| l.to_string() + "\n").concat();
    seed(&dir, "a", &jsonl);
    let meta = json!({ "format": 1, "thread": { "id": "a", "title": "a", "messages": [], "terminal": "cooee-a", "read": 2 } });
    fs::write(dir.join("data/threads/a.json"), meta.to_string()).unwrap();
    let hub = hub(&dir);
    let unloaded = |hub: &Hub| lock(&lock(&hub.threads)["a"]).unloaded.is_some();
    assert!(unloaded(&hub));
    let row = json!(hub.list());
    assert_eq!(row[0]["unseen"], 3, "{row}");
    let slice = hub.slice("a", None, 100, 0).unwrap();
    assert!(!unloaded(&hub));
    let ids: Vec<u64> = slice.items.iter().map(|i| i.id).collect();
    assert_eq!(ids, [1, 2, 3, 4, 5, 6]);
    assert_eq!(json!(hub.list()), row);
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
        Resume::Replay(outs) => Some(outs.iter().map(|o| o.rev).collect::<Vec<_>>()),
        Resume::Reset => None,
    };

    hub.unload(t0 + REPLAY - 1_000);
    assert!(!unloaded(&hub));
    let revs = replayed(&hub, &before.cursor).unwrap();
    assert!(revs.len() == 2 && revs[0] < revs[1], "{revs:?}");

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

#[test]
fn a_reply_ends_the_rows_turn() {
    let dir = scratch("reply-ends");
    seed(&dir, "t", "");
    let hub = hub(&dir);
    let working = Reported {
        state: "working".to_owned(),
        ..Reported::default()
    };
    hub.apply(event("t", 1, EventKind::State(Box::new(working))));
    assert_eq!(hub.list()[0].state, "working");
    let reply = EventKind::Reply {
        text: "Done.".to_owned(),
        stop_reason: "end_turn".to_owned(),
    };
    hub.apply(event("t", 2, reply));
    assert_eq!(hub.list()[0].state, "done");
}

#[test]
fn a_thread_with_no_messages_sends_no_age() {
    let dir = scratch("no-age");
    seed(&dir, "empty", "");
    let hub = hub(&dir);
    let rows = json!(hub.list());
    assert_eq!(rows[0]["id"], "empty");
    assert!(rows[0].get("at").is_none(), "{rows}");
}

#[test]
fn a_busy_thread_does_not_hold_up_another_threads_event() {
    let call = |n: u64, status: &str| EventKind::ToolCall {
        id: format!("call-{}", n / 3),
        title: "ls".to_owned(),
        status: status.to_owned(),
        kind: None,
        locations: Vec::new(),
        raw_input: None,
    };
    let round = |r: u64| {
        let dir = scratch(&format!("head-of-line-{r}"));
        seed(&dir, "busy", "");
        seed(&dir, "quiet", "");
        let hub = hub(&dir);
        hub.agents().ok();
        let at = cooee_core::clock::instant();
        for n in 0..1000 {
            let status = ["pending", "in_progress", "completed"][usize::try_from(n % 3).unwrap()];
            hub.apply(event("busy", n + 1, call(n, status)));
        }
        hub.apply(event("quiet", 1, call(0, "pending")));
        let took = at.elapsed();
        assert_eq!(hub.slice("quiet", None, 10, 0).unwrap().items.len(), 1);
        assert_eq!(hub.slice("busy", None, 500, 0).unwrap().items.len(), 334);
        took
    };
    let took = (0..3).map(round).min().unwrap();
    assert!(took < Duration::from_millis(200), "{took:?}");
}

fn texts(rx: &mut broadcast::Receiver<Out>) -> Vec<Out> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn joined(outs: &[Out]) -> String {
    outs.iter()
        .filter(|o| o.event == "text")
        .map(|o| {
            serde_json::from_str::<serde_json::Value>(&o.data).unwrap()["text"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect()
}

#[test]
fn five_hundred_chunks_in_a_second_are_at_most_thirty_text_events() {
    let mut live = Live::new(Thread::default(), Chat::default());
    let mut rx = live.tx.insert(broadcast::channel(1024).0).subscribe();
    let t0 = 1_000_000;
    let chunks: Vec<String> = (0..500).map(|i| format!("c{i} ")).collect();
    for (at, chunk) in (t0..).step_by(2).zip(&chunks) {
        live.text(at, chunk);
    }
    live.emit("state", "{}".to_owned());
    let outs = texts(&mut rx);
    let text = outs.iter().filter(|o| o.event == "text").count();
    assert!(text <= 30, "{text} text events");
    assert_eq!(joined(&outs), chunks.concat());
    assert_eq!(outs.last().unwrap().event, "state");
}

#[test]
fn a_held_chunk_goes_out_on_the_tick() {
    let mut live = Live::new(Thread::default(), Chat::default());
    let mut rx = live.tx.insert(broadcast::channel(16).0).subscribe();
    let t0 = 1_000_000;
    assert!(!live.text(t0, "a"));
    assert!(live.text(t0 + 1, "b"));
    assert!(!live.text(t0 + 2, "c"));
    assert_eq!(live.tick(t0 + 3), Some(t0 + 40));
    assert_eq!(joined(&texts(&mut rx)), "a");
    assert_eq!(live.tick(t0 + 40), None);
    assert_eq!(joined(&texts(&mut rx)), "bc");
    assert!(live.text(t0 + 41, "d"));
}

#[test]
fn a_reply_delivers_its_text_before_its_item_and_replays_only_the_item() {
    let dir = scratch("coalesce");
    seed(&dir, "t", "");
    let hub = hub(&dir);
    let before = hub.slice("t", None, 10, 0).unwrap().cursor;
    let mut rx = hub.subscribe("t", &before).unwrap().1;
    for (seq, chunk) in (1..).zip(["Do", "n", "e."]) {
        hub.apply(event("t", seq, EventKind::Text { text: chunk.into() }));
    }
    let reply = EventKind::Reply {
        text: "Done.".to_owned(),
        stop_reason: "end_turn".to_owned(),
    };
    hub.apply(event("t", 4, reply));
    let outs = texts(&mut rx);
    let item = outs.iter().position(|o| o.event == "item").unwrap();
    assert_eq!(joined(outs.get(..item).unwrap()), "Done.");
    assert_eq!(joined(outs.get(item..).unwrap()), "");
    assert_eq!(lock(&lock(&hub.threads)["t"]).unsent, "");

    let Resume::Replay(replayed) = hub.subscribe("t", &before).unwrap().0 else {
        panic!("reset");
    };
    let events: Vec<&str> = replayed.iter().map(|o| o.event).collect();
    assert!(
        events.contains(&"item") && !events.contains(&"text"),
        "{events:?}"
    );
}

#[test]
fn the_glance_shows_a_working_thread_as_running_then_its_ask() {
    let dir = scratch("glance");
    seed(&dir, "run", "");
    let hub = hub(&dir);
    let report = |seq, state: &str| {
        let r = Reported {
            state: state.to_owned(),
            ..Reported::default()
        };
        hub.apply(event("run", seq, EventKind::State(Box::new(r))));
    };
    report(1, "working");
    let first = hub.overview();
    let running: Vec<&str> = first.running.iter().map(|r| r.thread.as_str()).collect();
    assert_eq!((running, first.asks.len()), (vec!["run"], 0));
    assert!(first.running[0].since <= first.at && first.at <= now());
    report(2, "blocked");
    let asked = hub.overview();
    assert_eq!(
        (asked.running.len(), asked.asks[0].thread.as_str()),
        (0, "run")
    );
}

#[test]
fn the_activity_counts_the_tools_of_the_current_turn_only() {
    let dir = scratch("tools");
    let lines = [
        json!({ "id": 1, "at": 1, "seq": 1, "kind": "you", "text": "go", "random_id": "r1" }),
        json!({ "id": 2, "at": 2, "seq": 2, "kind": "tool", "call": "a", "title": "ls", "status": "completed" }),
        json!({ "id": 3, "at": 3, "seq": 3, "kind": "you", "text": "more", "random_id": "r2" }),
        json!({ "id": 4, "at": 4, "seq": 4, "kind": "tool", "call": "b", "title": "ls", "status": "completed" }),
        json!({ "id": 5, "at": 5, "seq": 5, "kind": "tool", "call": "c", "title": "cat", "status": "completed" }),
    ];
    seed(&dir, "t", &lines.map(|l| l.to_string() + "\n").concat());
    let hub = hub(&dir);
    assert_eq!((hub.tools("t"), hub.tools("gone")), (2, 0));
}

#[test]
fn a_hundred_lists_with_nothing_changed_read_no_persona_file_after_the_first() {
    let dir = scratch("personas-kept");
    let toml = dir.join("config/agents/ada/agent.toml");
    fs::create_dir_all(toml.parent().unwrap()).unwrap();
    fs::write(&toml, "name = \"Ada\"\nagent = \"x\"\n").unwrap();
    let meta = json!({ "format": 1, "thread": { "id": "a", "title": "a", "messages": [], "terminal": "cooee-a", "profile": "ada" } });
    fs::write(dir.join("data/threads/a.json"), meta.to_string()).unwrap();
    fs::write(dir.join("data/threads/a.jsonl"), "").unwrap();
    let shed = Shed::new(Some(&dir.join("no-shed-dir")));
    let (looks, _) = crate::look::Looks::load(None, None);
    let theme = looks.themes[&looks.default].clone();
    let config = Config {
        path: Some(dir.join("config/config.toml")),
        approval_expiry_minutes: 0,
        ..Config::default()
    };
    let hub = Hub::open(Seed {
        shed,
        providers: cooee_core::Providers::default(),
        data: &dir.join("data"),
        config: &config,
        theme,
        plugins: Vec::new(),
    })
    .unwrap();
    let name = |hub: &Hub| hub.list()[0].persona_name.clone();
    assert_eq!(name(&hub), "Ada");
    let was = fs::metadata(&toml).unwrap().modified().unwrap();
    let stamped = |text: &str, at| {
        let new = toml.with_extension("new");
        fs::write(&new, text).unwrap();
        let file = fs::File::options().write(true).open(&new).unwrap();
        file.set_modified(at).unwrap();
        fs::rename(&new, &toml).unwrap();
    };
    stamped("name = \"Bob\"\nagent = \"x\"\n", was);
    for _ in 0..100 {
        assert_eq!(name(&hub), "Ada", "an unchanged stamp read the file again");
    }
    stamped(
        "name = \"Cyd\"\nagent = \"x\"\n",
        was + Duration::from_secs(1),
    );
    assert_eq!(name(&hub), "Cyd");
}

#[test]
fn the_list_over_200_threads_of_500_items_takes_no_threads_lock() {
    let dir = scratch("list-unlocked");
    let item = |id: u64| {
        json!({ "id": id, "at": id, "seq": id, "kind": "agent", "text": "hi", "stop_reason": "end_turn" })
            .to_string()
            + "\n"
    };
    let items: String = (1..=500).map(item).collect();
    for t in 0..200 {
        seed(&dir, &format!("t{t}"), &items);
    }
    let hub = hub(&dir);
    hub.slice("t0", None, 10, 0).unwrap();
    let held: Vec<_> = hub.all();
    let guards: Vec<_> = held.iter().map(|l| lock(l)).collect();
    let (tx, rx) = mpsc::channel();
    let lister = Arc::clone(&hub);
    thread::spawn(move || tx.send(lister.list()).unwrap());
    let rows = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("list waited on a thread's lock");
    drop(guards);
    assert_eq!(rows.len(), 200);
    assert!(rows.iter().all(|r| r.at == Some(500) && r.unseen == 500));
}

#[test]
fn an_approval_in_a_background_thread_tells_home_at_once() {
    let dir = scratch("home-told");
    seed(&dir, "bg", "");
    seed(&dir, "fg", "");
    let hub = hub(&dir);
    hub.personas();
    let mut home = hub.home();
    hub.apply(event("bg", 1, EventKind::Text { text: "w".into() }));
    while home.try_recv().is_ok() {}
    hub.apply(event("bg", 2, EventKind::Text { text: "w".into() }));
    assert!(home.try_recv().is_err(), "an unchanged row told home");
    let option = ApprovalOption {
        id: "once".into(),
        name: "Allow once".into(),
        kind: "allow_once".into(),
    };
    let (tool, options) = ("Edit a.rs".to_owned(), vec![option]);
    hub.apply(event(
        "bg",
        3,
        EventKind::ApprovalRequest {
            id: String::new(),
            tool,
            options,
            call: None,
        },
    ));
    assert!(home.try_recv().is_ok(), "the approval did not tell home");
    let row = hub.list().into_iter().find(|r| r.id == "bg").unwrap();
    assert_eq!((row.last.as_str(), row.unseen), ("Edit a.rs", 1));
}
