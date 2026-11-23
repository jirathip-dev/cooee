//! Core's menu and settings screens as blocks (#312): `GET /screens/menu` is the app's bundled menu with `source:
//! "core"` and its server rows valued from the JSON routes' data; each `GET /screens/settings/<id>` is one list;
//! `GET /screens/plugins/<name>/attach` is one `permissions` card of what an installed plugin may do.

mod support;

use std::{fs, os::unix::fs::PermissionsExt};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, snapshot, version};

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

const PROVIDERS: &str = "[[providers]]\nname = \"fake\"\nagent = \"fake-acp\"\nlogin = [\"fake\"]\n\
                         status = [\"fake\"]\n\n[[providers]]\nname = \"fake-api\"\nagent = \"fake-api-acp\"\n\
                         login = [\"fake\"]\nstatus = [\"fake\"]\n";

fn signins(name: &str) -> (FakeShed, Cooee, String) {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let window =
        |id: &str, pct: u32, resets: u64| json!({ "id": id, "used_pct": pct, "resets_at": resets });
    let account = |provider: &str, name: &str, usage: Value| {
        json!({ "provider": provider, "name": name, "state": "ready", "who": format!("{name}@example.com"),
                "line": "", "usage": usage })
    };
    let usage = |windows: Value| json!({ "windows": windows, "read_at": 1_799_999_880u64 });
    shed.put_account(account(
        "fake",
        "work",
        usage(json!([
            window("5h", 84, 1_800_004_320),
            window("7d", 41, 1_800_300_000)
        ])),
    ));
    shed.put_account(account(
        "fake",
        "jk",
        usage(json!([window("7d", 36, 1_800_300_000)])),
    ));
    shed.put_account(account("fake", "plain", Value::Null));
    let mut api = account("fake-api", "ci", Value::Null);
    api["token"] = json!({ "saved_at": 1 });
    shed.put_account(api);
    let write = |file: &str, v: Value| fs::write(shed.dir.join(file), v.to_string()).unwrap();
    write(
        "machines.json",
        json!([{ "name": "air", "target": "air.local" }, { "name": "boat-1", "target": "boat-1.local" }]),
    );
    let mut limited = account(
        "fake",
        "air",
        usage(json!([window("7d", 100, 1_800_300_000)])),
    );
    limited["state"] = json!("limited");
    limited["limited_until"] = json!(1_800_008_040u64);
    write("accounts-air.json", json!([limited]));
    let login = shed.dir.join("login-fake");
    fs::write(&login, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&login, fs::Permissions::from_mode(0o755)).unwrap();
    let cooee = Cooee::start_with(&dir, &shed.dir, PROVIDERS);
    let host = cooee.json("/machines")[0]["host"]
        .as_str()
        .unwrap()
        .to_owned();
    (shed, cooee, host)
}

fn row(id: &str, title: &str, subtitle: &Value) -> Value {
    json!({ "type": "row", "id": id, "title": title, "subtitle": subtitle, "accessory": "chevron",
            "tap": { "sheet": format!("/screens/settings/signins/{id}") } })
}

#[test]
fn sign_ins_group_accounts_by_machine_with_a_figure_per_usage_window() {
    let (_shed, cooee, host) = signins("signins-grouped");
    let screen = cooee.json("/screens/settings/signins");
    assert_eq!(
        (&screen["id"], &screen["title"]),
        (&json!("signins"), &json!("Sign-ins"))
    );
    let sections = screen["blocks"].as_array().unwrap();
    let headers: Vec<_> = sections.iter().map(|s| s["header"].clone()).collect();
    assert_eq!(
        headers,
        [
            json!(format!("{host} · this machine")),
            json!("air"),
            json!("boat-1")
        ]
    );
    let ok = json!({ "text": "signed in", "tone": "ok" });
    let rows = |i: usize| sections[i]["blocks"][0]["rows"].clone();
    let mut two = row(&format!("{host}/fake/work"), "fake · work@example.com", &ok);
    two["badge"] = json!({ "text": "5h 84%", "tone": "warn" });
    two["value"] = json!("7d 41%");
    let mut one = row(&format!("{host}/fake/jk"), "fake · jk@example.com", &ok);
    one["value"] = json!("7d 36%");
    let none = row(
        &format!("{host}/fake/plain"),
        "fake · plain@example.com",
        &ok,
    );
    let api = row(
        &format!("{host}/fake-api/ci"),
        "fake-api · ci@example.com",
        &ok,
    );
    let mut local = rows(0).as_array().unwrap().clone();
    local.sort_by_key(|r| r["id"].as_str().unwrap().to_owned());
    assert_eq!(local, [api, one, none, two]);
    let mut limited = row(
        "air/fake/air",
        "fake · air@example.com",
        &json!({ "text": { "until": "2027-01-15T10:14:00Z", "label": "at its limit · resets in {rel}" },
                "tone": "warn" }),
    );
    limited["badge"] = json!({ "text": "7d 100%", "tone": "warn" });
    assert_eq!(rows(1), json!([limited]));
    assert_eq!(
        rows(2),
        json!([{ "type": "row", "id": "boat-1/offline", "title": "offline" }])
    );
}

