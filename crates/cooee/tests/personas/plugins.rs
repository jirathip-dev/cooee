use serde_json::json;

use crate::setup::{install_drover, setup};

#[test]
fn a_plugin_is_attached_to_a_persona_and_detached() {
    let s = setup("personas-plugins");
    install_drover(&s);
    let new = json!({ "name": "Chief", "agent": "fake-acp" });
    let (status, mut chief) = s.call("POST", "/personas", Some(&new));
    assert_eq!((status, &chief["plugins"]), (200, &json!([])), "{chief}");

    chief["plugins"] = json!(["drover"]);
    let (status, saved) = s.call("PUT", "/personas/chief", Some(&chief));
    assert_eq!(status, 200, "{saved}");
    chief["version"] = saved["version"].clone();
    assert_eq!(
        s.cooee.json("/personas/chief")["plugins"],
        json!(["drover"])
    );
    assert_eq!(s.cooee.json("/personas/coder")["plugins"], json!([]));
    assert_eq!(s.cooee.json("/personas/chief/places"), json!([]));
    assert_eq!(s.call("GET", "/personas/nobody/places", None).0, 400);

    chief["plugins"] = json!(["drover", "nope"]);
    let (status, why) = s.call("PUT", "/personas/chief", Some(&chief));
    assert_eq!(status, 400, "{why}");
    assert!(
        why.as_str().unwrap().contains("no installed plugin nope"),
        "{why}"
    );
    assert_eq!(
        s.cooee.json("/personas/chief")["plugins"],
        json!(["drover"])
    );

    chief["plugins"] = json!([]);
    assert_eq!(s.call("PUT", "/personas/chief", Some(&chief)).0, 200);
    assert_eq!(s.cooee.json("/personas/chief")["plugins"], json!([]));
}

#[test]
fn the_plugin_attaches_by_role() {
    let s = setup("personas-role");
    install_drover(&s);
    let new = json!({ "name": "Conductor", "agent": "fake-acp" });
    let (status, mut p) = s.call("POST", "/personas", Some(&new));
    assert_eq!((status, &p["plugins"]), (200, &json!([])), "{p}");
    p["role"] = json!("orchestrator");
    assert_eq!(s.call("PUT", "/personas/conductor", Some(&p)).0, 200);
    let read = s.cooee.json("/personas/conductor");
    assert_eq!(read["plugins"], json!(["drover"]));
    let mut read = read;
    read["role"] = json!("");
    read["plugins"] = json!([]);
    assert_eq!(s.call("PUT", "/personas/conductor", Some(&read)).0, 200);
    assert_eq!(s.cooee.json("/personas/conductor")["plugins"], json!([]));
    read["role"] = json!("boss");
    assert!(s.call("PUT", "/personas/conductor", Some(&read)).0 >= 400);
}
