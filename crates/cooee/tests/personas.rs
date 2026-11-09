//! Personas over the API (#126) with the fake shed (tests/support): one is created, edited and deleted; today's
//! profile is read as a persona with nothing lost and runs as that profile; a note by the persona (its `remember`
//! tool through cooee's MCP server) and one by the owner both list and reach its next session; clearing its memory,
//! or deleting it, leaves its chats.

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

    /// The persona of thread `id`'s row in `GET /threads`.
    fn row_persona(&self, id: &str) -> Value {
        let rows = self.cooee.json("/threads");
        let row = rows.as_array().unwrap().iter().find(|r| r["id"] == id);
        row.unwrap_or_else(|| panic!("no row {id}: {rows}"))["persona"].clone()
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

    // Edited: its id stays, the edit is kept and read back.
    let edit = json!({ "name": "Chief", "character": "A calm chef.", "instructions": "Answer in two lines.",
                       "agent": "fake-acp", "folder": "scratch" });
    let (status, edited) = s.call("PUT", "/personas/chief-cook", Some(&edit));
    assert_eq!(status, 200, "{edited}");
    let read = s.cooee.json("/personas/chief-cook");
    assert_eq!(
        (read["name"].as_str(), read["instructions"].as_str()),
        (Some("Chief"), Some("Answer in two lines."))
    );
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
                 "model": "", "effort": "", "profile": "coder" }])
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