#[test]
fn an_accounts_detail_lists_its_windows_and_its_two_buttons() {
    let (_shed, cooee, host) = signins("signins-detail");
    let screen = cooee.json(&format!("/screens/settings/signins/{host}/fake/work"));
    assert_eq!(
        (&screen["title"], &screen["back"]),
        (&json!("fake · work@example.com"), &json!("Sign-ins"))
    );
    let blocks = screen["blocks"].as_array().unwrap();
    assert_eq!(blocks[0]["rows"][1]["value"], json!(host));
    assert_eq!(
        blocks[1]["blocks"][0]["rows"],
        json!([
            { "type": "row", "id": "w/5h", "title": "5h", "badge": { "text": "84%", "tone": "warn" },
              "subtitle": { "text": { "until": "2027-01-15T09:12:00Z", "label": "resets in {rel}" } } },
            { "type": "row", "id": "w/7d", "title": "7d", "badge": { "text": "41%" },
              "subtitle": { "text": { "until": "2027-01-18T19:20:00Z", "label": "resets in {rel}" } } },
            { "type": "row", "id": "read", "title": "Read by shed",
              "value": { "since": "2027-01-15T07:58:00Z", "label": "{} ago" } }
        ])
    );
    let out = &blocks[2]["blocks"][1];
    assert_eq!(
        (&out["action"], &out["confirm"]["destructive"]),
        (&json!("signin.out"), &json!(true))
    );
    let api = cooee.json(&format!("/screens/settings/signins/{host}/fake-api/ci"));
    let ids: Vec<_> = api["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["id"].clone())
        .collect();
    assert_eq!(ids, [json!("who"), json!("acts")]);
    assert_eq!(cooee.get("/screens/settings/signins/boat-1/fake/x").0, 400);
}

#[test]
fn an_account_signs_in_again_or_out_through_its_machines_shed() {
    let (shed, cooee, host) = signins("signins-act");
    let act = |m: &str, action: &str, name: &str| {
        let values = json!({ "machine": m, "provider": "fake", "account": name });
        let screen = format!("/screens/settings/signins/{m}/fake/{name}");
        cooee.post(
            "/screens/action",
            &json!({ "screen": screen, "action": action, "values": values }),
        )
    };
    let (status, body) = act(&host, "signin.out", "jk");
    assert_eq!(
        (
            status,
            serde_json::from_str::<Value>(&body).unwrap()["close"].clone()
        ),
        (200, json!(true))
    );
    assert!(shed.account_of("fake", "jk").is_none());
    assert_eq!(act(&host, "signin.again", "work").0, 200);
    assert_eq!(shed.calls("account.login", "work").len(), 1);
    assert_eq!(act("air", "signin.out", "air").0, 200);
    assert_eq!(act("air", "signin.again", "air").0, 200);
    let log = shed.cli_log();
    for verb in ["rm", "login"] {
        assert!(
            log.contains(&format!(
                "--on air exec --timeout 10 -- shed --json account {verb} fake air"
            )),
            "{log}"
        );
    }
    assert_eq!(act("boat-1", "signin.out", "x").0, 400);
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
    assert_the_cards_places_actions_and_hosts(blocks);

    assert_eq!(
        cooee.get("/screens/plugins/nope/attach?persona=coder").0,
        404
    );
    assert_eq!(
        cooee.get("/screens/plugins/drover/attach?persona=nobody").0,
        404
    );
}

fn assert_the_cards_places_actions_and_hosts(blocks: &[Value]) {
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
}

#[test]
fn each_core_screen_is_on_the_wire_as_staging_served_it() {
    let (_shed, cooee) = start("screen-snapshots");
    let device = cooee.json("/devices")[0]["id"].as_str().unwrap().to_owned();
    let host = cooee.json("/machines")[0]["host"]
        .as_str()
        .unwrap()
        .to_owned();
    let varying = [(device.as_str(), "<device>"), (host.as_str(), "<host>")];
    for (name, path) in [
        ("menu", "/screens/menu"),
        ("devices", "/screens/settings/devices"),
        ("signins", "/screens/settings/signins"),
        ("machines", "/screens/settings/machines"),
        ("plugins", "/screens/settings/plugins"),
        ("attach", "/screens/plugins/drover/attach?persona=coder"),
        ("reset-layout", "/screens/reset-layout"),
        ("pairing", "/screens/pairing"),
        ("home", "/screens/home"),
        ("persona", "/screens/persona/coder"),
    ] {
        snapshot(name, &cooee.json(path), &varying);
    }
}
