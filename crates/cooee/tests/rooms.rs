//! Rooms (#154) end to end against the fake shed: a room of personas is made, members are added and taken out (a
//! taken-out member's session is closed), each member's own session starts on its first turn and is given the
//! others' final replies, attributed, and nothing else (G3); at most 5 members (G6); the conductor joins, a lane
//! thread doesn't (G5). The turn engine (#155): mentions first in mention order, else every member in room order and
//! a pass hidden (G1), mention chaining up to 6 turns a message, with no loop (G2), and [stop].

mod support;

use std::{fs, path::Path, thread, time::Duration};

use cooee_core::personas::PLAIN;
use serde_json::{Value, json};
use support::{Cooee, Events, FakeShed, scratch, version};

/// cooee with personas Ada, Bo, Cy, Di and Ed, and the conductor profile (read as a persona).
fn setup(name: &str) -> (std::path::PathBuf, FakeShed, Cooee) {
    let dir = scratch(name);
    fs::create_dir_all(dir.join("personas")).unwrap();
    for n in ["Ada", "Bo", "Cy", "Di", "Ed"] {
        let file = dir.join(format!("personas/{}.toml", n.to_lowercase()));
        fs::write(file, format!("name = \"{n}\"\nagent = \"fake-acp\"\n")).unwrap();
    }
    fs::create_dir_all(dir.join("profiles")).unwrap();
    fs::write(
        dir.join("profiles/conductor.toml"),
        "name = \"conductor\"\nlabel = \"Conductor\"\nagent = \"fake-acp\"\n",
    )
    .unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    (dir, shed, cooee)
}

