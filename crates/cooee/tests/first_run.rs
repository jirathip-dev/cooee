//! A fresh instance: with no agents in shed and no `roots`, the new-thread form says what to run instead of
//! showing empty pickers (and start waits), a provider whose CLI is missing shows its install line, and `cooee doctor`
//! warns about both; with one root, a thread starts in the root itself.

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

    let (status, page) = cooee.get("/new");
    assert_eq!(status, 200, "{page}");
    assert!(
        page.contains("no agents installed · on the Mac:<code>shed agents add claude-acp</code>")
            && page.contains(r#"data-copy="shed agents add claude-acp""#),
        "{page}"
    );
    assert!(
        page.contains(r#"no folders set · add<code>roots = ["~/Projects"]</code>to config.toml"#)
            && page.contains(r#"data-copy="roots = [&quot;~/Projects&quot;]""#),
        "{page}"
    );
    assert!(!page.contains(r#"name="pick""#), "{page}");
    assert!(
        page.contains(r#"<button type="submit" disabled="">send</button>"#),
        "{page}"
    );
    // codex's CLI is missing: one row, `not installed` and [copy install]; no sign-in, no accounts.
    assert!(
        page.contains(r#"<div class="acr ni"><span class="ptg">codex</span><span class="sk">not installed</span><button type="button" class="sec" data-copy="npm install -g @openai/codex" title="npm install -g @openai/codex">copy install</button></div>"#),
        "{page}"
    );
    assert!(
        !page.contains("sign in") && !page.contains("+ account"),
        "{page}"
    );

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

    // The root, after scratch (no folder, picked).
    let (_, page) = cooee.get("/new");
    let option = format!(r#"name="folder" value="{}" data-last="">"#, root.display());
    assert!(
        page.contains(r#"name="folder" value="" checked="""#)
            && page.contains(&option)
            && page.matches(r#"name="folder""#).count() == 2,
        "{page}"
    );
    let new = json!({ "agent": "fake-acp", "folder": root, "text": "hi", "random_id": "r0" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let cwd = format!("--cwd {} ", root.canonicalize().unwrap().display());
    assert!(shed.cli_log().contains(&cwd), "{}", shed.cli_log());
}
