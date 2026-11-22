use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use serde_json::{Value, json};

use crate::setup::{
    Agent, CHROMIUM, Mcp, PLAYWRIGHT, Setup, assert_sent, fake_page_list, frames_setup, named,
    screen_setup,
};
use crate::support::{Cooee, FakeShed, agent, scratch, version};

#[test]
fn each_persona_gets_its_own_browser_and_every_call_is_checked_and_logged() {
    let dir = scratch("mcp-browser");
    let s = two_browser_personas(&dir);
    let bin = dir.join("bin");
    let navigate = |m: &mut Mcp, url: &str| m.run("browser_navigate", &json!({ "url": url }));
    let (_, servers) = s.thread("ann");
    let mut ann = Mcp::start(named(&servers, "browser"), Agent::Claude);
    let (_, servers) = s.thread("bob");
    let mut bob = Mcp::start(named(&servers, "browser"), Agent::Codex);
    assert_eq!(
        navigate(&mut ann, "https://www.example.com/a"),
        Ok("done".to_owned())
    );
    assert_eq!(
        navigate(&mut bob, "https://github.com/"),
        Ok("done".to_owned())
    );
    let e = navigate(&mut ann, "https://github.com/").unwrap_err();
    assert!(
        e.contains("https://github.com/ isn't on ann's site allowlist"),
        "{e}"
    );
    assert!(ann.run("browser_snapshot", &json!({})).is_ok());
    drop((ann, bob));

    let data = dir.join("data/browsers");
    let launched = fs::read_to_string(bin.join("chromium.log")).unwrap();
    let mut profiles: Vec<&str> = launched.lines().collect();
    profiles.sort_unstable();
    profiles.dedup();
    let want = [data.join("ann"), data.join("bob")].map(|p| p.to_string_lossy().into_owned());
    assert_eq!(profiles, want);
    let started = fs::read_to_string(bin.join("playwright.log")).unwrap();
    assert_eq!(
        started.lines().collect::<Vec<_>>(),
        ["--cdp-endpoint http://127.0.0.1:1"; 2]
    );
    let calls = fs::read_to_string(bin.join("calls.log")).unwrap();
    assert_eq!(calls.lines().count(), 3, "{calls}");
    assert_eq!(calls.matches("https://github.com/").count(), 1, "{calls}");
    assert_each_call_is_logged_with_its_refusal(&data);
}

#[test]
fn each_browser_action_leaves_one_captioned_encrypted_frame_and_a_no_frames_site_none() {
    let dir = scratch("mcp-frames");
    let pages = Arc::new(Mutex::new(vec!["https://www.example.com/a".to_owned()]));
    let s = frames_setup(&dir, fake_page_list(Arc::clone(&pages)));
    let (id, servers) = s.thread("ann");
    let mut ann = Mcp::start(named(&servers, "browser"), Agent::Claude);
    act_on_example_then_on_a_no_frames_bank(&mut ann, &pages);
    drop(ann);

    let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let frames: Vec<(&str, &str)> = items
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "frame")
        .map(|i| (i["text"].as_str().unwrap(), i["name"].as_str().unwrap()))
        .collect();
    let captions: Vec<&str> = frames.iter().map(|f| f.0).collect();
    let asked: Vec<bool> = items
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "frame")
        .map(|i| i["asked"] == true)
        .collect();
    assert_eq!(asked, [false, false, false, true]);
    let want = [
        r#"navigate "https://www.example.com/a""#,
        r#"click "Sign in""#,
        r#"type "••••••••" into "Password""#,
        "https://www.example.com/a",
    ];
    assert_eq!(captions, want, "{items}");
    assert_frames_are_sealed_and_typing_is_masked(&s, &dir, &id, &frames);
}

#[test]
fn the_owner_watches_takes_control_and_hands_back_with_a_note_the_waiting_call_carries() {
    let (s, bin, cdp) = screen_setup("mcp-screen");
    let (_, servers) = s.thread("ann");
    let mut ann = Mcp::start(named(&servers, "browser"), Agent::Claude);
    let nav = json!({ "name": "browser_navigate", "arguments": { "url": "https://example.com/" } });
    assert_eq!(texts(&ann.call("tools/call", &nav).unwrap()), ["done"]);
    let calls = || {
        fs::read_to_string(bin.join("calls.log"))
            .unwrap()
            .lines()
            .count()
    };
    let screen = || s.cooee.json("/personas/ann/screen");
    assert_eq!(
        screen(),
        json!({ "control": { "by": "agent" }, "waiting": 0 })
    );
    assert_the_live_screen_sends_a_frame(&s);
    let click = json!({ "click": { "x": 10.0, "y": 20.0 } });
    assert_input_and_back_need_control(&s, &click);
    take_control_for_ten_minutes(&s);

    for input in [click, json!({ "text": "hunter2" })] {
        let (status, body) = s.cooee.post("/personas/ann/screen/input", &input);
        assert_eq!(status, 200, "{body}");
    }

    let waiting = thread::spawn(move || {
        let r = ann.call("tools/call", &nav).unwrap();
        (ann, r)
    });
    until_the_agents_call_waits(&s);
    assert!(!waiting.is_finished());
    assert_eq!(
        calls(),
        1,
        "the agent's call ran while the owner held control"
    );

    let (status, body) = s
        .cooee
        .post("/personas/ann/screen/back", &json!({ "note": "signed in" }));
    assert_eq!(status, 200, "{body}");
    let (mut ann, r) = waiting.join().unwrap();
    assert_eq!(
        texts(&r),
        [
            "done",
            "The owner had the browser and handed it back: signed in"
        ]
    );
    assert_eq!(calls(), 2);
    assert_eq!(
        screen(),
        json!({ "control": { "by": "agent" }, "waiting": 0 })
    );
    let nav =
        json!({ "name": "browser_navigate", "arguments": { "url": "https://example.com/b" } });
    assert_eq!(texts(&ann.call("tools/call", &nav).unwrap()), ["done"]);

    assert_sent(&cdp);
}