fn post(cooee: &Cooee, path: &str, body: &Value) -> (u16, Value) {
    let (status, text) = cooee.post(path, body);
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

fn room(cooee: &Cooee, members: &[&str]) -> String {
    let new = json!({ "members": members, "random_id": format!("room-{members:?}") });
    let (status, body) = post(cooee, "/rooms", &new);
    assert_eq!(status, 200, "{body}");
    body["id"].as_str().unwrap().to_owned()
}

/// Polls `f` for up to 10 s.
fn eventually(mut f: impl FnMut() -> bool) {
    for _ in 0..200 {
        if f() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("never happened");
}

/// The room's items once `n` member replies are in.
fn replies(cooee: &Cooee, id: &str, n: usize) -> Vec<Value> {
    let mut items = Vec::new();
    eventually(|| {
        let all = cooee.json(&format!("/threads/{id}/messages"));
        items.clone_from(all["items"].as_array().unwrap());
        items.iter().filter(|i| i["kind"] == "agent").count() == n
    });
    items
}

/// The `shed run` line of member `persona`'s session, if it started.
fn run_line(dir: &Path, persona: &str) -> Option<String> {
    let log = fs::read_to_string(dir.join("shed/cli.log")).unwrap_or_default();
    let member = format!("cooee.member={persona}");
    log.lines().find(|l| l.contains(&member)).map(str::to_owned)
}

/// Member `persona`'s session's terminal, once its session started.
fn terminal(dir: &Path, persona: &str) -> String {
    let line = run_line(dir, persona).expect("its session started");
    let t = line.split([' ', '=']).find(|w| w.starts_with("cooee-t"));
    t.unwrap().to_owned()
}

/// How many cancels member `persona`'s session was sent.
fn cancels(dir: &Path, shed: &FakeShed, persona: &str) -> usize {
    let sent = shed.sent(&terminal(dir, persona));
    sent.iter().filter(|s| s.contains("\"cancel\"")).count()
}

/// The prompts member `persona`'s session was given, in order.
fn prompts(dir: &Path, shed: &FakeShed, persona: &str) -> Vec<String> {
    let sent = shed.sent(&terminal(dir, persona));
    sent.iter()
        .map(|s| {
            let p: Value = serde_json::from_str(s.trim_start_matches('\x1e')).unwrap();
            p["prompt"].as_str().unwrap().to_owned()
        })
        .collect()
}

fn send(cooee: &Cooee, id: &str, text: &str) {
    let body = json!({ "text": text, "random_id": text });
    let (status, item) = post(cooee, &format!("/threads/{id}/send"), &body);
    assert_eq!((status, &item["speaker"]), (200, &json!("owner")), "{item}");
}

#[test]
fn each_member_takes_a_turn_in_its_own_session_and_hears_the_others_attributed() {
    let (dir, shed, cooee) = setup("room-talk");
    let id = room(&cooee, &["ada", "bo"]);
    let rows = cooee.json("/threads");
    assert_eq!(rows[0]["members"], json!(["ada", "bo"]), "{rows}");
    assert!(run_line(&dir, "ada").is_none(), "no session before a turn");

    send(&cooee, &id, "hi all");
    let items = replies(&cooee, &id, 2);
    let said: Vec<(&Value, &Value)> = items.iter().map(|i| (&i["speaker"], &i["text"])).collect();
    assert_eq!(
        said,
        [
            (&json!("owner"), &json!("hi all")),
            (&json!("ada"), &json!("Hello")),
            (&json!("bo"), &json!("Hello"))
        ]
    );
    let line = run_line(&dir, "ada").unwrap();
    assert!(line.contains(&format!("cooee.thread={id}")), "{line}");
    assert!(!prompts(&dir, &shed, "ada")[0].contains("[Bo]"));
    assert!(
        prompts(&dir, &shed, "bo")[0].ends_with("[Owner]: hi all\n\n[Ada]: Hello"),
        "{:?}",
        prompts(&dir, &shed, "bo")
    );

    // The next turn gives each only what it hasn't had, never its own.
    send(&cooee, &id, "and again");
    replies(&cooee, &id, 4);
    let ada = prompts(&dir, &shed, "ada");
    assert_eq!(ada[1], "[Bo]: Hello\n\n[Owner]: and again", "{ada:?}");
    // The members' sessions are the room's, never listed apart.
    assert_eq!(cooee.json("/threads").as_array().unwrap().len(), 1);
}

#[test]
fn members_are_given_final_replies_only() {
    let (dir, shed, cooee) = setup("room-final");
    let id = room(&cooee, &["ada", "bo"]);
    send(&cooee, &id, "use a tool");
    let items = replies(&cooee, &id, 2);
    assert!(items.iter().all(|i| i["kind"] != "tool"), "{items:?}");
    let bo = prompts(&dir, &shed, "bo").remove(0);
    assert!(bo.ends_with("[Ada]: Read it."), "{bo}");
    // Ada's tool ran in her own session; Bo's was given her reply alone.
    assert!(!bo.contains("Read src/lib.rs"), "{bo}");
}

#[test]
fn members_join_and_leave_up_to_five_the_conductor_but_never_a_lane() {
    let (dir, shed, cooee) = setup("room-members");
    let six = json!({ "members": ["ada", "bo", "cy", "di", "ed", "conductor"], "random_id": "r6" });
    let (status, why) = post(&cooee, "/rooms", &six);
    assert_eq!(status, 400, "{why}");
    assert!(why.to_string().contains("2 to 5 members"), "{why}");

    let id = room(&cooee, &["ada", "bo"]);
    let path = format!("/threads/{id}/members");
    let (status, members) = post(&cooee, &path, &json!({ "persona": "conductor" }));
    assert_eq!((status, members), (200, json!(["ada", "bo", "conductor"])));
    assert_eq!(post(&cooee, &path, &json!({ "persona": "cy" })).0, 200);
    // Twice is once.
    let (_, members) = post(&cooee, &path, &json!({ "persona": "cy" }));
    assert_eq!(members, json!(["ada", "bo", "conductor", "cy"]));
    assert_eq!(post(&cooee, &path, &json!({ "persona": "di" })).0, 200);
    let (status, why) = post(&cooee, &path, &json!({ "persona": "ed" }));
    assert!(
        status == 400 && why.to_string().contains("at most 5"),
        "{why}"
    );
    assert_eq!(post(&cooee, &path, &json!({ "persona": "nobody" })).0, 400);

    // A lane thread can't join.
    let labels = [("source", "drover"), ("group", "cooee"), ("item", "#33")];
    shed.lane("drover-cooee-33-impl", &labels);
    let lane = "w-drover-cooee-33-impl";
    eventually(|| cooee.json("/threads").to_string().contains(lane));
    let (status, why) = post(&cooee, &path, &json!({ "persona": "ed", "remove": true }));
    assert_eq!(status, 200, "{why}");
    let (_, why) = post(&cooee, &path, &json!({ "persona": "di", "remove": true }));
    assert_eq!(why, json!(["ada", "bo", "conductor", "cy"]));
    let (status, why) = post(&cooee, &path, &json!({ "persona": lane }));
    assert!(
        status == 400 && why.to_string().contains("lane thread"),
        "{why}"
    );
    let new = json!({ "members": ["ada", lane], "random_id": "lane" });
    assert_eq!(post(&cooee, "/rooms", &new).0, 400);

    // Taken out: its session is closed, and it takes no more turns.
    send(&cooee, &id, "hi all");
    replies(&cooee, &id, 4);
    let bo = run_line(&dir, "bo").unwrap();
    let bo = bo
        .split([' ', '='])
        .find(|w| w.starts_with("cooee-t"))
        .unwrap()
        .to_owned();
    let (_, members) = post(&cooee, &path, &json!({ "persona": "bo", "remove": true }));
    assert_eq!(members, json!(["ada", "conductor", "cy"]));
    eventually(|| !shed.calls("kill", &bo).is_empty());
    send(&cooee, &id, "once more");
    replies(&cooee, &id, 7);
    assert_eq!(prompts(&dir, &shed, "bo").len(), 1);
}

/// Room `id`'s page events from its first this run.
fn events(cooee: &Cooee, id: &str) -> Events {
    let cursor = cooee.json(&format!("/threads/{id}/messages"))["cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let boot = cursor.split('.').next().unwrap();
    cooee.events(id, &format!("{boot}.0"), false)
}

/// The next `n` turns taken, as (member, reason).
fn turns(events: &mut Events, n: usize) -> Vec<(String, String)> {
    let mut out = Vec::new();
    while out.len() < n {
        let e = events.next();
        if e.event == "room-turn" {
            let s = |k: &str| e.data[k].as_str().unwrap().to_owned();
            out.push((s("member"), s("reason")));
        }
    }
    out
}

/// The room's items, as (speaker or `note`, text).
fn said(items: &[Value]) -> Vec<(String, String)> {
    let row = |i: &Value| {
        let who = if i["kind"] == "note" {
            "note"
        } else {
            i["speaker"].as_str().unwrap()
        };
        (who.to_owned(), i["text"].as_str().unwrap().to_owned())
    };
    items.iter().map(row).collect()
}

/// The room's items once `n` notes are in.
fn notes(cooee: &Cooee, id: &str, n: usize) -> Vec<Value> {
    let mut items = Vec::new();
    eventually(|| {
        let all = cooee.json(&format!("/threads/{id}/messages"));
        items.clone_from(all["items"].as_array().unwrap());
        items.iter().filter(|i| i["kind"] == "note").count() == n
    });
    items
}

fn pairs(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect()
}

#[test]
fn mentioned_members_take_turns_in_mention_order_and_no_one_else() {
    let (dir, shed, cooee) = setup("room-mentions");
    let id = room(&cooee, &["ada", "bo", "cy"]);
    let mut events = events(&cooee, &id);
    send(&cooee, &id, "@cy then @Ada (and @cy again): hi");
    assert_eq!(
        turns(&mut events, 2),
        pairs(&[("cy", "mention"), ("ada", "mention")])
    );
    let items = replies(&cooee, &id, 2);
    let first = "@cy then @Ada (and @cy again): hi";
    assert_eq!(
        said(&items),
        pairs(&[("owner", first), ("cy", "Hello"), ("ada", "Hello")])
    );
    assert!(run_line(&dir, "bo").is_none(), "bo wasn't named");
    // A member's first turn tells it who is in the room and how to pass, after its Chat style's line (#242).
    let cy = prompts(&dir, &shed, "cy").remove(0);
    assert!(
        cy.starts_with(&format!(
            "{PLAIN}\n\nYou are Cy, a member of a room with the owner and Ada, Bo."
        )) && cy.contains("Reply exactly [pass]"),
        "{cy}"
    );
}

#[test]
fn with_no_mention_every_member_takes_a_turn_and_a_pass_is_hidden() {
    let (dir, shed, cooee) = setup("room-pass");
    let id = room(&cooee, &["ada", "bo", "cy"]);
    shed.cue("bo", &["[pass]"]);
    let mut events = events(&cooee, &id);
    send(&cooee, &id, "hi all");
    assert_eq!(
        turns(&mut events, 3),
        pairs(&[("ada", "round"), ("bo", "round"), ("cy", "round")])
    );
    let items = replies(&cooee, &id, 2);
    assert_eq!(
        said(&items),
        pairs(&[("owner", "hi all"), ("ada", "Hello"), ("cy", "Hello")])
    );
    // Bo took its turn, and Cy was never given its pass.
    assert_eq!(prompts(&dir, &shed, "bo").len(), 1);
    let cy = prompts(&dir, &shed, "cy").remove(0);
    assert!(cy.ends_with("[Owner]: hi all\n\n[Ada]: Hello"), "{cy}");
}

#[test]
fn a_chain_stops_at_six_turns_on_one_message_the_last_told_to_wind_down() {
    let (dir, shed, cooee) = setup("room-cap");
    let id = room(&cooee, &["ada", "bo", "cy", "di", "ed"]);
    shed.cue("ed", &["@ada over to you"]);
    shed.cue("ada", &["Hello", "@bo your call"]);
    let mut events = events(&cooee, &id);
    send(&cooee, &id, "hi all");
    let round = ["ada", "bo", "cy", "di", "ed"].map(|m| (m, "round"));
    let mut want = pairs(&round);
    want.extend(pairs(&[("ada", "chained")]));
    assert_eq!(turns(&mut events, 6), want);
    let stop = events.until(|e| e.event == "room-stop").pop().unwrap();
    assert_eq!(stop.data, json!({ "reason": "cap" }));

    let items = notes(&cooee, &id, 1);
    let said = said(&items);
    let tail = &said[said.len() - 3..];
    let note = "6 turns on one message: no more start until your next";
    assert_eq!(
        tail,
        pairs(&[
            ("ed", "@ada over to you"),
            ("ada", "@bo your call"),
            ("note", note)
        ])
    );
    let ada = prompts(&dir, &shed, "ada");
    assert!(
        ada[1].ends_with("wind down; naming a member starts no more turns.)"),
        "{ada:?}"
    );
    assert!(!ada[0].contains("wind down"), "{ada:?}");
    assert_eq!(
        prompts(&dir, &shed, "bo").len(),
        1,
        "Bo's chained turn was dropped"
    );
}

#[test]
fn two_members_naming_each_other_stop_at_the_cap_each_message() {
    let (_dir, shed, cooee) = setup("room-loop");
    let id = room(&cooee, &["ada", "bo"]);
    shed.cue("ada", &["@bo you?"]);
    shed.cue("bo", &["@ada you?"]);
    let mut events = events(&cooee, &id);
    send(&cooee, &id, "@ada start");
    let mut want = pairs(&[("ada", "mention")]);
    for m in ["bo", "ada", "bo", "ada", "bo"] {
        want.extend(pairs(&[(m, "chained")]));
    }
    assert_eq!(turns(&mut events, 6), want);
    let items = notes(&cooee, &id, 1);
    let replies = |items: &[Value]| items.iter().filter(|i| i["kind"] == "agent").count();
    assert_eq!(replies(&items), 6);
    // The round is over: the next message starts its own 6 turns, and stops there again.
    send(&cooee, &id, "@bo again");
    let items = notes(&cooee, &id, 2);
    assert_eq!(replies(&items), 12);
    assert_eq!(items.last().unwrap()["kind"], "note");
}

#[test]
fn stop_cancels_the_member_taking_its_turn_and_drops_the_rest() {
    let (dir, shed, cooee) = setup("room-stop");
    let id = room(&cooee, &["ada", "bo"]);
    let mut events = events(&cooee, &id);
    send(&cooee, &id, "@ada @bo hang on");
    assert_eq!(turns(&mut events, 1), pairs(&[("ada", "mention")]));
    let mut terminal = String::new();
    eventually(|| {
        let line = run_line(&dir, "ada").unwrap_or_default();
        let t = line.split([' ', '=']).find(|w| w.starts_with("cooee-t"));
        terminal = t.unwrap_or_default().to_owned();
        !terminal.is_empty() && !shed.sent(&terminal).is_empty()
    });

    let (status, body) = post(&cooee, &format!("/threads/{id}/cancel"), &json!({}));
    assert_eq!(status, 200, "{body}");
    let stop = events.until(|e| e.event == "room-stop").pop().unwrap();
    assert_eq!(stop.data, json!({ "reason": "stop" }));
    // Ada's turn was cancelled: its session's cancelled reply is in, and never shown in the room.
    let session = terminal.trim_start_matches("cooee-").to_owned();
    eventually(|| {
        let all = cooee.json(&format!("/threads/{session}/messages"));
        all["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["stop_reason"] == "cancelled")
    });
    assert!(
        run_line(&dir, "bo").is_none(),
        "Bo's queued turn was dropped"
    );

    // The room takes the next message as usual (Bo hears the first, so is cued past its `hang`).
    shed.cue("bo", &["Hello"]);
    send(&cooee, &id, "@bo go");
    let items = replies(&cooee, &id, 1);
    assert_eq!(
        said(&items),
        pairs(&[
            ("owner", "@ada @bo hang on"),
            ("owner", "@bo go"),
            ("bo", "Hello")
        ])
    );
}

/// [stop] while a member's session is starting cancels that session once it has started, and drops the rest.
#[test]
fn stop_while_a_members_session_starts_cancels_it_once_started() {
    let (dir, shed, cooee) = setup("room-stop-start");
    let id = room(&cooee, &["ada", "bo"]);
    shed.gate("hold-run", true);
    send(&cooee, &id, "hi all");
    eventually(|| run_line(&dir, "ada").is_some());
    let (status, body) = post(&cooee, &format!("/threads/{id}/cancel"), &json!({}));
    assert_eq!(status, 200, "{body}");
    shed.gate("hold-run", false);
    eventually(|| cancels(&dir, &shed, "ada") == 1);
    assert!(
        run_line(&dir, "bo").is_none(),
        "Bo's queued turn was dropped"
    );
}

/// cooee with personas Ada and Bo, sending its pushes to APNs through a fake `curl` that keeps each push's body in
/// the folder it returns, and one device's APNs token.
fn with_pushes(name: &str) -> (std::path::PathBuf, FakeShed, Cooee, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch(name);
    for n in ["Ada", "Bo"] {
        let file = dir.join(format!("personas/{}.toml", n.to_lowercase()));
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, format!("name = \"{n}\"\nagent = \"fake-acp\"\n")).unwrap();
    }
    let (bin, log) = (dir.join("bin"), dir.join("pushed"));
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&log).unwrap();
    let curl = format!(
        "#!/bin/sh\ncat >\"{}/$$.json\"\nprintf '\\n200'\n",
        log.display()
    );
    fs::write(bin.join("curl"), curl).unwrap();
    fs::set_permissions(bin.join("curl"), fs::Permissions::from_mode(0o755)).unwrap();
    let key = "MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg/FMQqoOmZU6bop6k\n\
               jwzlkZBiOA5CCW7ngc03CgGpgsKhRANCAASkJsDwiTE2wL6OIdpH5tfnsdyw5UK8\n\
               NOnlJZkFHy9kIEFLJ8uttasmC9v/MsS+Cbb3MJBa25UlVnmKe8xoXskq";
    let p = "PRIVATE KEY-----";
    fs::create_dir_all(dir.join("data")).unwrap();
    let pem = format!("-----BEGIN {p}\n{key}\n-----END {p}\n");
    fs::write(dir.join("data/apns.p8"), pem).unwrap();
    let apns = "[apns]\nkey_id = \"K\"\nteam_id = \"T\"\ntopic = \"dev.cooee.app\"\nenv = \"sandbox\"\nurl = \"https://apns.test\"\n";
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start_env(&dir, &shed.dir, apns, &[("PATH", path.as_str())]);
    let token = json!({ "token": "ab".repeat(32) });
    assert_eq!(cooee.post("/push/apns", &token).0, 200);
    (dir, shed, cooee, log)
}

/// The pushes in `log` once there are `n`: each one's title and body, sorted.
fn pushed(log: &Path, n: usize) -> Vec<(String, String)> {
    let mut got = Vec::new();
    eventually(|| {
        got = fs::read_dir(log)
            .unwrap()
            .map(|f| serde_json::from_str(&fs::read_to_string(f.unwrap().path()).unwrap()))
            .collect::<Result<Vec<Value>, _>>()
            .unwrap_or_default();
        got.len() >= n
    });
    let mut got: Vec<(String, String)> = got
        .iter()
        .map(|p| {
            let alert = &p["aps"]["alert"];
            let s = |k: &str| alert[k].as_str().unwrap().to_owned();
            (s("title"), s("body"))
        })
        .collect();
    got.sort();
    got
}

/// #158: a room pushes only when a member addresses the owner, asks or fails, each titled with the room and the
/// member; replies to each other, passes and a round's end stay quiet, and a member's ask shows on the room's row.
#[test]
fn a_room_pushes_only_when_a_member_addresses_the_owner_asks_or_fails() {
    let (dir, shed, cooee, log) = with_pushes("room-push");
    let id = room(&cooee, &["ada", "bo"]);
    shed.cue(
        "ada",
        &["Over to @Bo.", "@you can you sign the order form?"],
    );
    shed.cue("bo", &["[pass]"]);
    send(&cooee, &id, "hi all");
    replies(&cooee, &id, 1);
    send(&cooee, &id, "@ada over to you");
    replies(&cooee, &id, 2);
    let at_you = (
        "ada, bo · Ada".to_owned(),
        "needs you · Ada: @you can you sign the order form?".to_owned(),
    );
    let quiet = "a reply to each other, a pass and a turn's end push nothing";
    assert_eq!(pushed(&log, 1), std::slice::from_ref(&at_you), "{quiet}");

    let ada = terminal(&dir, "ada");
    shed.ask(&ada);
    let asks = (
        "ada, bo · Ada".to_owned(),
        "needs you · fake-acp: Edit early.rs".to_owned(),
    );
    assert_eq!(pushed(&log, 2), [at_you.clone(), asks.clone()]);
    let session = ada.trim_start_matches("cooee-");
    let items = cooee.json(&format!("/threads/{session}/messages"));
    let items = items["items"].as_array().unwrap();
    let card = items.iter().find(|i| i["kind"] == "approval");
    assert_eq!(card.unwrap()["speaker"], "ada", "the card names its member");
    let rows = cooee.json("/threads");
    let rows = rows.as_array().unwrap();
    let row = rows.iter().find(|r| r["id"] == id.as_str()).unwrap();
    let ask = (&json!("Ada: Edit early.rs waiting"), &json!(true));
    assert_eq!((&row["last"], &row["unread"]), ask, "{row}");

    shed.report(&terminal(&dir, "bo"), "error", Some("rate limited"));
    let fails = (
        "ada, bo · Bo".to_owned(),
        "failed · fake-acp: rate limited".to_owned(),
    );
    assert_eq!(pushed(&log, 3), [at_you, asks, fails]);
}
