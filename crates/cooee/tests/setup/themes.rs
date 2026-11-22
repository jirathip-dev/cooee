use std::fs;

use crate::setup::{Mac, files};

#[test]
fn setup_installs_the_first_party_themes_chosen_with_no_network() {
    let mac = Mac::new("su-themes", true);
    let swag = cooee_core::swag::Swag::pinned();
    fs::create_dir_all(mac.config().parent().unwrap()).unwrap();
    fs::write(
        mac.config(),
        format!("port = {}\nroots = [\"/srv\"]\n", mac.port),
    )
    .unwrap();
    let args = ["setup", "--port", &mac.port.to_string()];
    let out = mac.cooee_typing(&args, Some("kitchen\n\n\n1\n1\n"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    assert!(
        stdout.contains(
            "themes:\n  themes to install, or none [bush garden kitchen]:\n  installed theme farm\n  installed theme scene\n  installed theme terminal\n  installed theme kitchen\n"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("chief:\n  Chief is cooee's own persona: it sets up personas and routines for you.\n  its agent:  1 claude-acp\n  its machine:  1 here\n"),
        "{stdout}"
    );
    let chief = fs::read_to_string(mac.chief()).unwrap();
    assert!(
        chief.contains("agent = \"claude-acp\"") && !chief.contains("machine"),
        "{chief}"
    );
    let themes = mac.config().parent().unwrap().join("themes");
    assert_eq!(
        files(&themes),
        [
            "farm/icon.png",
            "farm/scene.svg",
            "farm/sprites.svg",
            "farm/theme.json",
            "kitchen/icon.png",
            "kitchen/scene.svg",
            "kitchen/sprites.svg",
            "kitchen/theme.json",
            "scene/style.json",
            "terminal/style.json"
        ]
    );
    let lock: serde_json::Value = serde_json::from_slice(
        &fs::read(mac.home.join(".local/share/cooee/cooee/themes.lock")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        lock,
        serde_json::json!([
            { "name": "farm", "source": swag.url, "commit": swag.commit, "listed": true },
            { "name": "kitchen", "source": swag.url, "commit": swag.commit, "listed": true },
            { "name": "scene", "source": "built in", "commit": "", "listed": true },
            { "name": "terminal", "source": "built in", "commit": "", "listed": true }
        ])
    );
    assert!(
        !mac.home
            .join(".local/share/cooee/cooee/themes/kitchen.git")
            .exists()
    );
    let out = mac.setup(&["--yes"]);
    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("themes:\n  installed theme bush\n  installed theme garden\n"),
        "{stdout}"
    );
    assert!(themes.join("bush/theme.json").exists());
    assert!(themes.join("garden/theme.json").exists());
    let out = mac.setup(&["--yes"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("themes: done, skipped\n"));
}
