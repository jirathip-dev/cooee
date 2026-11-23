use std::fs;

use serde_json::json;

use crate::setup::{Setup, install_drover, old_install, setup};
use crate::support::{Cooee, FakeShed, scratch, version};

#[test]
fn a_migrated_persona_without_instructions_takes_its_profiles() {
    let dir = scratch("personas-pinstr");
    let (personas, profiles) = (dir.join("personas"), dir.join("profiles"));
    fs::create_dir_all(&personas).unwrap();
    fs::create_dir_all(&profiles).unwrap();
    fs::write(
        profiles.join("coder.toml"),
        "name = \"coder\"\nlabel = \"Coder\"\nagent = \"fake-acp\"\ninstructions = \"coder.md\"\n\
         policy = \"allow\"\n[env]\nMODE = \"review\"\n",
    )
    .unwrap();
    fs::write(profiles.join("coder.md"), "Follow the house rules.\n").unwrap();
    let persona = |id: &str, more: &str| {
        let text = format!(
            "name = \"{id}\"\nagent = \"fake-acp\"\nprofile = \"coder\"\nstyle = \"detailed\"\ncharacter = \"Calm.\"\n{more}"
        );
        fs::write(personas.join(format!("{id}.toml")), text).unwrap();
    };
    persona("lead", "");
    persona("own", "instructions = \"My own rules.\"\n");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let s = Setup { dir, shed, cooee };
    assert_eq!(
        fs::read_to_string(s.dir.join("agents/lead/INSTRUCTIONS.md")).unwrap(),
        "Follow the house rules.\n"
    );
    let note = json!({ "text": "Tea, no sugar." });
    let (status, added) = s.call("POST", "/personas/lead/memory", Some(&note));
    assert_eq!(status, 200, "{added}");
    let id = added["id"].as_str().unwrap();

    let (_, line, brief) = s.start("lead");
    assert!(
        line.contains(" MODE=review "),
        "runs as the profile: {line}"
    );
    assert_eq!(
        brief,
        format!(
            "Calm.\n\nFollow the house rules.\n\nYour notes (edit them with your remember tool):\n- [{id}] Tea, no sugar.\n\nhi"
        )
    );
    assert_eq!(s.start("own").2, "Calm.\n\nMy own rules.\n\nhi");
}

#[test]
fn migration_keeps_the_name_and_sets_the_orchestrator_role() {
    let conductor = "name = \"conductor\"\nlabel = \"Maestro\"\nagent = \"fake-acp\"\n";
    let coder = "name = \"coder\"\nlabel = \"Coder\"\nagent = \"fake-acp\"\n";
    let s = old_install(
        "personas-drover",
        false,
        &[
            ("profiles/conductor.toml", conductor),
            ("profiles/coder.toml", coder),
        ],
    );
    let seen = |s: &Setup, id: &str| {
        let p = s.cooee.json(&format!("/personas/{id}"));
        (p["name"].clone(), p["role"].clone(), p["plugins"].clone())
    };
    assert_eq!(
        seen(&s, "conductor"),
        (json!("Maestro"), json!("orchestrator"), json!([]))
    );
    assert_eq!(seen(&s, "coder"), (json!("Coder"), json!(""), json!([])));
    assert!(s.dir.join("agents/conductor/agent.toml").exists());
    let style = |s: &Setup, id: &str| s.cooee.json(&format!("/personas/{id}"))["style"].clone();
    assert_eq!(style(&s, "conductor"), "detailed");
    assert_eq!(style(&s, "coder"), "chat");
    install_drover(&s);
    assert_eq!(seen(&s, "conductor").2, json!(["drover"]));
    assert_eq!(seen(&s, "coder").2, json!([]));

    let navi = "name = \"Navi\"\nagent = \"fake-acp\"\nplugins = []\n";
    let s = old_install(
        "personas-drover-file",
        true,
        &[
            ("personas/conductor.toml", navi),
            ("profiles/conductor.toml", conductor),
        ],
    );
    assert_eq!(
        seen(&s, "conductor"),
        (json!("Navi"), json!("orchestrator"), json!(["drover"]))
    );
    assert_eq!(style(&s, "conductor"), "detailed");
    fs::write(
        s.dir.join("agents/conductor/agent.toml"),
        "name = \"Navi\"\nrole = \"orchestrator\"\nagent = \"fake-acp\"\nstyle = \"chat\"\n",
    )
    .unwrap();
    assert_eq!(style(&s, "conductor"), "chat");
}

#[test]
fn renaming_the_orchestrator_changes_no_attachment() {
    let s = setup("personas-rename");
    install_drover(&s);
    let new = json!({ "name": "Conductor", "role": "orchestrator", "fleets": ["cooee"], "agent": "fake-acp" });
    let (status, mut p) = s.call("POST", "/personas", Some(&new));
    assert_eq!(status, 200, "{p}");
    let (id, ..) = s.start("conductor");
    p["name"] = json!("Navi");
    assert_eq!(s.call("PUT", "/personas/conductor", Some(&p)).0, 200);
    let read = s.cooee.json("/personas/conductor");
    assert_eq!(
        (
            &read["name"],
            &read["role"],
            &read["plugins"],
            &read["fleets"]
        ),
        (
            &json!("Navi"),
            &json!("orchestrator"),
            &json!(["drover"]),
            &json!(["cooee"])
        )
    );
    let row = s.row(&id);
    assert_eq!(
        (&row["persona"], &row["persona_name"], &row["persona_role"]),
        (&json!("conductor"), &json!("Navi"), &json!("orchestrator"))
    );
}

#[test]
fn two_orchestrator_personas_each_with_their_own_fleets() {
    let s = setup("personas-two");
    install_drover(&s);
    for (name, fleets) in [
        ("Ada", json!(["cooee"])),
        ("Bo", json!(["herdsman", "cooee"])),
    ] {
        let new =
            json!({ "name": name, "role": "orchestrator", "fleets": fleets, "agent": "fake-acp" });
        assert_eq!(s.call("POST", "/personas", Some(&new)).0, 200);
    }
    let all = s.cooee.json("/personas");
    let seen: Vec<_> = all
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["id"].clone(),
                p["role"].clone(),
                p["fleets"].clone(),
                p["plugins"].clone(),
            )
        })
        .collect();
    let drover = json!(["drover"]);
    assert_eq!(
        seen,
        [
            (
                json!("ada"),
                json!("orchestrator"),
                json!(["cooee"]),
                drover.clone()
            ),
            (
                json!("bo"),
                json!("orchestrator"),
                json!(["herdsman", "cooee"]),
                drover
            ),
            (json!("coder"), json!(""), json!([]), json!([])),
        ]
    );
    let (bo, ..) = s.start("bo");
    let (coder, ..) = s.start("coder");
    let (bo, coder) = (s.row(&bo), s.row(&coder));
    assert_eq!(
        (&bo["persona_name"], &bo["persona_role"]),
        (&json!("Bo"), &json!("orchestrator"))
    );
    assert_eq!(
        (&coder["persona_name"], &coder["persona_role"]),
        (&json!("Coder"), &json!(""))
    );
}
