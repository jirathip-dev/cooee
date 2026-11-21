//! Sample agents (#362) through the binary: `cooee agent add --list` names every folder under examples/agents/ with a
//! line on it; `agent add --from` copies each one byte for byte, offline, and every one is listed as a valid agent;
//! an id that is taken is refused and its folder left as it was; a sample whose MCP server isn't in the config
//! starts, and its first message names the server to connect.

mod support;

use std::{fs, path::Path};

use cooee_core::routines::Schedule;
use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

fn examples() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/agents")
}

fn files(root: &Path, at: &Path, out: &mut Vec<String>) {
    for e in fs::read_dir(at).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            files(root, &p, out);
        } else {
            out.push(p.strip_prefix(root).unwrap().display().to_string());
        }
    }
    out.sort();
}

#[test]
fn every_sample_is_added_listed_and_valid() {
    let dir = scratch("samples-all");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let listed = cooee.cli(&["agent", "add", "--list"]);
    let mut ids: Vec<String> = fs::read_dir(examples())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    ids.sort();
    assert_eq!(ids.len(), 12, "{ids:?}");
    let rows: Vec<(&str, &str)> = listed.lines().map(|l| l.split_once(' ').unwrap()).collect();
    assert_eq!(rows.iter().map(|r| r.0).collect::<Vec<_>>(), ids);
    assert!(rows.iter().all(|r| !r.1.trim().is_empty()), "{listed}");
    for id in &ids {
        let said = cooee.cli(&["agent", "add", "--from", id]);
        assert!(said.contains(&format!("added agent {id}")), "{said}");
        let (src, dst) = (examples().join(id), dir.join("agents").join(id));
        let (mut want, mut got) = (Vec::new(), Vec::new());
        files(&src, &src, &mut want);
        files(&dst, &dst, &mut got);
        assert_eq!(got, want, "{id}");
        for f in &want {
            assert_eq!(
                fs::read(src.join(f)).unwrap(),
                fs::read(dst.join(f)).unwrap()
            );
        }
        assert_eq!(fs::read_to_string(dst.join("memory.md")).unwrap(), "");
        if let Ok(t) = fs::read_to_string(dst.join("routine.toml")) {
            let r: toml::Table = toml::from_str(&t).unwrap();
            Schedule::parse(r["when"].as_str().unwrap()).unwrap();
            assert!(!r["name"].as_str().unwrap().is_empty() && r["prompt"].is_str());
        }
    }
    let list = cooee.json("/personas");
    let mut got: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap())
        .collect();
    got.sort_unstable();
    assert_eq!(got, ids, "{list}");
    let orchestrator = list
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "orchestrator");
    assert_eq!(orchestrator.unwrap()["role"], "orchestrator");
    let err = fs::read_to_string(dir.join("cooee.err")).unwrap();
    assert!(!err.contains("agent.toml"), "{err}");
}

#[test]
fn an_id_that_is_taken_is_refused() {
    let dir = scratch("samples-taken");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    cooee.cli(&["agent", "add", "--from", "writer", "--id", "kai"]);
    let toml = dir.join("agents/kai/agent.toml");
    fs::write(&toml, "name = \"Kai\"\nagent = \"fake-acp\"\n").unwrap();
    cooee.cli(&["agent", "add", "--from", "writer"]);
    for args in [
        ["agent", "add", "--from", "tutor", "--id", "kai"],
        ["agent", "add", "--from", "tutor", "--id", "writer"],
    ] {
        let out = cooee.command().args(args).output().unwrap();
        assert!(!out.status.success(), "{out:?}");
        let why = String::from_utf8(out.stderr).unwrap();
        assert!(why.contains("is taken"), "{why}");
    }
    assert_eq!(
        fs::read_to_string(&toml).unwrap(),
        "name = \"Kai\"\nagent = \"fake-acp\"\n"
    );
    assert!(!dir.join("agents/kai/routine.toml").exists());
    assert!(!dir.join("agents/writer/routine.toml").exists());
}

#[test]
fn a_sample_whose_mcp_server_is_missing_says_what_to_connect_first() {
    let dir = scratch("samples-mcp");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    cooee.cli(&["agent", "add", "--from", "inbox"]);
    let new = json!({ "agent": "fake-acp", "profile": "inbox", "text": "hi", "random_id": "r1" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let sent = shed.sent(&format!("cooee-{id}"));
    let prompt: Value = serde_json::from_str(sent[0].trim_start_matches('\x1e')).unwrap();
    let prompt = prompt["prompt"].as_str().unwrap();
    assert!(
        prompt.contains("aren't connected yet: email. Before anything else, your first reply tells the owner to connect"),
        "{prompt}"
    );
    assert!(
        prompt.contains("create_routine") && prompt.contains("0 8 * * *"),
        "{prompt}"
    );
    assert!(prompt.ends_with("\n\nhi"), "{prompt}");
}
