//! Personas over the API (#126) with the fake shed (tests/support): one is created, edited and deleted; an agent
//! folder's launch keys are read-only over the API and it runs with them (#359); a note by the persona (its `remember`
//! tool through cooee's MCP server) and one by the owner both list and reach its next session; clearing its memory,
//! or deleting it, leaves its chats. Plugins attach to a persona and detach (#129); an installed drover goes with every
//! orchestrator persona, by its role (#159); the first start moves an old conductor profile or persona into an
//! orchestrator's folder (#359).

mod support;

use std::{
    fs,
    io::Write as _,
    path::PathBuf,
    process::{Command, Stdio},
};

use cooee_core::agents::PLAIN;
use serde_json::{Value, json};
use support::{Cooee, FakeShed, agent, scratch, version};

struct Setup {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let toml = "name = \"Coder\"\nagent = \"fake-acp\"\npolicy = \"allow\"\navatar = \"idle\"\n\
                [env]\nMODE = \"review\"\n";
    agent(&dir, "coder", toml, "Follow the house rules.\n");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    Setup { dir, shed, cooee }
}

impl Setup {
    fn call(&self, method: &str, path: &str, body: Option<&Value>) -> (u16, Value) {
        let (status, text) = self.cooee.request(method, path, body);
        (
            status,
            serde_json::from_str(&text).unwrap_or(Value::String(text)),
        )
    }

    fn start(&self, persona: &str) -> (String, String, String) {
        let random = format!("start-{}", self.shed.cli_log().len());
        let new =
            json!({ "agent": "other-acp", "profile": persona, "text": "hi", "random_id": random });
        let (status, body) = self.call("POST", "/threads", Some(&new));
        assert_eq!(status, 200, "{body}");
        let id = body["id"].as_str().unwrap().to_owned();
        let log = self.shed.cli_log();
        let line = log
            .lines()
            .rfind(|l| l.contains(" run "))
            .unwrap()
            .to_owned();
        assert!(line.contains(&format!("cooee-{id}")), "{line}");
        let on = line.split("--on ").nth(1).and_then(|r| r.split(' ').next());
        let terminal = on.map_or(format!("cooee-{id}"), |m| format!("{m}/cooee-{id}"));
        let sent = self.shed.sent(&terminal);
        let prompt: Value = serde_json::from_str(sent[0].trim_start_matches('\x1e')).unwrap();
        (id, line, prompt["prompt"].as_str().unwrap().to_owned())
    }

    fn row(&self, id: &str) -> Value {
        let rows = self.cooee.json("/threads");
        let row = rows.as_array().unwrap().iter().find(|r| r["id"] == id);
        row.unwrap_or_else(|| panic!("no row {id}: {rows}")).clone()
    }

    fn row_persona(&self, id: &str) -> Value {
        self.row(id)["persona"].clone()
    }

    fn remember(&self, id: &str, args: &Value) -> Value {
        let file = self.shed.dir.join(format!("mcp-cooee-{id}.json"));
        let servers: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
        let cooee = &servers[0];
        assert_eq!(cooee["name"], "cooee");
        let mut child = Command::new(cooee["command"].as_str().unwrap())
            .args(
                cooee["args"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|a| a.as_str().unwrap()),
            )
            .env("COOEE_MCP", cooee["env"][0]["value"].as_str().unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let call = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                           "params": { "name": "remember", "arguments": args } });
        writeln!(child.stdin.take().unwrap(), "{call}").unwrap();
        let out = child.wait_with_output().unwrap();
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["result"].clone()
    }
}

