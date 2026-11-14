//! #553 row 34 and #193 end to end: a thread starts on a picked model and effort. The models are the ones the fake
//! ACP agent's session reports (no list in cooee): a short name runs on its family's newest (set with `acp.model`),
//! a model the agent adds shows in the picker and `GET /models` with no code change, a pinned model that is gone
//! falls back to its family's short name with one line, and a pick it doesn't offer is refused. The effort is the
//! provider row's words after the agent; the composer's chips change both, kept on the thread and said as a line.

mod support;

use std::{fs, path::Path};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

fn reports(shed: &FakeShed, ids: &[&str]) {
    let models: Vec<[&str; 2]> = ids.iter().map(|i| [*i, *i]).collect();
    let session = json!({ "id": "s1", "model": "fake-opus-1", "models": models });
    fs::write(shed.dir.join("session.json"), session.to_string()).unwrap();
}

fn set_on(shed: &FakeShed, id: &str) -> Option<String> {
    let calls = shed.calls("acp.model", &format!("cooee-{id}"));
    calls
        .last()
        .map(|c| c["model"].as_str().unwrap().to_owned())
}

fn setup(name: &str) -> (FakeShed, Cooee, std::path::PathBuf) {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = "[[providers]]\nname = \"fake\"\nagent = \"fake-acp\"\nlogin = [\"true\"]\nstatus = [\"true\"]\n\
                 efforts = [\"low\", \"high\"]\neffort = [\"--effort\", \"{}\"]\n";
    let cooee = Cooee::start_with(&dir, &shed.dir, extra);
    fs::create_dir_all(dir.join("root")).unwrap();
    (shed, cooee, dir)
}

fn start(cooee: &Cooee, dir: &Path, model: &str, random: &str) -> (u16, String) {
    let new = json!({ "agent": "fake-acp", "folder": dir.join("root"), "text": "hi", "random_id": random, "model": model, "effort": "high" });
    let (code, body) = cooee.post("/threads", &new);
    let id = serde_json::from_str::<Value>(&body).ok();
    let id = id.and_then(|v| Some(v["id"].as_str()?.to_owned()));
    (code, id.unwrap_or(body))
}

fn notes(cooee: &Cooee, id: &str) -> Vec<String> {
    let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let notes = items
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "note");
    notes
        .map(|i| i["text"].as_str().unwrap().to_owned())
        .collect()
}

fn stored(dir: &Path, id: &str) -> (Value, Value) {
    let file = fs::read_to_string(dir.join(format!("data/threads/{id}.json"))).unwrap();
    let t = serde_json::from_str::<Value>(&file).unwrap()["thread"].clone();
    (t["model"].clone(), t["effort"].clone())
}

