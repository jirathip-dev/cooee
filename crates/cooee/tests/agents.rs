//! Agent folders (#359) through the binary: one written by hand is listed and starts a chat with its instructions; a
//! malformed `agent.toml` hides only that agent and the log names its file, never its text; `cooee config migrate
//! --dry-run` prints the plan and changes no file of either tree.

mod support;

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use cooee_core::agents::PLAIN;
use serde_json::{Value, json};
use support::{Cooee, FakeShed, agent, scratch, version};

#[test]
fn a_folder_written_by_hand_is_listed_and_starts_a_chat() {
    let dir = scratch("agents-by-hand");
    agent(
        &dir,
        "kai",
        "name = \"Kai\"\nagent = \"fake-acp\"\n",
        "Answer in one paragraph.\n",
    );
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let list = cooee.json("/personas");
    assert_eq!(
        (
            &list[0]["id"],
            &list[0]["name"],
            list.as_array().unwrap().len()
        ),
        (&json!("kai"), &json!("Kai"), 1),
        "{list}"
    );
    let new = json!({ "agent": "other-acp", "profile": "kai", "text": "hi", "random_id": "r1" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let line = shed.cli_log();
    let line = line.lines().rfind(|l| l.contains(" run ")).unwrap();
    assert!(line.contains(" --acp fake-acp "), "{line}");
    let sent = shed.sent(&format!("cooee-{id}"));
    let prompt: Value = serde_json::from_str(sent[0].trim_start_matches('\x1e')).unwrap();
    let brief = format!("Answer in one paragraph.\n\n{PLAIN}\n\nhi");
    assert_eq!(prompt["prompt"], brief);
}

#[test]
fn a_malformed_agent_toml_hides_only_that_agent_and_the_log_never_quotes_it() {
    let dir = scratch("agents-malformed");
    let marker = "marker-7f3a9c";
    agent(&dir, "good", "name = \"Good\"\nagent = \"fake-acp\"\n", "");
    let bad = agent(
        &dir,
        "bad",
        &format!("name = \"Bad\"\nagent = \"fake-acp\"\nfolder = [\"{marker}\"\n"),
        "",
    );
    let typed = agent(
        &dir,
        "typed",
        &format!("name = \"Typed\"\nagent = 7\nmodel = \"{marker}\"\n"),
        "",
    );
    let secret = format!("sk-{}", "z9".repeat(16));
    let leak = agent(
        &dir,
        "leak",
        &format!("name = \"Leak\"\nagent = \"fake-acp\"\n[env]\nT = \"{secret}\"\n"),
        "",
    );
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let list = cooee.json("/personas");
    let ids: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["good"], "{list}");
    let log = fs::read_to_string(dir.join("cooee.err")).unwrap();
    for (home, says) in [(&bad, "line 3"), (&typed, "line 2"), (&leak, "env T")] {
        let line = format!("{}: {says}", home.join("agent.toml").display());
        assert!(log.contains(&line), "{line} not in {log}");
    }
    assert!(!log.contains(marker) && !log.contains(&secret), "{log}");
    let new = json!({ "agent": "fake-acp", "profile": "bad", "text": "hi", "random_id": "r1" });
    assert_eq!(cooee.post("/threads", &new).0, 400);
}

fn tree(dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            tree(&p, out);
        } else {
            out.insert(p.clone(), fs::read(&p).unwrap());
        }
    }
}

#[test]
fn a_dry_run_prints_the_plan_and_changes_nothing() {
    let dir = scratch("agents-dry-run");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../cooee-core/tests/fixtures/upgrade/0.1.0-personas");
    let (config, data) = (dir.join("config"), dir.join("data"));
    for (from, to) in [
        (fixture.join("config"), &config),
        (fixture.join("data"), &data),
    ] {
        let mut files = BTreeMap::new();
        tree(&from, &mut files);
        for (p, bytes) in files {
            let to = to.join(p.strip_prefix(&from).unwrap());
            fs::create_dir_all(to.parent().unwrap()).unwrap();
            fs::write(to, bytes).unwrap();
        }
    }
    let file = config.join("config.toml");
    let text = fs::read_to_string(&file).unwrap();
    fs::write(
        &file,
        format!("{text}data = {:?}\n", data.to_string_lossy()),
    )
    .unwrap();
    let mut before = BTreeMap::new();
    tree(&dir, &mut before);

    let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
        .args(["config", "migrate", "--dry-run", "--config"])
        .arg(&file)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let today = cooee_core::routines::local(u64::try_from(ms).unwrap(), 0);
    let plan = cooee_core::migrate::plan(&config, &data, &today[..10]);
    let printed = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        printed.lines().collect::<Vec<_>>(),
        plan.lines(),
        "{printed}"
    );
    assert!(
        printed.contains("write  agents/kai/agent.toml") && printed.contains("backup personas -> "),
        "{printed}"
    );

    let mut after = BTreeMap::new();
    tree(&dir, &mut after);
    assert_eq!(after, before, "a dry run changed a file");
    assert!(!config.join("agents").exists());
}
