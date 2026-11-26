use std::process::Command;

use serde_json::json;

use crate::setup::{checked, setup};

#[test]
fn plugin_check_passes_the_fixtures_and_names_a_broken_field() {
    let design = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/design");
    let mut files: Vec<_> = [
        "screens-as-data",
        "fleets-config",
        "queue-scene",
        "workflow-diagram",
    ]
    .iter()
    .flat_map(|d| std::fs::read_dir(design.join(d).join("fixtures")).unwrap())
    .map(|f| f.unwrap().path())
    .collect();
    files.sort();
    let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
        .args(["plugin", "check"])
        .args(&files)
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success()
            && said.lines().count() == 26
            && said.lines().all(|l| l.starts_with("ok")),
        "{said}"
    );

    let s = setup("card-check");
    let broken = json!({ "type": "screen", "title": "Queue", "blocks": [
        { "type": "list", "rows": [{ "type": "row", "title": "#722", "badge": { "text": "impl", "tone": "red" } }] },
        { "type": "secret", "name": "GH_TOKEN", "action": "set", "value": "leaked" },
        { "type": "editor", "name": "toml", "text": "" },
        { "type": "a-newer-block" },
        { "type": "scene", "lanes": [{ "id": "l-1", "title": "#1", "state": "lost" }] },
        { "type": "row", "title": "#2", "rounds": [{ "id": "r-1", "state": "done" }] } ] });
    let (ok, said) = checked(&s, &broken);
    assert!(!ok);
    assert_eq!(
        said.lines().skip(1).map(str::trim).collect::<Vec<_>>(),
        [
            "$.blocks[2].action: is required",
            "$.blocks[5].rounds[0].name: is required",
        ]
    );
}

#[test]
fn plugin_check_knows_swipe_menu_and_refresh() {
    let s = setup("additions-check");
    let act = json!({ "label": "Archive", "action": "archive", "tone": "bad" });
    let good = json!({ "type": "screen", "title": "Queue", "refresh": true,
        "actions": [{ "label": "More", "icon": "more", "menu": [act] }], "blocks": [
        { "type": "list", "rows": [{ "type": "row", "id": "1", "title": "#1", "swipe": [act], "menu": [act] }] },
        { "type": "card", "title": "Merge?", "menu": [act] } ] });
    let (ok, said) = checked(&s, &good);
    assert!(ok, "{said}");
    let bad = json!({ "type": "screen", "title": "Queue", "refresh": "yes", "blocks": [
        { "type": "row", "title": "#1", "swipe": [{ "action": "archive" }], "menu": { "label": "Archive" } },
        { "type": "card", "menu": [{ "label": "Pin", "tone": "red" }] } ] });
    let (ok, said) = checked(&s, &bad);
    assert!(!ok);
    assert_eq!(
        said.lines().skip(1).map(str::trim).collect::<Vec<_>>(),
        [
            "$.blocks[0].swipe[0].label: is required",
            "$.blocks[0].menu: an object is not array or null",
            "$.refresh: \"yes\" is not boolean or null",
        ]
    );
}

#[test]
fn plugin_check_checks_a_manifest() {
    let s = setup("manifest-check");
    let file = s.dir.join("cooee-plugin.toml");
    let check = |text: &str| {
        std::fs::write(&file, text).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
            .args(["plugin", "check"])
            .arg(&file)
            .output()
            .unwrap();
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };
    let ok = "name = \"drover\"\nversion = \"1\"\nbuild = \"true\"\nrun = \"true\"\n\
              attach = \"http://127.0.0.1:7777\"\n[[places]]\ntype = \"menu-row\"\nlabel = \"drover\"\n\
              screen = \"/screens/fleets\"\n[permissions]\nplaces = [\"menu-row\"]\n";
    assert!(check(ok).0, "{}", check(ok).1);
    let (passed, said) = check(&ok.replace("127.0.0.1:7777", "example.com"));
    assert!(
        !passed && said.contains("`attach` must be http://127.0.0.1:<port>"),
        "{said}"
    );
}
