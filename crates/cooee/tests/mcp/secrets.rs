use std::{fs, os::unix::fs::PermissionsExt};

use serde_json::{Value, json};

use crate::setup::{DROPPED, drop_setup};
use crate::support::agent;

#[test]
fn a_secret_dropped_once_is_typed_into_its_terminal_and_reaches_no_transcript_event_file_log_or_model()
 {
    let mut d = drop_setup("mcp-secret-once");
    let own = d.agent.run(
        "secret",
        &json!({ "name": "SITE_PASSWORD", "terminal": format!("cooee-{}", d.id) }),
    );
    assert!(own.unwrap().contains("an agent's session"));
    let bad = d.agent.run(
        "secret",
        &json!({ "name": "not a var", "terminal": "login" }),
    );
    assert!(bad.unwrap().contains("like a variable"));
    let mut events = d.events();
    let ask = json!({ "name": "SITE_PASSWORD", "why": "sign in to the vendor portal", "terminal": "login" });
    let said = d.agent.run("secret", &ask).unwrap();
    assert!(
        said.starts_with("Asked the owner for secret SITE_PASSWORD"),
        "{said}"
    );
    let card = d.card();
    assert!(
        card["tool"]
            .as_str()
            .unwrap()
            .ends_with("typed once, never stored"),
        "{card}"
    );
    assert_eq!(card["block"]["kind"], "secret", "{card}");

    assert_eq!(d.drop_it(&card, &Value::Null).0, 403, "no Face ID");
    assert_eq!(d.s.shed.sent("login"), Vec::<String>::new());
    let (status, body) = d.drop_it(&card, &d.phone.get(&d.s.cooee));
    assert_eq!(status, 200, "{body}");
    let line = serde_json::from_str::<Value>(&body).unwrap()["line"].clone();
    assert_eq!(line, "secret SITE_PASSWORD entered");
    assert_eq!(d.s.shed.sent("login"), [DROPPED]);
    assert_eq!(d.s.shed.calls("keys", "login").len(), 1, "then enter");
    let seen = events.until(|e| e.data.to_string().contains("SITE_PASSWORD entered"));
    let told = d.told();
    assert!(
        told.iter()
            .any(|t| t.contains("secret SITE_PASSWORD entered")),
        "{told:?}"
    );
    assert!(!d.leaks(&seen).contains(DROPPED), "the value leaked");
    assert!(
        d.s.dir
            .join("data/secrets")
            .read_dir()
            .unwrap()
            .all(|e| e.unwrap().file_name() == "TOKEN"),
        "never stored"
    );

    let again = d.agent.run("secret", &ask).unwrap();
    assert!(
        again.starts_with("Asked the owner"),
        "a once secret isn't kept: {again}"
    );
}

#[cfg(not(target_os = "macos"))]
#[test]
fn a_saved_secret_is_typed_again_without_asking_and_reaches_the_personas_environment_only() {
    let mut d = drop_setup("mcp-secret-saved");
    let mut events = d.events();
    let ask =
        json!({ "name": "SITE_KEY", "why": "the vendor API", "terminal": "login", "save": true });
    d.agent.run("secret", &ask).unwrap();
    let card = d.card();
    assert!(
        card["tool"]
            .as_str()
            .unwrap()
            .ends_with("saved in the keychain for coder"),
        "{card}"
    );
    let (status, body) = d.drop_it(&card, &d.phone.get(&d.s.cooee));
    assert_eq!(status, 200, "{body}");
    let line = serde_json::from_str::<Value>(&body).unwrap()["line"].clone();
    assert_eq!(line, "secret SITE_KEY entered");
    let seen = events.until(|e| e.data.to_string().contains("SITE_KEY entered"));
    assert!(!d.leaks(&seen).contains(DROPPED), "the value leaked");
    let file = d.s.dir.join("data/secrets/coder.SITE_KEY");
    assert_eq!(fs::read_to_string(&file).unwrap(), DROPPED);
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );

    let said = d.agent.run("secret", &ask).unwrap();
    assert_eq!(said, "secret SITE_KEY entered");
    assert_eq!(d.s.shed.sent("login"), [DROPPED, DROPPED]);
    assert_eq!(d.card()["id"], card["id"], "no second card");

    let (coder, _) = d.s.thread("coder");
    assert!(
        d.s.shed
            .adapter_env_of(&format!("cooee-{coder}"))
            .contains(&format!("SITE_KEY={DROPPED}"))
    );
    let writer = "name = \"Writer\"\nagent = \"fake-acp\"\nenv = { MODE = \"write\" }\n";
    agent(&d.s.dir, "writer", writer, "");
    let (writer, _) = d.s.thread("writer");
    let env = d.s.shed.adapter_env_of(&format!("cooee-{writer}"));
    assert!(env.lines().any(|l| l == "MODE=write"), "{env}");
    assert!(!env.contains(DROPPED), "only coder's");
}
