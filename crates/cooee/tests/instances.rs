//! Named instances: two `cooee serve --instance` on one shed keep their own data folders, threads and devices;
//! `cooee install` writes a launchd plist and prints the commands to run; `cooee doctor` reports a missing shed.

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
    let b = Cooee::start_instance(&dir, &shed.dir, "cooee-drover");

    let folder = dir.join("root/project");
    let new =
        json!({ "agent": "fake-acp", "folder": folder, "text": "Only on a", "random_id": "r0" });
    let (status, body) = a.post("/threads", &new);
    assert_eq!(status, 200, "{body}");

    let share = dir.join("home/.local/share/cooee");
    assert!(share.join("cooee").is_dir());
    assert!(share.join("cooee-drover").is_dir());
    let id = serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let link = format!(r#"href="/t/{id}""#);
    assert!(a.get("/").1.contains(&link));
    let (status, page) = b.get("/");
    assert!(status == 200 && !page.contains(&link), "{page}");

    let (devices_a, devices_b) = (a.cli(&["devices"]), b.cli(&["devices"]));
    assert_eq!(
        (devices_a.lines().count(), devices_b.lines().count()),
        (1, 1)
    );
    assert_ne!(devices_a, devices_b);
    // a's cookie is not a device of b's.
    let (head, _) = b.raw("GET", "/", &[("Cookie", &a.cookie)], "");
    assert!(
        head.contains(" 303 ") && head.contains("location: /pair"),
        "{head}"
    );
}

/// `cooee <args>` with `HOME` at `home`, no XDG folders and only the system `PATH` (no shed, no Tailscale).
fn cooee(home: &Path, args: &[&str]) -> std::process::Output {
    Command::new(BIN)
        .args(args)
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_DATA_HOME")
        .output()
        .unwrap()
}

#[test]
fn install_writes_a_plist_and_prints_the_commands() {
    let home = scratch("install");
    let out = cooee(&home, &["install", "--instance", "cooee", "--port", "4799"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");

    let plist = home.join("Library/LaunchAgents/dev.cooee.cooee.plist");
    let text = fs::read_to_string(&plist).unwrap();
    let log = home.join(".local/share/cooee/cooee/cooee.log");
    for want in [
        "<string>dev.cooee.cooee</string>".to_owned(),
        format!(
            "<string>{BIN}</string>\n    <string>serve</string>\n    <string>--instance</string>\n    <string>cooee</string>\n  </array>"
        ),
        "<key>KeepAlive</key>\n  <true/>".to_owned(),
        format!(
            "<key>StandardErrorPath</key>\n  <string>{}</string>",
            log.display()
        ),
    ] {
        assert!(text.contains(&want), "{want} not in {text}");
    }
    if cfg!(target_os = "macos") {
        let lint = Command::new("plutil")
            .arg("-lint")
            .arg(&plist)
            .output()
            .unwrap();
        assert!(lint.status.success(), "{lint:?}");
    }
    let config = home.join(".config/cooee/cooee/config.toml");
    assert_eq!(fs::read_to_string(config).unwrap(), "port = 4799\n");
    for want in [
        format!("launchctl bootstrap gui/$(id -u) {}", plist.display()),
        "tailscale serve --bg --https=4799 http://127.0.0.1:4799".to_owned(),
        "cargo install --locked --git https://github.com/jirathip-dev/shed".to_owned(),
    ] {
        assert!(stdout.contains(&want), "{want} not in {stdout}");
    }

    // The config is the one place the port lives: another port is refused, not silently diverged.
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
                "install: cargo install --locked --git https://github.com/jirathip-dev/shed"
            ),
        "{stdout}"
    );
}
