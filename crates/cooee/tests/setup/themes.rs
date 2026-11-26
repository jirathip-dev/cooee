use std::fs;

use crate::setup::{Mac, files};

#[test]
fn setup_installs_the_style_packs_and_asks_no_theme_pick() {
    let mac = Mac::new("su-themes", true);
    fs::create_dir_all(mac.config().parent().unwrap()).unwrap();
    fs::write(
        mac.config(),
        format!("port = {}\nroots = [\"/srv\"]\n", mac.port),
    )
    .unwrap();
    let args = ["setup", "--port", &mac.port.to_string()];
    let out = mac.cooee_typing(&args, Some("\n\n1\n1\n"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    assert!(
        stdout.contains("themes:\n  installed theme scene\n  installed theme terminal\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("chief:\n  Chief is cooee's own persona: it sets up personas and routines for you.\n  its agent:  1 claude-acp\n  its machine:  1 here\n"),
        "{stdout}"
    );
    let themes = mac.config().parent().unwrap().join("themes");
    assert_eq!(files(&themes), ["scene/style.json", "terminal/style.json"]);
    let lock: serde_json::Value = serde_json::from_slice(
        &fs::read(mac.home.join(".local/share/cooee/cooee/themes.lock")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        lock,
        serde_json::json!([
            { "name": "scene", "source": "built in", "commit": "", "listed": true },
            { "name": "terminal", "source": "built in", "commit": "", "listed": true }
        ])
    );
    let out = mac.setup(&["--yes"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("themes: done, skipped\n"));
}
