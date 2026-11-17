//! The persona panel as a core screen (#314): `GET /screens/persona/<id>` is the app's bundled panel with
//! `source: "core"`, its routines, places and memory filled from the persona's own; `POST /screens/action` saves the
//! persona and clears its memory. Core adds the thread actions (#495).

mod support;

use std::fs;

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

const BUNDLED: &str = include_str!("../../../apple/Sources/CooeeUI/Default/screens/persona.json");

fn routine(id: &str, profile: &str) -> Value {
    let who = json!({ "agent": "fake-acp", "account": "", "profile": profile, "folder": "", "model": "",
                      "effort": "", "machine": "" });
    json!({ "id": id, "name": format!("Board {id}"), "trigger": { "schedule": "30 8 * * 1" },
            "place": { "in": "new_thread", "who": who }, "prompt": "board", "paused": { "why": "owner" },
            "made_in": "t-1", "made_at": 0 })
}

fn start(name: &str) -> (FakeShed, Cooee) {
    let dir = scratch(name);
    let profiles = dir.join("profiles");
    fs::create_dir_all(&profiles).unwrap();
    fs::write(
        profiles.join("coder.toml"),
        "name = \"coder\"\nlabel = \"Coder\"\nagent = \"fake-acp\"\n",
    )
    .unwrap();
    let routines =
        json!({ "format": 1, "routines": [routine("r-1", "coder"), routine("r-2", "other")] });
    fs::create_dir_all(dir.join("data")).unwrap();
    fs::write(dir.join("data/routines.json"), routines.to_string()).unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    (shed, cooee)
}

fn block(screen: &Value, kind: &str) -> Value {
    let blocks = screen["blocks"].as_array().unwrap();
    blocks.iter().find(|b| b["type"] == kind).unwrap().clone()
}

#[test]
fn the_panel_is_the_bundled_screen_filled_with_the_personas_own() {
    let (_shed, cooee) = start("persona-screen");
    let note = json!({ "text": "Small commits" });
    assert_eq!(cooee.post("/personas/coder/memory", &note).0, 200);
    let screen = cooee.json("/screens/persona/coder");
    let bundled: Value = serde_json::from_str(BUNDLED).unwrap();
    assert_eq!(screen["source"], "core");
    assert_eq!(screen["title"], "Coder");
    let kinds = |s: &Value| {
        s["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["type"].clone())
            .collect::<Vec<_>>()
    };
    let mut served = kinds(&bundled);
    served.push(json!("actions"));
    assert_eq!(kinds(&screen), served);
    assert_eq!(
        block(&screen, "routines")["rows"],
        json!([{ "id": "r-1", "name": "Board r-1", "when": "Mondays 08:30", "on": false }])
    );
    let memory = block(&screen, "memory");
    assert_eq!(memory["count"], 1);
    assert_eq!(memory["notes"][0]["text"], "Small commits");
    assert_eq!(memory["notes"][0]["by"], "owner");
    assert_eq!(cooee.get("/screens/persona/nobody").0, 404);
}

#[test]
fn its_actions_save_the_persona_and_clear_its_memory() {
    let (_shed, cooee) = start("persona-action");
    cooee.post(
        "/personas/coder/memory",
        &json!({ "text": "Small commits" }),
    );
    let mut coder = cooee.json("/personas/coder");
    coder["character"] = json!("Terse.");
    let save = json!({ "screen": "/screens/persona/coder", "block": "", "action": "save", "values": coder });
    let (status, saved) = cooee.post("/screens/action", &save);
    assert_eq!(status, 200, "{saved}");
    assert_eq!(cooee.json("/personas/coder")["character"], "Terse.");
    let clear =
        json!({ "screen": "/screens/persona/coder", "block": "", "action": "clear", "values": {} });
    let (status, body) = cooee.post("/screens/action", &clear);
    assert_eq!(status, 200, "{body}");
    let screen: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(block(&screen, "memory")["count"], 0);
    assert_eq!(cooee.json("/personas/coder/memory"), json!([]));
    let elsewhere =
        json!({ "screen": "/screens/menu", "block": "", "action": "save", "values": {} });
    assert_eq!(cooee.post("/screens/action", &elsewhere).0, 400);
}