#[test]
fn a_persona_is_created_edited_and_deleted_and_its_notes_reach_its_next_session() {
    let s = setup("personas");
    let new = json!({ "name": "Chief Cook", "character": "A calm chef.", "instructions": "Answer in one line.",
                      "agent": "fake-acp", "folder": "scratch" });
    let (status, chief) = s.call("POST", "/personas", Some(&new));
    assert_eq!(status, 200, "{chief}");
    assert_eq!(chief["id"], "chief-cook");
    let file = s.dir.join("agents/chief-cook/agent.toml");
    assert!(
        fs::read_to_string(&file)
            .unwrap()
            .contains("name = \"Chief Cook\"")
    );
    assert_eq!(s.call("POST", "/personas", Some(&new)).0, 400);
    assert_eq!(
        s.call("POST", "/personas", Some(&json!({ "name": "X" }))).0,
        400
    );

    assert_eq!(chief["pinned"], false);

    let edit = json!({ "name": "Chief", "character": "A calm chef.", "instructions": "Answer in two lines.",
                       "agent": "fake-acp", "folder": "scratch", "pinned": true });
    let (status, edited) = s.call("PUT", "/personas/chief-cook", Some(&edit));
    assert_eq!(status, 200, "{edited}");
    let read = s.cooee.json("/personas/chief-cook");
    assert_eq!(read["name"], "Chief");
    assert_eq!(read["instructions"], "Answer in two lines.");
    assert_eq!(read["pinned"], true);
    assert!(fs::read_to_string(&file).unwrap().contains("pinned = true"));
    assert_eq!(s.call("PUT", "/personas/nobody", Some(&edit)).0, 400);

    assert_eq!(chief["style"], "chat");
    let chat = format!("A calm chef.\n\nAnswer in two lines.\n\n{PLAIN}\n\nhi");
    let (id, line, prompt) = s.start("chief-cook");
    assert!(line.contains(" --acp fake-acp "), "{line}");
    assert_eq!(prompt, chat);
    assert_eq!(s.row_persona(&id), "chief-cook");

    let said = s.remember(&id, &json!({ "text": "The owner takes tea, no sugar." }));
    assert_eq!(said["isError"], false, "{said}");
    let (status, owner) = s.call(
        "POST",
        "/personas/chief-cook/memory",
        Some(&json!({ "text": "Call me Sam." })),
    );
    assert_eq!((status, &owner["by"]), (200, &json!("owner")), "{owner}");
    let notes = s.cooee.json("/personas/chief-cook/memory");
    let notes = notes.as_array().unwrap();
    assert_eq!(
        notes
            .iter()
            .map(|n| (n["by"].as_str().unwrap(), n["text"].as_str().unwrap()))
            .collect::<Vec<_>>(),
        [
            ("persona", "The owner takes tea, no sugar."),
            ("owner", "Call me Sam.")
        ]
    );
    let (next, _, prompt) = s.start("chief-cook");
    let ids: Vec<&str> = notes.iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert!(
        prompt.starts_with("A calm chef.\n\nAnswer in two lines.\n\nYour notes")
            && prompt.contains(&format!(
                "- [{}] The owner takes tea, no sugar.\n- [{}] Call me Sam.\n\n{PLAIN}\n\nhi",
                ids[0], ids[1]
            ))
            && prompt.ends_with("\n\nhi"),
        "{prompt}"
    );
    let (plain, _, _) = s.start("");
    assert_eq!(s.remember(&plain, &json!({ "text": "x" }))["isError"], true);

    assert_eq!(s.call("DELETE", "/personas/chief-cook/memory", None).0, 200);
    assert_eq!(s.cooee.json("/personas/chief-cook/memory"), json!([]));
    assert_eq!(s.cooee.json("/personas/chief-cook")["name"], "Chief");
    let (_, _, prompt) = s.start("chief-cook");
    assert_eq!(prompt, chat);
    assert_eq!(
        s.cooee.json(&format!("/threads/{id}/messages"))["items"][0]["text"],
        "hi"
    );

    assert_eq!(s.call("DELETE", "/personas/chief-cook", None).0, 200);
    assert!(!s.dir.join("agents/chief-cook").exists());
    assert_eq!(s.call("GET", "/personas/chief-cook", None).0, 400);
    assert!(
        s.cooee
            .json("/personas")
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["id"] != "chief-cook")
    );
    for t in [&id, &next] {
        assert_eq!(s.row_persona(t), "");
        assert_eq!(
            s.cooee.json(&format!("/threads/{t}/messages"))["items"][0]["text"],
            "hi"
        );
    }

    let (head, _) = s.cooee.raw("GET", "/personas", &[], "");
    assert!(
        head.starts_with("HTTP/1.0 401") || head.starts_with("HTTP/1.1 401"),
        "{head}"
    );
}

