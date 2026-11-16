//! Core's menu and settings screens as blocks (#312): `GET /screens/menu` is the app's bundled menu with `source:
//! "core"` and its server rows valued from the JSON routes' data; each `GET /screens/settings/<id>` is one list;
//! `GET /screens/plugins/<name>/attach` is one `permissions` card of what an installed plugin may do.

mod support;

use std::fs;

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

const BUNDLED: &str = include_str!("../../../apple/Sources/CooeeUI/Default/screens/menu.json");

const MANIFEST: &str = "name = \"drover\"\nversion = \"0.1.0\"\nbuild = \"true\"\nrun = \"true\"\nclaims = [\"lab\"]\n\n[[places]]\n\
                        type = \"menu-row\"\nlabel = \"Queue\"\nscreen = \"/screens/queue\"\n\n[permissions]\n\
                        places = [\"menu-row\"]\npush = true\nactions = [\"merge\", \"wave\"]\n\
                        destructive = [\"merge\"]\nnetwork = [\"api.github.com\"]\n";

fn start(name: &str) -> (FakeShed, Cooee) {
    let dir = scratch(name);
    let profiles = dir.join("profiles");
    fs::create_dir_all(&profiles).unwrap();
    fs::write(
        profiles.join("coder.toml"),
        "name = \"coder\"\nlabel = \"Coder\"\nagent = \"fake-acp\"\n",
    )
    .unwrap();
    let build = dir.join("data/plugins/drover/builds/c0ffee12345");
    fs::create_dir_all(&build).unwrap();
    fs::write(build.join("cooee-plugin.toml"), MANIFEST).unwrap();
    let lock = json!([{ "name": "drover", "url": "https://example.com/drover", "commit": "c0ffee12345",
                        "version": "0.1.0", "listed": true, "tabs": [], "port": 1, "enabled": false }]);
    fs::write(dir.join("data/plugins.lock"), lock.to_string()).unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    (shed, cooee)
}

fn ids(blocks: &Value) -> Vec<Value> {
    blocks
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["id"].clone())
        .collect()
}

#[test]
fn the_menu_is_the_bundled_screen_valued_from_the_servers_data() {
    let (_shed, cooee) = start("menu-screen");
    let menu = cooee.json("/screens/menu");
    let bundled: Value = serde_json::from_str(BUNDLED).unwrap();
    assert_eq!(menu["source"], "core");
    assert!(
        bundled.get("source").is_none(),
        "the server sets the source, not the file"
    );
    for key in ["type", "id", "title"] {
        assert_eq!(menu[key], bundled[key], "{key}");
    }
    assert_eq!(ids(&menu["blocks"]), ids(&bundled["blocks"]));
    let row = |id: &str| {
        let groups = menu["blocks"].as_array().unwrap().iter();
        let rows = groups.filter_map(|g| g["rows"].as_array()).flatten();
        rows.clone().find(|r| r["id"] == id).unwrap().clone()
    };
    let devices = cooee.json("/devices").as_array().unwrap().len();
    assert_eq!(row("devices")["value"], devices.to_string());
    assert_eq!(row("plugins")["value"], "1");
    let machines = cooee.json("/machines");
    assert_eq!(row("machines")["value"], machines[0]["host"]);
    assert!(row("signins")["value"].is_string());
    assert!(row("look").get("value").is_none());
    assert!(row("notifications").get("value").is_none());
}

#[test]
fn each_settings_screen_is_one_list_from_its_route() {
    let (_shed, cooee) = start("settings-screens");
    for (id, title) in [
        ("devices", "Devices"),
        ("signins", "Sign-ins"),
        ("machines", "Machines"),
        ("plugins", "Plugins"),
    ] {
        let screen = cooee.json(&format!("/screens/settings/{id}"));
        assert_eq!(
            (&screen["source"], &screen["id"], &screen["title"]),
            (&json!("core"), &json!(id), &json!(title)),
            "{screen}"
        );
        assert_eq!(screen["blocks"][0]["type"], "list", "{screen}");
    }
    let devices = cooee.json("/screens/settings/devices");
    let me = cooee.json("/devices");
    let me = me.as_array().unwrap().iter().find(|d| d["me"] == true);
    let rows = &devices["blocks"][0]["rows"];
    assert_eq!(
        rows.as_array().unwrap().len(),
        cooee.json("/devices").as_array().unwrap().len()
    );
    if let Some(me) = me {
        let row = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == me["id"])
            .unwrap();
        assert_eq!(row["subtitle"], "this device");
    }
    let plugins = cooee.json("/screens/settings/plugins");
    assert_eq!(
        plugins["blocks"][0]["rows"],
        json!([{ "type": "row", "id": "drover", "title": "drover", "subtitle": "0.1.0" }])
    );
}

#[test]
fn the_attach_sheet_is_one_permissions_card() {
    let (_shed, cooee) = start("attach-screen");
    let screen = cooee.json("/screens/plugins/drover/attach?persona=coder");
    assert_eq!(
        (&screen["source"], &screen["id"], &screen["title"]),
        (&json!("core"), &json!("attach"), &json!("Attach drover")),
        "{screen}"
    );
    let cards = screen["blocks"].as_array().unwrap();
    assert_eq!(cards.len(), 1, "{screen}");
    let card = &cards[0];
    assert_eq!(
        (&card["kind"], &card["id"]),
        (&json!("permissions"), &json!("permissions"))
    );
    assert_eq!(card["subtitle"], "drover 0.1.0 · c0ffee1 · cooee's index");
    let name = cooee.json("/personas/coder")["name"].clone();
    let blocks = card["blocks"].as_array().unwrap();
    let headers: Vec<_> = blocks
        .iter()
        .step_by(2)
        .map(|b| b["markdown"].clone())
        .collect();
    assert_eq!(
        headers,
        [
            json!(format!("To {}, it may", name.as_str().unwrap())),
            json!("Actions"),
            json!("Its backend reaches")
        ]
    );
    let titles = |i: usize| -> Vec<Value> {
        blocks[i]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["title"].clone())
            .collect()
    };
    assert_eq!(
        titles(1),
        [
            json!("Queue row in Plugins"),
            json!("sessions labelled source=lab in its Workflow sheet"),
            json!("notifications")
        ]
    );
    assert_eq!(titles(5), [json!("api.github.com")]);
    let actions = &blocks[3]["rows"];
    assert_eq!(actions[0]["title"], "merge");
    assert_eq!(actions[0]["style"], "destructive");
    assert_eq!(
        actions[0]["badge"],
        json!({ "text": "always confirmed first", "tone": "bad" })
    );
    assert_eq!(actions[1]["title"], "wave");
    assert!(actions[1].get("style").is_none() && actions[1].get("badge").is_none());

    assert_eq!(
        cooee.get("/screens/plugins/nope/attach?persona=coder").0,
        404
    );
    assert_eq!(
        cooee.get("/screens/plugins/drover/attach?persona=nobody").0,
        404
    );
}