#[test]
fn a_lease_saved_as_a_skill_hands_the_persona_its_steps_and_no_typed_value_and_a_declined_card_saves_nothing()
 {
    let (s, _, _) = screen_setup("mcp-screen-skill");
    let (id, servers) = s.thread("ann");
    let mut ann = Mcp::start(named(&servers, "browser"), Agent::Claude);
    let (status, body) = s.cooee.post("/personas/ann/screen/control", &json!({}));
    assert_eq!(status, 200, "{body}");
    for input in [
        json!({ "click": { "x": 10.0, "y": 20.0 } }),
        json!({ "text": "hunter2" }),
    ] {
        let (status, body) = s.cooee.post("/personas/ann/screen/input", &input);
        assert_eq!(status, 200, "{body}");
    }
    let until = |b: &str| serde_json::from_str::<Value>(b).unwrap()["control"]["until"].as_u64();
    let (_, more) = s.cooee.post("/personas/ann/screen/control", &json!({}));
    assert_eq!(until(&more), until(&body).map(|u| u + 600_000), "{more}");
    let back = json!({ "note": "done it", "skill": true });
    let (status, body) = s.cooee.post("/personas/ann/screen/back", &back);
    assert_eq!(status, 200, "{body}");
    let nav = json!({ "name": "browser_navigate", "arguments": { "url": "https://example.com/" } });
    let r = ann.call("tools/call", &nav).unwrap();
    let note = r["content"][1]["text"].as_str().unwrap();
    assert_eq!(
        note,
        "The owner had the browser and handed it back: done it\n\nThe owner asks you to save what they just did as a \
         skill: draft it from these steps and propose it with your skill tool. What they typed is left out: name the \
         field, never a value.\n1. https://www.example.com \"A\": click \"Sign in\"\n2. https://www.example.com \"A\": \
         type into \"Email\""
    );

    let steps = note.split_once("value.\n").unwrap().1;
    let mut cooee = Mcp::start(named(&servers, "cooee"), Agent::Claude);
    let plan = json!({ "name": "sign-in", "description": "How I sign in.", "body": steps });
    let said = cooee.run("skill", &json!({ "propose": plan })).unwrap();
    assert!(said.starts_with("Proposed."), "{said}");
    let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let card = items
        .as_array()
        .unwrap()
        .iter()
        .rfind(|i| i["form"] == "skill")
        .unwrap()
        .clone();
    assert!(
        card["detail"]
            .as_str()
            .unwrap()
            .contains("type into \"Email\"")
    );
    let tap = json!({ "item": card["id"], "option": "decline" });
    let (status, body) = s.cooee.post(&format!("/threads/{id}/approve"), &tap);
    assert_eq!(status, 200, "{body}");
    assert!(!s.dir.join("agents/ann/skills/sign-in").exists());
    assert_eq!(s.cooee.json("/personas/ann/skills"), json!([]));
}

fn two_browser_personas(dir: &Path) -> Setup {
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    for (name, text) in [("chromium", CHROMIUM), ("playwright", PLAYWRIGHT)] {
        fs::write(bin.join(name), text).unwrap();
        fs::set_permissions(bin.join(name), fs::Permissions::from_mode(0o755)).unwrap();
    }
    for (id, site) in [("ann", "example.com"), ("bob", "github.com")] {
        let toml = format!(
            "name = \"{id}\"\nagent = \"fake-acp\"\n\n[[mcp]]\nserver = \"browser\"\n\n[[sites]]\nsite = \"{site}\"\naccount = true\n"
        );
        agent(dir, id, &toml, "");
    }
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = format!(
        "chromium = {:?}\n\n[[mcp]]\nname = \"browser\"\ncommand = {:?}\n",
        bin.join("chromium").to_string_lossy(),
        bin.join("playwright").to_string_lossy()
    );
    let cooee = Cooee::start_env(dir, &shed.dir, &extra, &[]);
    Setup {
        dir: dir.to_path_buf(),
        shed,
        cooee,
        token: String::new(),
    }
}

