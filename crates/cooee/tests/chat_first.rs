//! Chat-first (#86): a new chat sends with no folder picked and runs in a scratch folder of its own; "Use a folder…"
//! moves its later turns to a folder inside a root, the agent started again there with a hand-over.

mod support;

use serde_json::{Value, json};
use support::{Cooee, FakeShed, is_item, scratch, version};

fn run_line<'a>(log: &'a str, name: &str) -> &'a str {
    log.lines()
        .find(|l| l.contains(&format!("--name {name} ")))
        .unwrap_or_else(|| panic!("no run of {name}: {log}"))
}

#[test]
fn a_new_chat_runs_in_its_own_scratch_folder_and_a_folder_attached_takes_later_turns() {
    let dir = scratch("chat-first");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);

    // No folder in the request at all.
    let new = |rid: &str| json!({ "agent": "fake-acp", "text": "Say hello", "random_id": rid });
    let (status, body) = cooee.post("/threads", &new("a"));
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let own = dir.join("data/scratch").join(&id);
    assert!(own.is_dir(), "{}", own.display());
    let log = shed.cli_log();
    let run = run_line(&log, &format!("cooee-{id}"));
    assert!(run.contains(&format!("--cwd {} ", own.display())), "{run}");
    let cursor = cooee.json(&format!("/threads/{id}/messages"))["cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let boot = cursor.split('.').next().unwrap();
    let mut events = cooee.events(&id, &format!("{boot}.0"), false);
    events.until(|e| is_item(e, "agent"));

    // A second chat gets another scratch folder.
    let (_, body) = cooee.post("/threads", &new("b"));
    let other = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(other, id);
    assert!(dir.join("data/scratch").join(&other).is_dir());

    // A folder outside the roots is refused, and the chat stays where it was.
    let attach = |folder: &std::path::Path| {
        cooee.post(
            &format!("/threads/{id}/folder"),
            &json!({ "folder": folder }),
        )
    };
    let (status, body) = attach(&dir);
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("not inside a configured root"), "{body}");
    assert_eq!(
        cooee.json(&format!("/threads/{id}/messages"))["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let project = dir.join("root/project").canonicalize().unwrap();
    let (status, body) = attach(&project);
    assert_eq!(status, 200, "{body}");
    let note: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(note["kind"], "note");
    assert!(
        note["text"]
            .as_str()
            .unwrap()
            .ends_with("project · later turns run here"),
        "{note}"
    );
    let log = shed.cli_log();
    let moved = log
        .lines()
        .find_map(|l| {
            let name = l.split("--name ").nth(1)?.split(' ').next()?;
            name.starts_with(&format!("cooee-{id}-"))
                .then(|| name.to_owned())
        })
        .unwrap_or_else(|| panic!("no second start: {log}"));
    let run = run_line(&log, &moved);
    assert!(
        run.contains(&format!("--cwd {} ", project.display())),
        "{run}"
    );
    assert!(run.contains(&format!("--label cooee.thread={id}")), "{run}");
    // The old terminal is closed.
    assert!(!shed.calls("kill", &format!("cooee-{id}")).is_empty());

    // The next message goes to the agent in the new folder, after the hand-over; the one after that alone.
    let send = |text: &str, rid: &str| {
        let (status, body) = cooee.post(
            &format!("/threads/{id}/send"),
            &json!({ "text": text, "random_id": rid }),
        );
        assert_eq!(status, 200, "{body}");
    };
    send("and now?", "r1");
    send("again", "r2");
    let sent = shed.sent(&moved);
    assert_eq!(sent.len(), 2, "{sent:?}");
    let handover = &sent[0];
    assert!(
        handover.contains(&format!("moved from {}", own.display()))
            && handover.contains(&format!("to {}", project.display()))
            && handover.contains("Owner: Say hello")
            && handover.contains("You: Hello")
            && handover.contains("and now?"),
        "{handover}"
    );
    assert_eq!(sent[1], "\x1e{\"prompt\":\"again\"}\n");
    // Nothing more went to the old terminal.
    assert_eq!(shed.sent(&format!("cooee-{id}")).len(), 1);
    let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let you: Vec<&str> = items
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "you")
        .map(|i| i["text"].as_str().unwrap())
        .collect();
    assert_eq!(you, ["Say hello", "and now?", "again"]);
}
