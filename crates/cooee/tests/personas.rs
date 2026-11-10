//! Personas over the API (#126) with the fake shed (tests/support): one is created, edited and deleted; today's
//! profile is read as a persona with nothing lost and runs as that profile; a note by the persona (its `remember`
//! tool through cooee's MCP server) and one by the owner both list and reach its next session; clearing its memory,
//! or deleting it, leaves its chats. Plugins attach to a persona and detach (#129); an installed drover goes with the
//! conductor profile's.

mod support;

use std::{
    fs,
    io::Write as _,
    path::PathBuf,
    process::{Command, Stdio},
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

struct Setup {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
}

/// cooee with the profile `coder` in `<config>/profiles/` (its instructions, env, policy and avatar).
fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let profiles = dir.join("profiles");
    fs::create_dir_all(&profiles).unwrap();
    fs::write(profiles.join("coder.md"), "Follow the house rules.\n").unwrap();
    fs::write(
        profiles.join("coder.toml"),
        "name = \"coder\"\nlabel = \"Coder\"\nagent = \"fake-acp\"\ninstructions = \"coder.md\"\n\
         policy = \"allow\"\navatar = \"idle\"\nenv = { MODE = \"review\" }\n",
    )
    .unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    Setup { dir, shed, cooee }
}

impl Setup {
    /// `method path` with `body`: the status and the body as JSON (a string when it isn't).
    fn call(&self, method: &str, path: &str, body: Option<&Value>) -> (u16, Value) {
        let (status, text) = self.cooee.request(method, path, body);
        (
            status,
            serde_json::from_str(&text).unwrap_or(Value::String(text)),
        )
    }

    /// A new thread as `persona`: its id, its `shed run` line, and the first prompt the agent got.
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
        let sent = self.shed.sent(&format!("cooee-{id}"));
        let prompt: Value = serde_json::from_str(sent[0].trim_start_matches('\x1e')).unwrap();
        (id, line, prompt["prompt"].as_str().unwrap().to_owned())
    }

    /// Thread `id`'s row in `GET /threads`.
    fn row(&self, id: &str) -> Value {
        let rows = self.cooee.json("/threads");
        let row = rows.as_array().unwrap().iter().find(|r| r["id"] == id);
        row.unwrap_or_else(|| panic!("no row {id}: {rows}")).clone()
    }

    /// The persona of thread `id`'s row in `GET /threads`.
    fn row_persona(&self, id: &str) -> Value {
        self.row(id)["persona"].clone()
    }

    /// `remember` called by thread `id`'s agent through cooee's own MCP server: the tool's answer.
    fn remember(&self, id: &str, text: &str) -> Value {
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
                           "params": { "name": "remember", "arguments": { "text": text } } });
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
    let file = s.dir.join("personas/chief-cook.toml");
    assert!(
        fs::read_to_string(&file)
            .unwrap()
            .contains("name = \"Chief Cook\"")
    );
    // The same name again, or one with no agent, is refused.
    assert_eq!(s.call("POST", "/personas", Some(&new)).0, 400);
    assert_eq!(
        s.call("POST", "/personas", Some(&json!({ "name": "X" }))).0,
        400
    );

    assert_eq!(chief["pinned"], false);

    // Edited and pinned (#139): its id stays, the edit is kept and read back.
    let edit = json!({ "name": "Chief", "character": "A calm chef.", "instructions": "Answer in two lines.",
                       "agent": "fake-acp", "folder": "scratch", "pinned": true });
    let (status, edited) = s.call("PUT", "/personas/chief-cook", Some(&edit));
    assert_eq!(status, 200, "{edited}");
    let read = s.cooee.json("/personas/chief-cook");
    assert_eq!(
        (
            read["name"].as_str(),
            read["instructions"].as_str(),
            read["pinned"].as_bool()
        ),
        (Some("Chief"), Some("Answer in two lines."), Some(true))
    );
    assert!(fs::read_to_string(&file).unwrap().contains("pinned = true"));
    assert_eq!(s.call("PUT", "/personas/nobody", Some(&edit)).0, 400);

    // Its chat: its agent, Character and Instructions ahead of the first message; the row names it.
    let (id, line, prompt) = s.start("chief-cook");
    assert!(line.contains(" --acp fake-acp "), "{line}");
    assert_eq!(prompt, "A calm chef.\n\nAnswer in two lines.\n\nhi");
    assert_eq!(s.row_persona(&id), "chief-cook");

    // A note by the persona (its remember tool) and one by the owner: both listed, both in its next session.
    let said = s.remember(&id, "The owner takes tea, no sugar.");
    assert_eq!(said["isError"], false, "{said}");
    let (status, owner) = s.call(
        "POST",
        "/personas/chief-cook/memory",
        Some(&json!({ "text": "Call me Sam." })),
    );
    assert_eq!((status, &owner["by"]), (200, &json!("owner")), "{owner}");
    let notes = s.cooee.json("/personas/chief-cook/memory");
    let notes: Vec<(&str, &str)> = notes
        .as_array()
        .unwrap()
        .iter()
        .map(|n| (n["by"].as_str().unwrap(), n["text"].as_str().unwrap()))
        .collect();
    assert_eq!(
        notes,
        [
            ("persona", "The owner takes tea, no sugar."),
            ("owner", "Call me Sam.")
        ]
    );
    let (next, _, prompt) = s.start("chief-cook");
    assert!(
        prompt.starts_with("A calm chef.\n\nAnswer in two lines.\n\nYour notes")
            && prompt
                .contains("- The owner takes tea, no sugar. (persona)\n- Call me Sam. (owner)")
            && prompt.ends_with("\n\nhi"),
        "{prompt}"
    );
    // A thread with no persona has nothing to remember for.
    let (plain, _, _) = s.start("");
    assert_eq!(s.remember(&plain, "x")["isError"], true);

    // Clearing its memory removes the notes only: it and its chats stay.
    assert_eq!(s.call("DELETE", "/personas/chief-cook/memory", None).0, 200);
    assert_eq!(s.cooee.json("/personas/chief-cook/memory"), json!([]));
    assert_eq!(s.cooee.json("/personas/chief-cook")["name"], "Chief");
    let (_, _, prompt) = s.start("chief-cook");
    assert_eq!(prompt, "A calm chef.\n\nAnswer in two lines.\n\nhi");
    assert_eq!(
        s.cooee.json(&format!("/threads/{id}/messages"))["items"][0]["text"],
        "hi"
    );

    // Deleted: gone from the list, its file gone, its chats kept (their rows name no persona now).
    assert_eq!(s.call("DELETE", "/personas/chief-cook", None).0, 200);
    assert!(!file.exists());
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

    // Paired devices only.
    let (head, _) = s.cooee.raw("GET", "/personas", &[], "");
    assert!(
        head.starts_with("HTTP/1.0 401") || head.starts_with("HTTP/1.1 401"),
        "{head}"
    );
}

