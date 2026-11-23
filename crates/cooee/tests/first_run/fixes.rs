use std::{fs, process::Command};

use crate::support::{Cooee, FakeShed, scratch, version};

#[test]
fn no_agents_and_no_roots_show_the_fix_lines_and_doctor_warns() {
    let dir = scratch("first-run");
    let shed = FakeShed::start(&dir.join("shed"), version());
    fs::write(shed.dir.join("shed"), "#!/bin/sh\necho '[]'\n").unwrap();
    let codex = "[[providers]]\nname = \"codex\"\nagent = \"codex-acp\"\nlogin = [\"no-such-codex\"]\n\
                 status = [\"true\"]\ninstall = \"npm install -g @openai/codex\"\n";
    let cooee = Cooee::start_with(&dir, &shed.dir, &format!("roots = []\n{codex}"));

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
