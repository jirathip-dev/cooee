use std::fs;

use cooee_core::agents::PLAIN;
use serde_json::{Value, json};

use crate::setup::{Setup, setup};
use crate::support::agent;

fn create_chief_cook(s: &Setup) -> (Value, std::path::PathBuf) {
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
    (chief, file)
}

fn edit_chief_cook_and_pin_it(s: &Setup, file: &std::path::Path) {
    let edit = json!({ "name": "Chief", "character": "A calm chef.", "instructions": "Answer in two lines.",
                       "agent": "fake-acp", "folder": "scratch", "pinned": true });
    let (status, edited) = s.call("PUT", "/personas/chief-cook", Some(&edit));
    assert_eq!(status, 200, "{edited}");
    let read = s.cooee.json("/personas/chief-cook");
    assert_eq!(read["name"], "Chief");
    assert_eq!(read["instructions"], "Answer in two lines.");
    assert_eq!(read["pinned"], true);
    assert!(fs::read_to_string(file).unwrap().contains("pinned = true"));
    assert_eq!(s.call("PUT", "/personas/nobody", Some(&edit)).0, 400);
}

fn notes_reach_the_next_session(s: &Setup, id: &str) -> String {
    let said = s.remember(id, &json!({ "text": "The owner takes tea, no sugar." }));
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
    next
}

fn deleting_the_persona_keeps_its_threads(s: &Setup, id: &str, next: &str) {
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
    for t in [id, next] {
        assert_eq!(s.row_persona(t), "");
        assert_eq!(
            s.cooee.json(&format!("/threads/{t}/messages"))["items"][0]["text"],
            "hi"
        );
    }
}

#[test]
fn a_persona_is_created_edited_and_deleted_and_its_notes_reach_its_next_session() {
    let s = setup("personas");
    let (chief, file) = create_chief_cook(&s);
    edit_chief_cook_and_pin_it(&s, &file);

    assert_eq!(chief["style"], "chat");
    let chat = format!("A calm chef.\n\nAnswer in two lines.\n\n{PLAIN}\n\nhi");
    let (id, line, prompt) = s.start("chief-cook");
    assert!(line.contains(" --acp fake-acp "), "{line}");
    assert_eq!(prompt, chat);
    assert_eq!(s.row_persona(&id), "chief-cook");

    let next = notes_reach_the_next_session(&s, &id);

    assert_eq!(s.call("DELETE", "/personas/chief-cook/memory", None).0, 200);
    assert_eq!(s.cooee.json("/personas/chief-cook/memory"), json!([]));
    assert_eq!(s.cooee.json("/personas/chief-cook")["name"], "Chief");
    let (_, _, prompt) = s.start("chief-cook");
    assert_eq!(prompt, chat);
    assert_eq!(
        s.cooee.json(&format!("/threads/{id}/messages"))["items"][0]["text"],
        "hi"
    );

    deleting_the_persona_keeps_its_threads(&s, &id, &next);

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
    let mut list = s.cooee.json("/personas");
    let version = list[0].as_object_mut().unwrap().remove("version");
    assert!(version.is_some_and(|v| v.is_string()));
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
            line.contains(" --policy ask ") && line.contains(" MODE=review "),
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

fn box_offline_and_local_busy(s: &Setup) {
    let write = |name: &str, v: &Value| fs::write(s.shed.dir.join(name), v.to_string()).unwrap();
    write(
        "machines.json",
        &json!([{ "name": "box", "target": "box.local" }]),
    );
    write(
        "stats-local.json",
        &json!({ "at": 1_800_000_000u64, "load_per_cpu": 0.42, "mem_pressure_pct": 61, "swap_used_mb": 0,
                 "disk_free_mb": 50_000, "cpus": 10, "uptime_s": 60, "terminals": { "working": 1 } }),
    );
    write(
        "stats-box.json",
        &json!({ "machine": "box", "online": false, "last_seen": 1_700_000_000u64 }),
    );
    s.shed.link("box", "up", 1_700_000_000, None);
}

fn old_persona_with_its_character_kept(s: &Setup) -> (std::path::PathBuf, Value) {
    let old = "name = \"Old\"\nfolder = \"scratch\"\nagent = \"fake-acp\"\nmodel = \"\"\neffort = \"\"\nplugins = []\n";
    let home = agent(&s.dir, "old", old, "Still.");
    fs::write(home.join("CHARACTER.md"), "Kept.").unwrap();
    let mut read = s.cooee.json("/personas/old");
    let version = read.as_object_mut().unwrap().remove("version");
    assert!(version.is_some_and(|v| v.is_string()));
    assert_eq!(
        read,
        json!({ "id": "old", "name": "Old", "character": "Kept.", "instructions": "Still.", "folder": "scratch",
                "agent": "fake-acp", "model": "", "effort": "", "machine": "", "plugins": [], "can_message": [],
                "pinned": false, "role": "", "style": "chat", "fleets": [] })
    );
    assert_eq!(s.cooee.json("/personas/coder")["machine"], "");
    let (plain, _, _) = s.start("old");
    assert_eq!(s.row(&plain)["machine"], "");
    (home, read)
}

fn an_unknown_machine_is_refused(s: &Setup, read: &Value) {
    let mut nowhere = read.clone();
    nowhere["machine"] = json!("nowhere");
    let (status, why) = s.call("PUT", "/personas/old", Some(&nowhere));
    assert_eq!(status, 400, "{why}");
    assert!(why.to_string().contains("no machine nowhere"), "{why}");
    assert_eq!(s.cooee.json("/personas/old")["machine"], "box");
    let rows = s.cooee.json("/threads").as_array().unwrap().len();
    let new =
        json!({ "agent": "fake-acp", "text": "hi", "random_id": "far", "machine": "nowhere" });
    let (status, why) = s.call("POST", "/threads", Some(&new));
    assert_eq!(status, 400, "{why}");
    assert_eq!(s.cooee.json("/threads").as_array().unwrap().len(), rows);
}

fn assert_machines_list_local_and_box(s: &Setup) {
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
        (&json!("box"), &json!(false), &json!(1_700_000_000u64))
    );
}

