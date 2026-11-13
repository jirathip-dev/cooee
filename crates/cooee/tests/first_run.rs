//! A fresh instance: with no agents in shed and no `roots`, `cooee doctor` warns about both; with no roots, a chat
//! still starts (in its scratch folder) and Use a folder… is refused; a provider whose CLI is missing reports its install line; with one root, a
//! thread starts in the root.

mod support;

use std::{fs, process::Command};

use serde_json::json;
use support::{Cooee, FakeShed, scratch, version};

#[test]
fn no_agents_and_no_roots_show_the_fix_lines_and_doctor_warns() {
    let dir = scratch("first-run");
    let shed = FakeShed::start(&dir.join("shed"), version());
    // A shed with nothing installed: `shed agents` lists none.
    fs::write(shed.dir.join("shed"), "#!/bin/sh\necho '[]'\n").unwrap();
    let codex = "[[providers]]\nname = \"codex\"\nagent = \"codex-acp\"\nlogin = [\"no-such-codex\"]\n\
                 status = [\"true\"]\ninstall = \"npm install -g @openai/codex\"\n";
    let cooee = Cooee::start_with(&dir, &shed.dir, &format!("roots = []\n{codex}"));

    // codex's CLI is missing: `not installed` with its install line; not signed in.
    let v = cooee.json("/logins/codex");
    assert_eq!(
        v["status"]["install"], "npm install -g @openai/codex",
        "{v}"
    );
    assert_eq!(v["status"]["line"], "not installed on this Mac", "{v}");
    assert_eq!(v["status"]["signed_in"], false, "{v}");

    let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
        .arg("--config")
        .arg(&cooee.config)
        .arg("doctor")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("warn no agents installed in shed\n     fix: shed agents add claude-acp\n")
            && stdout.contains(
                "warn no roots configured\n     fix: add roots = [\"~/Projects\"] to config.toml\n"
            ),
        "{stdout}"
    );
}

#[test]
fn with_one_root_and_nothing_in_it_a_thread_starts_in_the_root() {
    let dir = scratch("first-run-root");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let root = dir.join("only");
    fs::create_dir_all(&root).unwrap();
    let cooee = Cooee::start_with(
        &dir,
        &shed.dir,
        &format!("roots = [{:?}]\n", root.to_string_lossy()),
    );

    // The root is the one folder offered.
    assert_eq!(cooee.json("/folders")["roots"], json!([root]));
    let new = json!({ "agent": "fake-acp", "folder": root, "text": "hi", "random_id": "r0" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let cwd = format!("--cwd {} ", root.canonicalize().unwrap().display());
    assert!(shed.cli_log().contains(&cwd), "{}", shed.cli_log());
}

#[test]
fn with_no_roots_a_chat_still_starts_in_scratch_and_use_a_folder_is_refused() {
    let dir = scratch("first-run-noroots");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start_with(&dir, &shed.dir, "roots = []\n");
    assert_eq!(cooee.json("/folders")["roots"], json!([]));
    let new = json!({ "agent": "fake-acp", "text": "hi", "random_id": "r0" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"].clone();
    let own = dir.join("data/scratch").join(id.as_str().unwrap());
    let cwd = format!("--cwd {} ", own.display());
    assert!(shed.cli_log().contains(&cwd), "{}", shed.cli_log());
    // Use a folder… has no root to take one from.
    let attach = json!({ "folder": dir });
    let (status, body) = cooee.post(
        &format!("/threads/{}/folder", id.as_str().unwrap()),
        &attach,
    );
    assert!(
        status == 400 && body.contains("not inside a configured root"),
        "{status} {body}"
    );
}
