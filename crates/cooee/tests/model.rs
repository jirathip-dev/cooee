//! #553 row 34 end to end: a thread starts on a picked model and effort (the provider row's words: a `NAME=value`
//! word before the agent, the rest after it); the composer's chips change them, kept on the thread and said as a
//! line, and a pick the provider doesn't offer is refused.

mod support;

use std::fs;

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

#[test]
fn row_34_a_thread_starts_on_its_pick_and_the_chips_change_it() {
    let dir = scratch("model-34");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = "[[providers]]\nname = \"fake\"\nagent = \"fake-acp\"\nlogin = [\"true\"]\nstatus = [\"true\"]\n\
                 models = [\"opus\", \"sonnet\"]\nmodel = [\"FAKE_MODEL={}\"]\n\
                 efforts = [\"low\", \"high\"]\neffort = [\"--effort\", \"{}\"]\n";
    let cooee = Cooee::start_with(&dir, &shed.dir, extra);
    let root = dir.join("root");
    fs::create_dir_all(&root).unwrap();
    let start = |model: &str, random: &str| {
        let new = json!({ "agent": "fake-acp", "folder": root, "text": "hi", "random_id": random, "model": model, "effort": "high" });
        cooee.post("/threads", &new)
    };
    assert_eq!(start("gpt", "r0").0, 400, "a model it doesn't offer");
    let (code, body) = start("opus", "r1");
    assert_eq!(code, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let log = shed.cli_log();
    let line = log.lines().rfind(|l| l.contains(" run ")).unwrap();
    assert!(
        line.ends_with(" -- env FAKE_MODEL=opus fake-acp --effort high"),
        "{line}"
    );
    let page = cooee.get(&format!("/t/{id}")).1;
    assert!(
        page.contains("fake-acp · System default · opus · "),
        "{page}"
    );
    assert!(
        page.contains(r#"<option value="opus" selected="">model: opus</option>"#),
        "{page}"
    );

    let path = format!("/threads/{id}/model");
    assert_eq!(cooee.post(&path, &json!({ "effort": "max" })).0, 400);
    let (code, body) = cooee.post(&path, &json!({ "model": "sonnet", "effort": "low" }));
    assert_eq!(code, 200, "{body}");
    let note: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        (&note["kind"], &note["text"]),
        (
            &json!("note"),
            &json!("model sonnet · effort low from the next start")
        )
    );
    let page = cooee.get(&format!("/t/{id}")).1;
    assert!(
        page.contains(r#"<option value="low" selected="">effort: low</option>"#),
        "{page}"
    );
    let stored: Value = serde_json::from_str(
        &fs::read_to_string(dir.join(format!("data/threads/{id}.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(
        (&stored["thread"]["model"], &stored["thread"]["effort"]),
        (&json!("sonnet"), &json!("low"))
    );
}