#[test]
fn a_profile_is_read_as_a_persona_with_nothing_lost_and_an_edit_keeps_it() {
    let s = setup("personas-migrate");
    let profile = s.dir.join("profiles/coder.toml");
    let before = fs::read_to_string(&profile).unwrap();
    let list = s.cooee.json("/personas");
    assert_eq!(
        list,
        json!([{ "id": "coder", "name": "Coder", "avatar": "idle", "character": "",
                 "instructions": "Follow the house rules.\n", "folder": "", "agent": "fake-acp",
                 "model": "", "effort": "", "machine": "", "plugins": [], "can_message": [], "pinned": false, "profile": "coder" }])
    );

    // It runs as the profile: its env and policy on the adapter, its instructions ahead of the first message.
    let check = |prompt: &str| {
        let (id, line, sent) = s.start("coder");
        assert!(
            line.contains(" --policy allow ") && line.contains(" MODE=review "),
            "{line}"
        );
        assert_eq!(sent, prompt);
        assert_eq!(s.row_persona(&id), "coder");
    };
    check("Follow the house rules.\n\nhi");
    // The phone's page still offers it as before.
    assert!(s.cooee.get("/").1.contains("Coder"));

    // Edited: saved as a persona that still runs as the profile, which is untouched.
    let mut edit = list[0].clone();
    edit["character"] = json!("Gruff but kind.");
    edit["profile"] = json!("someone-else");
    let (status, saved) = s.call("PUT", "/personas/coder", Some(&edit));
    assert_eq!(
        (status, &saved["profile"]),
        (200, &json!("coder")),
        "{saved}"
    );
    assert!(s.dir.join("personas/coder.toml").exists());
    assert_eq!(fs::read_to_string(&profile).unwrap(), before);
    check("Gruff but kind.\n\nFollow the house rules.\n\nhi");

    // Deleting it removes the persona and the profile it runs as.
    assert_eq!(s.call("DELETE", "/personas/coder", None).0, 200);
    assert!(!profile.exists() && !s.dir.join("personas/coder.toml").exists());
    assert_eq!(s.cooee.json("/personas"), json!([]));
}

/// drover installed (off, so nothing runs it) in `<data>/plugins.lock`.
fn install_drover(s: &Setup) {
    let lock = json!([{ "name": "cooee-drover", "url": "https://example.com/cooee-drover", "commit": "c0",
                        "version": "0.1.0", "tabs": ["queue"], "port": 1, "enabled": false }]);
    fs::write(s.dir.join("data/plugins.lock"), lock.to_string()).unwrap();
}

