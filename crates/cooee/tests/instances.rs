//! Named instances: two `cooee serve --instance` on one shed keep their own data folders, threads and devices;
//! `cooee install` writes and loads the launchd or systemd job; `cooee doctor` reports a missing shed.

mod support;

use std::{fs, path::Path, process::Command};

use serde_json::json;
use support::{Cooee, FakeShed, scratch, version};

const BIN: &str = env!("CARGO_BIN_EXE_cooee");

#[test]
fn two_instances_never_see_each_others_threads_or_devices() {
    let dir = scratch("instances");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let a = Cooee::start_instance(&dir, &shed.dir, "cooee");
    let b = Cooee::start_instance(&dir, &shed.dir, "drover");

    let folder = dir.join("root/project");
    let new =
        json!({ "agent": "fake-acp", "folder": folder, "text": "Only on a", "random_id": "r0" });
    let (status, body) = a.post("/threads", &new);
    assert_eq!(status, 200, "{body}");

    let share = dir.join("home/.local/share/cooee");
    assert!(share.join("cooee").is_dir());
    assert!(share.join("drover").is_dir());
    let id = serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let ids = |c: &Cooee| c.json("/threads").to_string();
    assert!(ids(&a).contains(&id));
    let theirs = ids(&b);
    assert!(!theirs.contains(&id), "{theirs}");

    let (devices_a, devices_b) = (a.cli(&["devices"]), b.cli(&["devices"]));
    assert_eq!(
        (devices_a.lines().count(), devices_b.lines().count()),
        (1, 1)
    );
    assert_ne!(devices_a, devices_b);
    let (head, _) = b.raw("GET", "/threads", &[("Cookie", &a.cookie)], "");
    assert!(head.contains(" 401 "), "{head}");
}

fn cooee(home: &Path, args: &[&str]) -> std::process::Output {
    let bin = home.join("bin");
    fs::create_dir_all(&bin).unwrap();
    for (name, body) in [
        ("launchctl", "[ \"$1\" = bootstrap ]"),
        ("systemctl", "true"),
    ] {
        let log = format!("printf '%s\\n' \"{name} $*\" >>\"$HOME/calls.log\"");
        fs::write(bin.join(name), format!("#!/bin/sh\n{log}\n{body}\n")).unwrap();
        fs::set_permissions(
            bin.join(name),
            std::os::unix::fs::PermissionsExt::from_mode(0o755),
        )
        .unwrap();
    }
    Command::new(BIN)
        .args(args)
        .env("HOME", home)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_DATA_HOME")
        .output()
        .unwrap()
}

fn expected_job(home: &Path) -> (std::path::PathBuf, Vec<String>, String) {
    let log = home.join(".local/share/cooee/cooee/cooee.log");
    let uid = String::from_utf8(Command::new("id").arg("-u").output().unwrap().stdout).unwrap();
    if cfg!(target_os = "macos") {
        let plist = home.join("Library/LaunchAgents/dev.cooee.cooee.plist");
        let load = format!("launchctl bootstrap gui/{} {}", uid.trim(), plist.display());
        let wants = vec![
            "<string>dev.cooee.cooee</string>".to_owned(),
            format!(
                "<string>{BIN}</string>\n    <string>serve</string>\n    <string>--instance</string>\n    <string>cooee</string>\n  </array>"
            ),
            "<key>KeepAlive</key>\n  <true/>".to_owned(),
            format!(
                "<key>StandardErrorPath</key>\n  <string>{}</string>",
                log.display()
            ),
        ];
        (plist, wants, load)
    } else {
        let unit = home.join(".config/systemd/user/dev.cooee.cooee.service");
        let wants = vec![
            format!("ExecStart={BIN} serve --instance cooee\n"),
            "Restart=always\n".to_owned(),
            format!("StandardError=append:{}\n", log.display()),
        ];
        let load = "systemctl --user enable --now dev.cooee.cooee.service".to_owned();
        (unit, wants, load)
    }
}