#[test]
fn a_detailed_persona_gets_no_plain_line() {
    let s = setup("personas-detailed");
    let new = json!({ "name": "Chief", "character": "A calm chef.", "instructions": "Answer in two lines.",
                      "agent": "fake-acp", "folder": "scratch", "style": "detailed" });
    assert_eq!(s.call("POST", "/personas", Some(&new)).0, 200);
    let file = fs::read_to_string(s.dir.join("agents/chief/agent.toml")).unwrap();
    assert!(file.contains("style = \"detailed\""), "{file}");
    assert_eq!(s.cooee.json("/personas/chief")["style"], "detailed");
    let (_, _, prompt) = s.start("chief");
    assert_eq!(prompt, "A calm chef.\n\nAnswer in two lines.\n\nhi");
}

#[test]
fn an_agents_launch_keys_are_read_only_and_an_edit_keeps_them() {
    let s = setup("personas-launch");
    let file = s.dir.join("agents/coder/agent.toml");
    let list = s.cooee.json("/personas");
    assert_eq!(
        list,
        json!([{ "id": "coder", "name": "Coder", "avatar": "idle", "character": "",
                 "instructions": "Follow the house rules.\n", "folder": "", "agent": "fake-acp",
                 "model": "", "effort": "", "machine": "", "plugins": [], "can_message": [], "pinned": false,
                 "role": "", "style": "chat", "fleets": [], "env": { "MODE": "review" }, "policy": "allow" }])
    );

    let check = |prompt: &str| {
        let (id, line, sent) = s.start("coder");
        assert!(
            line.contains(" --policy allow ") && line.contains(" MODE=review "),
            "{line}"
        );
        assert_eq!(sent, prompt);
        assert_eq!(s.row_persona(&id), "coder");
    };
    check(&format!("Follow the house rules.\n\n{PLAIN}\n\nhi"));

    let mut edit = list[0].clone();
    edit["character"] = json!("Gruff but kind.");
    edit["policy"] = json!("deny");
    edit["env"] = json!({});
    let (status, saved) = s.call("PUT", "/personas/coder", Some(&edit));
    assert_eq!(
        (status, &saved["policy"]),
        (200, &json!("allow")),
        "{saved}"
    );
    let text = fs::read_to_string(&file).unwrap();
    assert!(
        text.contains("policy = \"allow\"") && text.contains("MODE = \"review\""),
        "{text}"
    );
    let character = fs::read_to_string(s.dir.join("agents/coder/CHARACTER.md")).unwrap();
    assert_eq!(character, "Gruff but kind.");
    check(&format!(
        "Gruff but kind.\n\nFollow the house rules.\n\n{PLAIN}\n\nhi"
    ));
    edit["profile"] = json!("coder");
    assert_eq!(s.call("PUT", "/personas/coder", Some(&edit)).0, 400);

    assert_eq!(s.call("DELETE", "/personas/coder", None).0, 200);
    assert!(!s.dir.join("agents/coder").exists());
    assert_eq!(s.cooee.json("/personas"), json!([]));
}

fn install_drover(s: &Setup) {
    let lock = json!([{ "name": "drover", "url": "https://example.com/drover", "commit": "c0",
                        "version": "0.1.0", "tabs": ["queue"], "port": 1, "enabled": false }]);
    fs::write(s.dir.join("data/plugins.lock"), lock.to_string()).unwrap();
}

#[test]
fn a_plugin_is_attached_to_a_persona_and_detached() {
    let s = setup("personas-plugins");
    install_drover(&s);
    let new = json!({ "name": "Chief", "agent": "fake-acp" });
    let (status, mut chief) = s.call("POST", "/personas", Some(&new));
    assert_eq!((status, &chief["plugins"]), (200, &json!([])), "{chief}");

    chief["plugins"] = json!(["drover"]);
    let (status, saved) = s.call("PUT", "/personas/chief", Some(&chief));
    assert_eq!(status, 200, "{saved}");
    assert_eq!(
        s.cooee.json("/personas/chief")["plugins"],
        json!(["drover"])
    );
    assert_eq!(s.cooee.json("/personas/coder")["plugins"], json!([]));
    assert_eq!(s.cooee.json("/personas/chief/places"), json!([]));
    assert_eq!(s.call("GET", "/personas/nobody/places", None).0, 400);

    chief["plugins"] = json!(["drover", "nope"]);
    let (status, why) = s.call("PUT", "/personas/chief", Some(&chief));
    assert_eq!(status, 400, "{why}");
    assert!(
        why.as_str().unwrap().contains("no installed plugin nope"),
        "{why}"
    );
    assert_eq!(
        s.cooee.json("/personas/chief")["plugins"],
        json!(["drover"])
    );

    chief["plugins"] = json!([]);
    assert_eq!(s.call("PUT", "/personas/chief", Some(&chief)).0, 200);
    assert_eq!(s.cooee.json("/personas/chief")["plugins"], json!([]));
}