/// #129: a plugin is attached and detached over the API, only an installed one; the places list only attached
/// plugins' screens, and none has one before #125.
#[test]
fn a_plugin_is_attached_to_a_persona_and_detached() {
    let s = setup("personas-plugins");
    install_drover(&s);
    let new = json!({ "name": "Chief", "agent": "fake-acp" });
    let (status, mut chief) = s.call("POST", "/personas", Some(&new));
    assert_eq!((status, &chief["plugins"]), (200, &json!([])), "{chief}");

    chief["plugins"] = json!(["cooee-drover"]);
    let (status, saved) = s.call("PUT", "/personas/chief", Some(&chief));
    assert_eq!(status, 200, "{saved}");
    assert_eq!(
        s.cooee.json("/personas/chief")["plugins"],
        json!(["cooee-drover"])
    );
    assert_eq!(s.cooee.json("/personas/coder")["plugins"], json!([]));
    assert_eq!(s.cooee.json("/personas/chief/places"), json!([]));
    assert_eq!(s.call("GET", "/personas/nobody/places", None).0, 400);

    chief["plugins"] = json!(["cooee-drover", "nope"]);
    let (status, why) = s.call("PUT", "/personas/chief", Some(&chief));
    assert_eq!(status, 400, "{why}");
    assert!(
        why.as_str().unwrap().contains("no installed plugin nope"),
        "{why}"
    );
    assert_eq!(
        s.cooee.json("/personas/chief")["plugins"],
        json!(["cooee-drover"])
    );

    chief["plugins"] = json!([]);
    assert_eq!(s.call("PUT", "/personas/chief", Some(&chief)).0, 200);
    assert_eq!(s.cooee.json("/personas/chief")["plugins"], json!([]));
}

/// #129: an installed drover is attached to the persona read from the conductor profile, and to no other.
#[test]
fn migration_attaches_drover_to_the_conductor_persona_only() {
    let s = setup("personas-drover");
    fs::write(
        s.dir.join("profiles/conductor.toml"),
        "name = \"conductor\"\nlabel = \"Conductor\"\nagent = \"fake-acp\"\n",
    )
    .unwrap();
    let plugins = |id: &str| s.cooee.json(&format!("/personas/{id}"))["plugins"].clone();
    assert_eq!(
        (plugins("conductor"), plugins("coder")),
        (json!([]), json!([]))
    );
    install_drover(&s);
    assert_eq!(
        (plugins("conductor"), plugins("coder")),
        (json!(["cooee-drover"]), json!([]))
    );
}

/// #148: a persona names the machine it runs on (empty: this host), one added to shed; a thread runs on its
/// persona's or the one picked at its start; anything else is refused; one saved before #148 is on this host with
/// nothing lost; `GET /machines` lists the allowed ones with what shed says of them.
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

    // Migration: a persona file from before #148 (and the profile read as one) is on this host, nothing lost.
    let old = "name = \"Old\"\ncharacter = \"Kept.\"\ninstructions = \"Still.\"\nfolder = \"scratch\"\n\
               agent = \"fake-acp\"\nmodel = \"\"\neffort = \"\"\nplugins = []\n";
    fs::create_dir_all(s.dir.join("personas")).unwrap();
    fs::write(s.dir.join("personas/old.toml"), old).unwrap();
    let read = s.cooee.json("/personas/old");
    assert_eq!(
        read,
        json!({ "id": "old", "name": "Old", "character": "Kept.", "instructions": "Still.", "folder": "scratch",
                "agent": "fake-acp", "model": "", "effort": "", "machine": "", "plugins": [], "can_message": [],
                "pinned": false })
    );
    assert_eq!(s.cooee.json("/personas/coder")["machine"], "");
    let (plain, _, _) = s.start("old");
    assert_eq!(s.row(&plain)["machine"], "");

    // Set: kept in its file and read back; its threads run there.
    let mut on_air = read.clone();
    on_air["machine"] = json!("air");
    let (status, saved) = s.call("PUT", "/personas/old", Some(&on_air));
    assert_eq!((status, &saved["machine"]), (200, &json!("air")), "{saved}");
    let file = fs::read_to_string(s.dir.join("personas/old.toml")).unwrap();
    assert!(
        file.contains("machine = \"air\"") && file.contains("character = \"Kept.\""),
        "{file}"
    );
    assert_eq!(s.cooee.json("/personas/old")["machine"], "air");
    let (id, _, _) = s.start("old");
    assert_eq!(s.row(&id)["machine"], "air");

    // Override at start: this host for one thread; the persona keeps air.
    let new = json!({ "agent": "fake-acp", "profile": "old", "text": "hi", "random_id": "here", "machine": "" });
    let (status, body) = s.call("POST", "/threads", Some(&new));
    assert_eq!(status, 200, "{body}");
    assert_eq!(s.row(body["id"].as_str().unwrap())["machine"], "");
    assert_eq!(s.cooee.json("/personas/old")["machine"], "air");

    // Outside the allowlist: refused, for a persona and for a thread, nothing saved or started.
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

    // Cleared: back on this host.
    let (status, cleared) = s.call("PUT", "/personas/old", Some(&read));
    assert_eq!(
        (status, &cleared["machine"]),
        (200, &json!("")),
        "{cleared}"
    );
    let (id, _, _) = s.start("old");
    assert_eq!(s.row(&id)["machine"], "");

    // The allowed machines: this host first, then each added one, with shed's link state and health.
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