#[test]
fn install_writes_and_loads_the_job() {
    let home = scratch("install");
    let out = cooee(&home, &["install", "--instance", "cooee", "--port", "4799"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");

    let (file, wants, load) = expected_job(&home);
    let text = fs::read_to_string(&file).unwrap();
    for want in wants {
        assert!(text.contains(&want), "{want} not in {text}");
    }
    if cfg!(target_os = "macos") {
        let lint = Command::new("plutil")
            .arg("-lint")
            .arg(&file)
            .output()
            .unwrap();
        assert!(lint.status.success(), "{lint:?}");
    }
    let calls = fs::read_to_string(home.join("calls.log")).unwrap();
    assert!(calls.lines().any(|l| l == load), "{load} not in {calls}");
    assert!(stdout.contains("dev.cooee.cooee written\n"), "{stdout}");
    assert!(!stdout.contains(&load), "{stdout}");
    let config = home.join(".config/cooee/cooee/config.toml");
    assert_eq!(fs::read_to_string(config).unwrap(), "port = 4799\n");
    for want in [
        "tailscale serve --bg --https=4799 http://127.0.0.1:4799".to_owned(),
        format!(
            "shed isn't running: install or update shed to protocol {} or newer from https://github.com/jirathip-dev/shed/releases",
            cooee_core::SHED_MIN_PROTOCOL
        ),
    ] {
        assert!(stdout.contains(&want), "{want} not in {stdout}");
    }

    let out = cooee(&home, &["install", "--port", "4800"]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("port = 4799"),
        "{out:?}"
    );
}

#[test]
fn doctor_reports_a_missing_shed() {
    let home = scratch("doctor");
    let config = home.join(".config/cooee/cooee/config.toml");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    let no_shed = home.join("no-shed");
    fs::write(
        &config,
        format!("port = 1\nshed_dir = {:?}\n", no_shed.to_string_lossy()),
    )
    .unwrap();
    let out = cooee(&home, &["doctor"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        stdout.contains("FAIL shed: ")
            && stdout.contains(&no_shed.join("shed.sock").display().to_string())
            && stdout.contains(
                &format!("fix: install or update shed to protocol {} or newer from https://github.com/jirathip-dev/shed/releases", cooee_core::SHED_MIN_PROTOCOL)
            ),
        "{stdout}"
    );
    assert!(
        stdout.contains("warn apns: no [apns] in config.toml: the native app gets no pushes\n"),
        "{stdout}"
    );

    let apns =
        "[apns]\nkey_id = \"K\"\nteam_id = \"T\"\ntopic = \"dev.cooee.app\"\nenv = \"sandbox\"\n";
    fs::write(&config, format!("port = 1\n{apns}")).unwrap();
    let stdout = String::from_utf8_lossy(&cooee(&home, &["doctor"]).stdout).into_owned();
    let p8 = home.join(".local/share/cooee/cooee/apns.p8");
    let missing = format!(
        "warn apns: no {} (Apple Developer › Keys, APNs)",
        p8.display()
    );
    assert!(stdout.contains(&missing), "{stdout}");
    let key = "MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg/FMQqoOmZU6bop6k\n\
               jwzlkZBiOA5CCW7ngc03CgGpgsKhRANCAASkJsDwiTE2wL6OIdpH5tfnsdyw5UK8\n\
               NOnlJZkFHy9kIEFLJ8uttasmC9v/MsS+Cbb3MJBa25UlVnmKe8xoXskq";
    let p = "PRIVATE KEY-----";
    fs::create_dir_all(p8.parent().unwrap()).unwrap();
    fs::write(&p8, format!("-----BEGIN {p}\n{key}\n-----END {p}\n")).unwrap();
    let stdout = String::from_utf8_lossy(&cooee(&home, &["doctor"]).stdout).into_owned();
    assert!(
        stdout.contains("ok   apns ready\n") && !stdout.contains(&key[..40]),
        "{stdout}"
    );
}