#[test]
fn a_persona_and_its_threads_name_their_machine() {
    let s = setup("persona-machine");
    box_offline_and_local_busy(&s);
    let (home, read) = old_persona_with_its_character_kept(&s);

    let mut on_box = read.clone();
    on_box["machine"] = json!("box");
    let (status, saved) = s.call("PUT", "/personas/old", Some(&on_box));
    assert_eq!((status, &saved["machine"]), (200, &json!("box")), "{saved}");
    let file = fs::read_to_string(home.join("agent.toml")).unwrap();
    let character = fs::read_to_string(home.join("CHARACTER.md")).unwrap();
    assert!(
        file.contains("machine = \"box\"") && character == "Kept.",
        "{file}"
    );
    assert_eq!(s.cooee.json("/personas/old")["machine"], "box");
    let (id, _, _) = s.start("old");
    assert_eq!(s.row(&id)["machine"], "box");

    let new = json!({ "agent": "fake-acp", "profile": "old", "text": "hi", "random_id": "here", "machine": "" });
    let (status, body) = s.call("POST", "/threads", Some(&new));
    assert_eq!(status, 200, "{body}");
    assert_eq!(s.row(body["id"].as_str().unwrap())["machine"], "");
    assert_eq!(s.cooee.json("/personas/old")["machine"], "box");

    an_unknown_machine_is_refused(&s, &read);

    let (status, cleared) = s.call("PUT", "/personas/old", Some(&read));
    assert_eq!(
        (status, &cleared["machine"]),
        (200, &json!("")),
        "{cleared}"
    );
    let (id, _, _) = s.start("old");
    assert_eq!(s.row(&id)["machine"], "");

    assert_machines_list_local_and_box(&s);
}
