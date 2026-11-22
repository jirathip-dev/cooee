use std::fs;

use serde_json::{Value, json};

use crate::setup::{Agent, Mcp, desk_setup, named};
use crate::support::scratch;

#[test]
fn a_persona_with_no_desk_or_a_desk_the_provider_did_not_make_is_refused_and_nothing_runs() {
    let dir = scratch("mcp-desk-no");
    let s = desk_setup(&dir);
    let bin = dir.join("bin");
    for (who, why) in [("bob", "no desk hand-made"), ("cy", "cy has no desk")] {
        let (_, servers) = s.thread(who);
        let mut m = Mcp::start(named(&servers, "cooee"), Agent::Codex);
        let e = m
            .run("desk", &json!({ "click": { "x": 1, "y": 2 } }))
            .unwrap();
        assert_eq!(e, format!("NO_DESK: {why}"));
    }
    assert!(!bin.join("desk.log").exists(), "a refused desk ran nothing");
}

#[test]
fn each_desk_action_leaves_one_captioned_encrypted_frame_and_only_an_asked_screenshot_reaches_the_agent()
 {
    let dir = scratch("mcp-desk");
    let s = desk_setup(&dir);
    let bin = dir.join("bin");
    let (id, servers) = s.thread("ann");
    let mut ann = Mcp::start(named(&servers, "cooee"), Agent::Claude);
    assert!(ann.tools().contains(&"desk".to_owned()));
    let act = |m: &mut Mcp, args: Value| {
        let r = m
            .call("tools/call", &json!({ "name": "desk", "arguments": args }))
            .unwrap();
        r["content"].as_array().unwrap().clone()
    };
    let clicked = act(&mut ann, json!({ "click": { "x": 120, "y": 340 } }));
    assert_eq!(
        clicked,
        [json!({ "type": "text", "text": "click 120,340 on ann-desk: done" })]
    );
    let typed = act(&mut ann, json!({ "type": { "text": "hello" } }));
    assert_eq!(typed.len(), 1, "{typed:?}");
    let shot = act(&mut ann, json!({ "screenshot": {} }));
    assert_eq!(
        shot[1],
        json!({ "type": "image", "data": "ZGVzay1waXhlbHM=", "mimeType": "image/png" })
    );
    drop(ann);

    let ran = fs::read_to_string(bin.join("desk.log")).unwrap();
    let on = "exec ann-desk env DISPLAY=:1";
    let import = format!("{on} import -window root png:-");
    let want = [
        format!("{on} xdotool mousemove 120 340 click 1"),
        import.clone(),
        format!("{on} xdotool type -- hello"),
        import.clone(),
        import,
    ];
    assert_eq!(ran.lines().collect::<Vec<_>>(), want);
    let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let frames: Vec<(String, bool, String)> = items
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "frame")
        .map(|i| {
            (
                i["text"].as_str().unwrap().to_owned(),
                i["asked"] == true,
                i["name"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let captions: Vec<(&str, bool)> = frames.iter().map(|f| (f.0.as_str(), f.1)).collect();
    assert_eq!(
        captions,
        [
            ("click 120,340", false),
            ("type \"hello\"", false),
            ("screenshot", true)
        ]
    );
    let stored = dir.join("data/frames").join(&id);
    for (_, _, name) in &frames {
        let served = s.cooee.get(&format!("/threads/{id}/frames/{name}"));
        assert_eq!(served, (200, "desk-pixels".to_owned()));
        let sealed = fs::read(stored.join(name)).unwrap();
        assert!(
            !sealed.windows(11).any(|w| w == b"desk-pixels"),
            "{name} is plain on disk"
        );
    }
    let log = fs::read_to_string(dir.join("data/browsers/ann.log")).unwrap();
    let logged: Vec<(String, String)> = log
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .map(|v| {
            (
                v["desk"].as_str().unwrap().to_owned(),
                v["tool"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let desk = |t: &str| ("ann-desk".to_owned(), t.to_owned());
    assert_eq!(
        logged,
        [
            desk("click 120,340"),
            desk("type \"hello\""),
            desk("screenshot")
        ]
    );
}