fn old_install(name: &str, drover: bool, old: &[(&str, &str)]) -> Setup {
    let dir = scratch(name);
    for (file, text) in old {
        fs::create_dir_all(dir.join(file).parent().unwrap()).unwrap();
        fs::write(dir.join(file), text).unwrap();
    }
    if drover {
        let lock = json!([{ "name": "drover", "url": "https://example.com/drover", "commit": "c0",
                            "version": "0.1.0", "tabs": ["queue"], "port": 1, "enabled": false }]);
        fs::create_dir_all(dir.join("data")).unwrap();
        fs::write(dir.join("data/plugins.lock"), lock.to_string()).unwrap();
    }
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    Setup { dir, shed, cooee }
}

#[test]
fn migration_keeps_the_name_and_sets_the_orchestrator_role() {
    let conductor = "name = \"conductor\"\nlabel = \"Maestro\"\nagent = \"fake-acp\"\n";
    let coder = "name = \"coder\"\nlabel = \"Coder\"\nagent = \"fake-acp\"\n";
    let s = old_install(
        "personas-drover",
        false,
        &[
            ("profiles/conductor.toml", conductor),
            ("profiles/coder.toml", coder),
        ],
    );
    let seen = |s: &Setup, id: &str| {
        let p = s.cooee.json(&format!("/personas/{id}"));
        (p["name"].clone(), p["role"].clone(), p["plugins"].clone())
    };
    assert_eq!(
        seen(&s, "conductor"),
        (json!("Maestro"), json!("orchestrator"), json!([]))
    );
    assert_eq!(seen(&s, "coder"), (json!("Coder"), json!(""), json!([])));
    assert!(s.dir.join("agents/conductor/agent.toml").exists());
    let style = |s: &Setup, id: &str| s.cooee.json(&format!("/personas/{id}"))["style"].clone();
    assert_eq!(style(&s, "conductor"), "detailed");
    assert_eq!(style(&s, "coder"), "chat");
    install_drover(&s);
    assert_eq!(seen(&s, "conductor").2, json!(["drover"]));
    assert_eq!(seen(&s, "coder").2, json!([]));

    let navi = "name = \"Navi\"\nagent = \"fake-acp\"\nplugins = []\n";
    let s = old_install(
        "personas-drover-file",
        true,
        &[
            ("personas/conductor.toml", navi),
            ("profiles/conductor.toml", conductor),
        ],
    );
    assert_eq!(
        seen(&s, "conductor"),
        (json!("Navi"), json!("orchestrator"), json!(["drover"]))
    );
    assert_eq!(style(&s, "conductor"), "detailed");
    fs::write(
        s.dir.join("agents/conductor/agent.toml"),
        "name = \"Navi\"\nrole = \"orchestrator\"\nagent = \"fake-acp\"\nstyle = \"chat\"\n",
    )
    .unwrap();
    assert_eq!(style(&s, "conductor"), "chat");
}

#[test]
fn renaming_the_orchestrator_changes_no_attachment() {
    let s = setup("personas-rename");
    install_drover(&s);
    let new = json!({ "name": "Conductor", "role": "orchestrator", "fleets": ["cooee"], "agent": "fake-acp" });
    let (status, mut p) = s.call("POST", "/personas", Some(&new));
    assert_eq!(status, 200, "{p}");
    let (id, ..) = s.start("conductor");
    p["name"] = json!("Navi");
    assert_eq!(s.call("PUT", "/personas/conductor", Some(&p)).0, 200);
    let read = s.cooee.json("/personas/conductor");
    assert_eq!(
        (
            &read["name"],
            &read["role"],
            &read["plugins"],
            &read["fleets"]
        ),
        (
            &json!("Navi"),
            &json!("orchestrator"),
            &json!(["drover"]),
            &json!(["cooee"])
        )
    );
    let row = s.row(&id);
    assert_eq!(
        (&row["persona"], &row["persona_name"], &row["persona_role"]),
        (&json!("conductor"), &json!("Navi"), &json!("orchestrator"))
    );
}

