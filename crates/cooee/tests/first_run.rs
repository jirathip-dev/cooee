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

    let (status, page) = cooee.get("/");
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
    assert!(
        !page.contains("<select") && !page.contains(r#"name="agent""#),
        "{page}"
    );
    assert!(
        page.contains(r#"<button type="submit" disabled="">start</button>"#),
        "{page}"
    );
    // codex's CLI is missing: "not installed on this Mac" and its install line; no sign-in, no accounts.
    assert!(
        page.contains(r#"<b>codex</b><span class="m">not installed</span></div><p class="ds">not installed on this Mac</p><p class="hn">on the Mac:<code>npm install -g @openai/codex</code>"#),
        "{page}"
    );
    let codex = &page[page.find(r#"data-provider="codex""#).unwrap()..];
    let codex = &codex[..codex.find(r#"class="lg""#).unwrap()];
    assert!(
        !codex.contains("sign in")
            && !codex.contains("+ account")
            && !codex.contains("class=\"ac\""),
        "{codex}"
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

    let (_, page) = cooee.get("/");
    let option = format!(r#"name="folder" value="{}" checked="#, root.display());
    assert!(
        page.contains(&option) && page.matches(r#"name="folder""#).count() == 1,
        "{page}"
    );
    let new = json!({ "agent": "fake-acp", "folder": root, "text": "hi", "random_id": "r0" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let cwd = format!("--cwd {} ", root.canonicalize().unwrap().display());
    assert!(shed.cli_log().contains(&cwd), "{}", shed.cli_log());
}