#[test]
fn row_34_a_thread_starts_on_its_pick_and_a_new_model_shows() {
    let (shed, cooee, dir) = setup("model-34");
    let start = |model: &str, random: &str| start(&cooee, &dir, model, random);

    reports(&shed, &["fake-opus-1", "fake-opus-2", "fake-sonnet-1"]);
    let (code, id) = start("opus", "r1");
    assert_eq!(code, 200, "{id}");
    assert_eq!(set_on(&shed, &id).as_deref(), Some("fake-opus-2"));
    let log = shed.cli_log();
    let line = log.lines().rfind(|l| l.contains(" run ")).unwrap();
    assert_eq!(log.matches(" agents\n").count(), 1, "{log}");
    assert!(line.ends_with(" -- env fake-acp --effort high"), "{line}");
    let page = cooee.get(&format!("/t/{id}")).1;
    assert!(page.contains("fake · main login · opus · "), "{page}");
    assert!(
        page.contains(r#"<option value="fake-opus">model: fake-opus</option>"#)
            && page.contains(r#"<option value="fake-sonnet-1">model: fake-sonnet-1</option>"#),
        "{page}"
    );
    assert_eq!(start("gpt", "r0").0, 400, "a model it doesn't offer");

    reports(
        &shed,
        &["fake-opus-1", "fake-opus-2", "fake-opus-3", "fake-sonnet-1"],
    );
    let (code, id) = start("opus", "r2");
    assert_eq!(code, 200, "{id}");
    assert_eq!(set_on(&shed, &id).as_deref(), Some("fake-opus-3"));
    let page = cooee.get(&format!("/t/{id}")).1;
    assert!(
        page.contains(r#"<option value="fake-opus-3">model: fake-opus-3</option>"#),
        "{page}"
    );
    let lists = cooee.json("/models");
    assert_eq!(lists.as_array().unwrap().len(), 1, "{lists}");
    let fake = &lists[0];
    assert_eq!(
        (&fake["agent"], &fake["account"]),
        (&json!("fake-acp"), &json!(""))
    );
    assert_eq!(fake["models"][2], json!(["fake-opus-3", "fake-opus-3"]));
    assert_eq!(
        fake["short"],
        json!({ "fake-opus": "fake-opus-3", "fake-sonnet": "fake-sonnet-1" })
    );
    assert!(fake["seen"].as_u64().unwrap() > 0, "{fake}");
}

#[test]
fn a_gone_pin_falls_back_with_one_line_and_a_short_name_moves_on() {
    let (shed, cooee, dir) = setup("model-gone");
    let start = |model: &str, random: &str| start(&cooee, &dir, model, random);
    reports(
        &shed,
        &["fake-opus-1", "fake-opus-2", "fake-opus-3", "fake-sonnet-1"],
    );
    let (code, id) = start("fake-opus-3", "r2");
    assert_eq!(code, 200, "{id}");
    assert_eq!(set_on(&shed, &id).as_deref(), Some("fake-opus-3"));

    let persona =
        json!({ "name": "Pin", "agent": "fake-acp", "folder": "scratch", "model": "fake-opus-3" });
    let (code, body) = cooee.post("/personas", &persona);
    assert_eq!(code, 200, "{body}");
    reports(&shed, &["fake-opus-1", "fake-opus-2", "fake-sonnet-1"]);
    let new = json!({ "agent": "fake-acp", "profile": "pin", "text": "hi", "random_id": "r3" });
    let (code, body) = cooee.post("/threads", &new);
    assert_eq!(code, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(set_on(&shed, &id).as_deref(), Some("fake-opus-2"));
    let gone = "model fake-opus-3 is gone: now fake-opus (fake-opus-2)";
    assert_eq!(notes(&cooee, &id), [gone]);
    assert_eq!(stored(&dir, &id).0, "fake-opus");
    let memory = cooee.json("/personas/pin/memory");
    let said = memory
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["text"] == gone);
    assert_eq!(said.count(), 1, "{memory}");

    let (code, id) = start("fake-opus", "r4");
    assert_eq!(code, 200, "{id}");
    assert_eq!(set_on(&shed, &id).as_deref(), Some("fake-opus-2"));
    assert_eq!(notes(&cooee, &id), Vec::<String>::new());

    let path = format!("/threads/{id}/model");
    assert_eq!(cooee.post(&path, &json!({ "effort": "max" })).0, 400);
    assert_eq!(cooee.post(&path, &json!({ "model": "gpt" })).0, 400);
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
    assert_eq!(stored(&dir, &id), (json!("sonnet"), json!("low")));
}

#[test]
fn a_day_old_list_is_asked_again() {
    let dir = scratch("model-day");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let old = json!([{ "agent": "fake-acp", "account": "", "models": [["fake-opus-1", "fake-opus-1"]], "seen": 1 }]);
    fs::create_dir_all(dir.join("data")).unwrap();
    fs::write(dir.join("data/models.json"), old.to_string()).unwrap();
    reports(&shed, &["fake-opus-1", "fake-opus-2"]);
    let cooee = Cooee::start(&dir, &shed.dir);
    let mut lists = cooee.json("/models");
    for _ in 0..100 {
        if lists[0]["seen"] != 1 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        lists = cooee.json("/models");
    }
    assert_eq!(lists[0]["short"], json!({ "fake-opus": "fake-opus-2" }));
    let probe = shed.cli_log();
    let probe = probe
        .lines()
        .find(|l| l.contains("--name cooee-models-"))
        .unwrap();
    let name = probe
        .split("--name ")
        .nth(1)
        .unwrap()
        .split(' ')
        .next()
        .unwrap();
    assert_eq!(shed.calls("rm", name).len(), 1, "the probe is closed");
}

#[test]
fn the_agent_list_carries_each_agents_efforts() {
    let (_shed, cooee, _dir) = setup("model-agents");
    assert_eq!(
        cooee.json("/agents"),
        json!([{ "id": "fake-acp", "efforts": ["low", "high"] }])
    );
}