#[test]
fn two_orchestrator_personas_each_with_their_own_fleets() {
    let s = setup("personas-two");
    install_drover(&s);
    for (name, fleets) in [
        ("Ada", json!(["cooee"])),
        ("Bo", json!(["herdsman", "cooee"])),
    ] {
        let new =
            json!({ "name": name, "role": "orchestrator", "fleets": fleets, "agent": "fake-acp" });
        assert_eq!(s.call("POST", "/personas", Some(&new)).0, 200);
    }
    let all = s.cooee.json("/personas");
    let seen: Vec<_> = all
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["id"].clone(),
                p["role"].clone(),
                p["fleets"].clone(),
                p["plugins"].clone(),
            )
        })
        .collect();
    let drover = json!(["drover"]);
    assert_eq!(
        seen,
        [
            (
                json!("ada"),
                json!("orchestrator"),
                json!(["cooee"]),
                drover.clone()
            ),
            (
                json!("bo"),
                json!("orchestrator"),
                json!(["herdsman", "cooee"]),
                drover
            ),
            (json!("coder"), json!(""), json!([]), json!([])),
        ]
    );
    let (bo, ..) = s.start("bo");
    let (coder, ..) = s.start("coder");
    let (bo, coder) = (s.row(&bo), s.row(&coder));
    assert_eq!(
        (&bo["persona_name"], &bo["persona_role"]),
        (&json!("Bo"), &json!("orchestrator"))
    );
    assert_eq!(
        (&coder["persona_name"], &coder["persona_role"]),
        (&json!("Coder"), &json!(""))
    );
}

#[test]
fn the_plugin_attaches_by_role() {
    let s = setup("personas-role");
    install_drover(&s);
    let new = json!({ "name": "Conductor", "agent": "fake-acp" });
    let (status, mut p) = s.call("POST", "/personas", Some(&new));
    assert_eq!((status, &p["plugins"]), (200, &json!([])), "{p}");
    p["role"] = json!("orchestrator");
    assert_eq!(s.call("PUT", "/personas/conductor", Some(&p)).0, 200);
    let read = s.cooee.json("/personas/conductor");
    assert_eq!(read["plugins"], json!(["drover"]));
    let mut read = read;
    read["role"] = json!("");
    read["plugins"] = json!([]);
    assert_eq!(s.call("PUT", "/personas/conductor", Some(&read)).0, 200);
    assert_eq!(s.cooee.json("/personas/conductor")["plugins"], json!([]));
    read["role"] = json!("boss");
    assert!(s.call("PUT", "/personas/conductor", Some(&read)).0 >= 400);
}