fn assert_each_call_is_logged_with_its_refusal(data: &Path) {
    let logged = |id: &str| -> Vec<(String, Value, bool)> {
        let text = fs::read_to_string(data.join(format!("{id}.log"))).unwrap();
        text.lines()
            .map(|l| serde_json::from_str::<Value>(l).unwrap())
            .map(|v| {
                (
                    v["tool"].as_str().unwrap().to_owned(),
                    v["arguments"].clone(),
                    v["refused"].as_bool().unwrap(),
                )
            })
            .collect()
    };
    let nav = |u: &str| ("browser_navigate".to_owned(), json!({ "url": u }), false);
    let (mut off, snap) = (
        nav("https://github.com/"),
        ("browser_snapshot".to_owned(), json!({}), false),
    );
    off.2 = true;
    assert_eq!(logged("ann"), [nav("https://www.example.com/a"), off, snap]);
    assert_eq!(logged("bob"), [nav("https://github.com/")]);
}

fn act_on_example_then_on_a_no_frames_bank(ann: &mut Mcp, pages: &Mutex<Vec<String>>) {
    let shown = [
        (
            "browser_navigate",
            json!({ "url": "https://www.example.com/a" }),
        ),
        (
            "browser_click",
            json!({ "element": "Sign in", "ref": "e1" }),
        ),
        (
            "browser_type",
            json!({ "element": "Password", "ref": "e2", "text": "hunter2" }),
        ),
        ("browser_snapshot", json!({})),
    ];
    let banked = [
        (
            "browser_navigate",
            json!({ "url": "https://bank.com/acct" }),
        ),
        (
            "browser_click",
            json!({ "element": "Transfer", "ref": "e3" }),
        ),
    ];
    for (tool, args) in &shown {
        assert_eq!(ann.run(tool, args), Ok("done".to_owned()), "{tool}");
    }
    let shot = ann.run("browser_take_screenshot", &json!({ "type": "jpeg" }));
    assert_eq!(shot, Ok("shot".to_owned()));
    *pages.lock().unwrap() = vec!["https://bank.com/acct".to_owned()];
    for (tool, args) in &banked {
        assert_eq!(ann.run(tool, args), Ok("done".to_owned()), "{tool}");
    }
}

fn assert_frames_are_sealed_and_typing_is_masked(
    s: &Setup,
    dir: &Path,
    id: &str,
    frames: &[(&str, &str)],
) {
    let stored = dir.join("data/frames").join(id);
    assert_eq!(fs::read_dir(&stored).unwrap().count(), 4);
    for (_, name) in frames {
        let served = s.cooee.get(&format!("/threads/{id}/frames/{name}"));
        assert_eq!(served, (200, "frame-pixels".to_owned()));
        let sealed = fs::read(stored.join(name)).unwrap();
        assert!(
            !sealed.windows(12).any(|w| w == b"frame-pixels"),
            "{name} is plain on disk"
        );
    }
    let bin = dir.join("bin");
    let shots = fs::read_to_string(bin.join("shots.log")).unwrap();
    assert_eq!(shots.lines().count(), 6, "{shots}");
    assert!(
        fs::read_to_string(bin.join("calls.log"))
            .unwrap()
            .contains("hunter2")
    );
    let log = fs::read_to_string(dir.join("data/browsers/ann.log")).unwrap();
    assert!(
        !log.contains("hunter2") && log.contains("••••••••"),
        "{log}"
    );
}

fn texts(r: &Value) -> Vec<String> {
    let content = r["content"].as_array().unwrap();
    content
        .iter()
        .map(|c| c["text"].as_str().unwrap().to_owned())
        .collect()
}

fn assert_the_live_screen_sends_a_frame(s: &Setup) {
    let mut live = s.cooee.sse("/personas/ann/screen/live", "");
    let frame = live.next();
    assert_eq!(frame.event, "frame");
    assert_eq!(
        frame.data,
        json!({ "url": "https://www.example.com/a", "title": "A", "jpeg": "anBlZw==" })
    );
    drop(live);
}

fn assert_input_and_back_need_control(s: &Setup, click: &Value) {
    let (status, body) = s.cooee.post("/personas/ann/screen/input", click);
    assert_eq!(
        (status, body.as_str()),
        (400, "take control of ann's browser first")
    );
    let (status, body) = s
        .cooee
        .post("/personas/ann/screen/back", &json!({ "note": "x" }));
    assert_eq!(
        (status, body.as_str()),
        (400, "you don't hold ann's browser")
    );
}

fn take_control_for_ten_minutes(s: &Setup) {
    let (status, body) = s.cooee.post("/personas/ann/screen/control", &json!({}));
    assert_eq!(status, 200, "{body}");
    let until = serde_json::from_str::<Value>(&body).unwrap()["control"]["until"]
        .as_u64()
        .unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let left = u128::from(until) - now;
    assert!((590_000..=600_000).contains(&left), "{left}");
}

fn until_the_agents_call_waits(s: &Setup) {
    let screen = || s.cooee.json("/personas/ann/screen");
    let mut tries = 0;
    while screen()["waiting"] != 1 {
        tries += 1;
        assert!(tries < 500, "the agent's call never waited: {}", screen());
        thread::sleep(Duration::from_millis(20));
    }
}
