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

fn start(cooee: &Cooee, rid: &str) -> String {
    let new = json!({ "agent": "fake-acp", "text": "Say hello", "random_id": rid });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"].clone();
    id.as_str().unwrap().to_owned()
}

fn moved_to(log: &str, id: &str) -> String {
    let moved = log.lines().find_map(|l| {
        let name = l.split("--name ").nth(1)?.split(' ').next()?;
        name.starts_with(&format!("cooee-{id}-"))
            .then(|| name.to_owned())
    });
    moved.unwrap_or_else(|| panic!("no second start: {log}"))
}

#[test]
fn a_new_chat_runs_in_its_own_scratch_folder_and_a_folder_attached_takes_later_turns() {
    let dir = scratch("chat-first");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);

    let id = start(&cooee, "a");
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

    let other = start(&cooee, "b");
    assert_ne!(other, id);
    assert!(dir.join("data/scratch").join(&other).is_dir());

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
    let moved = moved_to(&log, &id);
    let run = run_line(&log, &moved);
    assert!(
        run.contains(&format!("--cwd {} ", project.display())),
        "{run}"
    );
    assert!(run.contains(&format!("--label cooee.thread={id}")), "{run}");
    assert_eq!(shed.calls("kill", &format!("cooee-{id}")).len(), 1);

    let say = |text: &str, rid: &str| {
        let (status, body) = cooee.post(
            &format!("/threads/{id}/send"),
            &json!({ "text": text, "random_id": rid }),
        );
        assert_eq!(status, 200, "{body}");
    };
    say("and now?", "r1");
    say("again", "r2");
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

#[test]
fn an_agent_has_one_ongoing_chat_and_side_threads_and_a_new_chat_needs_nothing_picked() {
    let dir = scratch("chat-first-side");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let (_, page) = cooee.get("/new?agent=fake-acp");
    assert!(
        page.contains(r#"<input type="hidden" name="agent" value="fake-acp">"#)
            && page.contains("New chat with fake-acp")
            && page.contains(r#"name="folder" value="" checked="">scratch"#)
            && page.contains(
                r#"<button type="button" class="fo" popovertarget="ufs">folder: scratch ▾</button>"#
            ),
        "{page}"
    );
    let start = |text: &str| {
        let new = json!({ "agent": "fake-acp", "text": text, "random_id": text });
        let (_, body) = cooee.post("/threads", &new);
        let id = serde_json::from_str::<Value>(&body).unwrap()["id"].clone();
        id.as_str().unwrap().to_owned()
    };
    let ongoing = start("first");
    let side = start("second");
    let (_, home) = cooee.get("/?list&agent=fake-acp");
    let nav = &home[home.find(r#"<nav class="ags""#).unwrap()..];
    assert!(
        nav.contains(&format!(r#"<a class="agr on" href="/t/{ongoing}""#))
            && nav.contains(&format!(
                r#"href="/t/{side}" data-id="{side}"><span class="tt"><i class="ur" aria-hidden="true"></i>second</span>"#
            ))
            && nav.contains(r#"<a class="str nw" href="/new?agent=fake-acp">+ new chat</a>"#),
        "{nav}"
    );
    let (_, page) = cooee.get(&format!("/t/{side}"));
    let nav = &page[page.find(r#"<nav class="ags""#).unwrap()..];
    assert!(
        nav.contains(&format!(
            r#"<a class="str on" href="/t/{side}" data-id="{side}"><span class="tt"><i class="ur" aria-hidden="true"></i>second</span>"#
        )) && nav.contains(r#"<a class="str nw" href="/new?agent=fake-acp">+ new chat</a>"#),
        "{nav}"
    );
    assert!(
        page.contains(">second</button></h1>") && page.contains("fake-acp · Plain · fake · "),
        "{page}"
    );
    assert!(
        page.contains(r#"<span id="bf">scratch</span> · "#),
        "{page}"
    );
}

#[test]
fn a_new_agent_is_saved_as_a_profile_and_listed() {
    let dir = scratch("chat-first-agent");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let form = [
        ("Content-Type", "application/x-www-form-urlencoded"),
        ("Cookie", cooee.cookie.as_str()),
    ];
    let (head, _) = cooee.raw("POST", "/agents", &form, "name=Account+Manager&profile=");
    assert!(
        head.contains(" 303") && head.contains("/new?profile=account-manager"),
        "{head}"
    );
    let file = dir.join("profiles/account-manager.toml");
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "name = \"account-manager\"\nlabel = \"Account Manager\"\nagent = \"fake-acp\"\n"
    );
    let (_, page) = cooee.get("/new?profile=account-manager");
    assert!(
        page.contains(r#"<span class="tt">Account Manager</span>"#)
            && page.contains(r#"<i class="av" style="--av:var(--link)" aria-hidden="true">AM</i>"#)
            && page.contains(r#"<input type="hidden" name="profile" value="account-manager">"#),
        "{page}"
    );
    let (head, body) = cooee.raw("POST", "/agents", &form, "name=account+manager");
    assert!(
        head.contains(" 400") && body.contains("already"),
        "{head}{body}"
    );
}