#[test]
fn a_persona_and_its_threads_name_their_machine() {
    let s = setup("persona-machine");
    let write = |name: &str, v: &Value| fs::write(s.shed.dir.join(name), v.to_string()).unwrap();
    write(
        "machines.json",
        &json!([{ "name": "air", "target": "air.local" }]),
    );
    write(
        "stats-local.json",
        &json!({ "at": 1_800_000_000u64, "load_per_cpu": 0.42, "mem_pressure_pct": 61, "swap_used_mb": 0,
                 "disk_free_mb": 50_000, "cpus": 10, "uptime_s": 60, "terminals": { "working": 1 } }),
    );
    write(
        "stats-air.json",
        &json!({ "machine": "air", "online": false, "last_seen": 1_700_000_000u64 }),
    );
    s.shed.link("air", "up", 1_700_000_000, None);

    let old = "name = \"Old\"\nfolder = \"scratch\"\nagent = \"fake-acp\"\nmodel = \"\"\neffort = \"\"\nplugins = []\n";
    let home = agent(&s.dir, "old", old, "Still.");
    fs::write(home.join("CHARACTER.md"), "Kept.").unwrap();
    let read = s.cooee.json("/personas/old");
    assert_eq!(
        read,
        json!({ "id": "old", "name": "Old", "character": "Kept.", "instructions": "Still.", "folder": "scratch",
                "agent": "fake-acp", "model": "", "effort": "", "machine": "", "plugins": [], "can_message": [],
                "pinned": false, "role": "", "style": "chat", "fleets": [] })
    );
    assert_eq!(s.cooee.json("/personas/coder")["machine"], "");
    let (plain, _, _) = s.start("old");
    assert_eq!(s.row(&plain)["machine"], "");

    let mut on_air = read.clone();
    on_air["machine"] = json!("air");
    let (status, saved) = s.call("PUT", "/personas/old", Some(&on_air));
    assert_eq!((status, &saved["machine"]), (200, &json!("air")), "{saved}");
    let file = fs::read_to_string(home.join("agent.toml")).unwrap();
    let character = fs::read_to_string(home.join("CHARACTER.md")).unwrap();
    assert!(
        file.contains("machine = \"air\"") && character == "Kept.",
        "{file}"
    );
    assert_eq!(s.cooee.json("/personas/old")["machine"], "air");
    let (id, _, _) = s.start("old");
    assert_eq!(s.row(&id)["machine"], "air");

    let new = json!({ "agent": "fake-acp", "profile": "old", "text": "hi", "random_id": "here", "machine": "" });
    let (status, body) = s.call("POST", "/threads", Some(&new));
    assert_eq!(status, 200, "{body}");
    assert_eq!(s.row(body["id"].as_str().unwrap())["machine"], "");
    assert_eq!(s.cooee.json("/personas/old")["machine"], "air");

    let mut nowhere = read.clone();
    nowhere["machine"] = json!("nowhere");
    let (status, why) = s.call("PUT", "/personas/old", Some(&nowhere));
    assert_eq!(status, 400, "{why}");
    assert!(why.to_string().contains("no machine nowhere"), "{why}");
    assert_eq!(s.cooee.json("/personas/old")["machine"], "air");
    let rows = s.cooee.json("/threads").as_array().unwrap().len();
    let new =
        json!({ "agent": "fake-acp", "text": "hi", "random_id": "far", "machine": "nowhere" });
    let (status, why) = s.call("POST", "/threads", Some(&new));
    assert_eq!(status, 400, "{why}");
    assert_eq!(s.cooee.json("/threads").as_array().unwrap().len(), rows);

    let (status, cleared) = s.call("PUT", "/personas/old", Some(&read));
    assert_eq!(
        (status, &cleared["machine"]),
        (200, &json!("")),
        "{cleared}"
    );
    let (id, _, _) = s.start("old");
    assert_eq!(s.row(&id)["machine"], "");

    let machines = s.cooee.json("/machines");
    let machines = machines.as_array().unwrap();
    assert_eq!(machines.len(), 2, "{machines:?}");
    assert_eq!(
        (
            &machines[0]["name"],
            &machines[0]["online"],
            &machines[0]["health"]["load_per_cpu"]
        ),
        (&json!(""), &json!(true), &json!(0.42))
    );
    assert_eq!(
        (
            &machines[1]["name"],
            &machines[1]["online"],
            &machines[1]["last_seen"]
        ),
        (&json!("air"), &json!(false), &json!(1_700_000_000u64))
    );
}

fn cook(s: &Setup) -> String {
    let new = json!({ "name": "Cook", "agent": "fake-acp", "folder": "scratch" });
    assert_eq!(s.call("POST", "/personas", Some(&new)).0, 200);
    s.start("cook").0
}

fn memory(s: &Setup) -> Vec<Value> {
    let notes = s.cooee.json("/personas/cook/memory");
    notes.as_array().unwrap().clone()
}

