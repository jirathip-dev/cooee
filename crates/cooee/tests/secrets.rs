//! #167: secrets live in keychain items only the signed cooee reads, across rebuilds, with no prompt.
#![cfg(target_os = "macos")]

mod support;

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use support::scratch;

/// The Apple Development identity install.sh signs with, if the keychain has one.
fn identity() -> Option<String> {
    let out = Command::new("security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let line = text.lines().find(|l| l.contains("\"Apple Development: "))?;
    Some(line.split('"').nth(1)?.to_owned())
}

/// A copy of the built cooee at `dir/name`, signed with `id` and `extra` codesign flags when `id` is set.
fn copy(dir: &Path, name: &str, id: Option<&str>, extra: &[&str]) -> PathBuf {
    let path = dir.join(name);
    fs::copy(env!("CARGO_BIN_EXE_cooee"), &path).unwrap();
    if let Some(id) = id {
        let mut sign = Command::new("codesign");
        sign.args(["--force", "--sign", id, "--identifier", "cooee"]);
        let ok = sign.args(extra).arg(&path).status().unwrap();
        assert!(ok.success(), "codesign {name}");
    }
    path
}

fn secrets(bin: &Path, config: &Path) -> Output {
    let mut cmd = Command::new(bin);
    cmd.args(["secrets", "--config"])
        .arg(config)
        .output()
        .unwrap()
}

fn text(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned() + &String::from_utf8_lossy(&o.stderr)
}

/// The migration moves the file's secret into the keychain and shreds the file; a rebuilt signed cooee (a new
/// cdhash, same identity) reads it with prompts off; an unsigned build and an agent shell's `security` can't.
#[test]
fn only_the_signed_cooee_reads_its_secrets_across_rebuilds() {
    let Some(id) = identity() else {
        eprintln!("skipped: no Apple Development signing identity on this Mac");
        return;
    };
    let dir = scratch("secrets-signed");
    let data = dir.join("data");
    fs::create_dir_all(data.join("secrets")).unwrap();
    let value = format!("v-{}", std::process::id());
    fs::write(data.join("secrets/TOKEN"), format!("{value}\n")).unwrap();
    let config = dir.join("config.toml");
    fs::write(&config, format!("port = 1\ndata = {data:?}\n")).unwrap();
    let b1 = copy(&dir, "b1", Some(&id), &[]);
    let b2 = copy(&dir, "b2", Some(&id), &["--options", "runtime"]);
    let unsigned = copy(&dir, "u", None, &[]);
    let account = format!(
        "{}-TOKEN",
        &cooee_core::sha256(data.as_os_str().as_encoded_bytes())[..16]
    );

    let first = secrets(&b1, &config);
    let out = text(&first);
    assert!(first.status.success(), "{out}");
    assert!(
        out.contains("moved TOKEN into the keychain") && out.contains("ok   TOKEN"),
        "{out}"
    );
    assert!(!data.join("secrets/TOKEN").exists(), "the file is shredded");
    assert!(!out.contains(&value), "{out}");

    let rebuilt = secrets(&b2, &config);
    assert!(rebuilt.status.success(), "{}", text(&rebuilt));
    assert!(text(&rebuilt).contains("ok   TOKEN"), "{}", text(&rebuilt));

    let refused = secrets(&unsigned, &config);
    assert!(!refused.status.success(), "{}", text(&refused));
    assert!(
        text(&refused).contains("FAIL TOKEN: the keychain refused cooee"),
        "{}",
        text(&refused)
    );

    // An agent shell: `security` is no trusted app, so it gets no value (killed if it waits on a dialog).
    let shell = Command::new("perl")
        .args([
            "-e",
            "alarm 3; exec @ARGV",
            "security",
            "find-generic-password",
        ])
        .args(["-s", "cooee-secret", "-a", &account, "-w"])
        .output()
        .unwrap();
    assert!(!shell.status.success(), "{}", text(&shell));
    assert!(!text(&shell).contains(&value), "the agent shell read it");

    let gone = Command::new("security")
        .args([
            "delete-generic-password",
            "-s",
            "cooee-secret",
            "-a",
            &account,
        ])
        .output()
        .unwrap();
    assert!(gone.status.success(), "{}", text(&gone));
}