#[test]
fn old_notes_past_the_cap_get_ids_and_a_session_keeps_its_brief() {
    let s = setup("personas-memory-old");
    let new = json!({ "name": "Cook", "agent": "fake-acp", "folder": "scratch" });
    assert_eq!(s.call("POST", "/personas", Some(&new)).0, 200);

    let big = "y".repeat(2000);
    let file = s.dir.join("agents/cook/memory.md");
    let old =
        format!("- Tea, no sugar. <!-- by:persona at:1 -->\n- {big} <!-- by:owner at:2 -->\n");
    fs::write(&file, old).unwrap();
    let (first, _, brief) = s.start("cook");
    let notes = memory(&s);
    let (tea, long) = (
        notes[0]["id"].as_str().unwrap(),
        notes[1]["id"].as_str().unwrap(),
    );
    assert!(
        !tea.is_empty() && !long.is_empty() && tea != long,
        "{notes:?}"
    );
    assert_eq!(
        brief,
        format!(
            "Your notes (edit them with your remember tool):\n- [{tea}] Tea, no sugar.\n- [{long}] {big}\n\n{PLAIN}\n\nhi"
        )
    );
    let (second, _, again) = s.start("cook");
    assert_eq!(again, brief);

    let full = s.remember(&first, &json!({ "text": "New." }));
    let said = full["content"][0]["text"].as_str().unwrap();
    assert_eq!(full["isError"], true, "{full}");
    assert!(
        said.contains("2014 of 2000") && said.contains(tea) && said.contains(long),
        "{said}"
    );
    assert_eq!(memory(&s), notes);

    let path = format!("/personas/cook/memory/{long}");
    let (status, put) = s.call("PUT", &path, Some(&json!({ "text": "Short." })));
    assert_eq!((status, put["id"].as_str()), (200, Some(long)), "{put}");
    let now = memory(&s);
    assert_eq!(now[0], notes[0]);
    assert_eq!((now[1]["text"].as_str(), now.len()), (Some("Short."), 2));

    let say = json!({ "text": "again", "random_id": "r2" });
    let (status, body) = s.call("POST", &format!("/threads/{second}/send"), Some(&say));
    assert_eq!(status, 200, "{body}");
    let sent = s.shed.sent(&format!("cooee-{second}"));
    assert_eq!(sent[1], "\x1e{\"prompt\":\"again\"}\n");
}

#[test]
fn remember_adds_replaces_and_forgets_one_note_under_the_cap() {
    let s = setup("personas-memory");
    let id = cook(&s);

    let tea = s.remember(&id, &json!({ "text": "Tea, no sugar." }));
    assert_eq!(tea["isError"], false, "{tea}");
    let tea = memory(&s)[0]["id"].as_str().unwrap().to_owned();
    let dup = s.remember(&id, &json!({ "text": "Tea, no sugar." }));
    let kept = format!("Kept as [{tea}]; 14 of 2000 characters used.");
    assert_eq!(dup["content"][0]["text"], kept, "{dup}");
    assert_eq!(memory(&s).len(), 1);
    for text in ["Call me Sam.", "Lunch at one."] {
        assert_eq!(s.remember(&id, &json!({ "text": text }))["isError"], false);
    }
    let notes = memory(&s);
    let sam = notes[1]["id"].as_str().unwrap();

    let swap = s.remember(&id, &json!({ "text": "Call me Sammy.", "replaces": sam }));
    assert_eq!(swap["isError"], false, "{swap}");
    let now = memory(&s);
    assert_eq!((&now[0], &now[2]), (&notes[0], &notes[2]));
    assert_eq!(
        (now[1]["id"].as_str(), now[1]["text"].as_str()),
        (Some(sam), Some("Call me Sammy."))
    );

    let gone = s.remember(&id, &json!({ "forget": [tea] }));
    let said = "Forgotten; 27 of 2000 characters used.";
    assert_eq!(gone["content"][0]["text"], said, "{gone}");
    assert_eq!(memory(&s), now[1..]);
    let path = format!("/personas/cook/memory/{sam}");
    assert_eq!(s.call("DELETE", &path, None).0, 200);
    assert_eq!(memory(&s), now[2..]);
    assert_eq!(
        s.remember(&id, &json!({ "forget": ["zz"] }))["isError"],
        true
    );

    let up_to = json!({ "text": "z".repeat(1987) });
    assert_eq!(s.call("POST", "/personas/cook/memory", Some(&up_to)).0, 200);
    let one = json!({ "text": "1" });
    let (status, body) = s.call("POST", "/personas/cook/memory", Some(&one));
    assert_eq!(status, 400, "{body}");
    let body = body.as_str().unwrap();
    assert!(body.contains("2000 of 2000 characters used"), "{body}");
    assert_eq!(memory(&s).len(), 2);
}
