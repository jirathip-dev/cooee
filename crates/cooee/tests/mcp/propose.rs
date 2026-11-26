use serde_json::{Value, json};

use crate::setup::{Agent, Mcp, named, setup};
use crate::support::Authenticator;

const COOEE_TOOLS: [&str; 12] = [
    "ask_persona",
    "create_routine",
    "desk",
    "handoff",
    "history",
    "mail",
    "propose_server",
    "remember",
    "secret",
    "skill",
    "stop_routine",
    "update_persona",
];

#[test]
fn propose_server_shows_the_exact_command_runs_nothing_and_needs_face_id_to_add_it_off() {
    let s = setup("mcp-propose");
    let (id, servers) = s.thread("");
    let ran = s.dir.join("bin/ran");
    let web = s.dir.join("bin/fake-mcp");
    let mut cooee = Mcp::start(named(&servers, "cooee"), Agent::Codex);
    assert_eq!(cooee.tools(), COOEE_TOOLS);
    let proposal = json!({ "name": "web", "type": "stdio", "command": web, "args": ["--port", "9 9"], "why": "browse the docs" });
    let said = cooee.run("propose_server", &proposal).unwrap();
    assert!(
        said.starts_with("Proposed.") && said.contains("Nothing has run"),
        "{said}"
    );
    let bad = cooee.call("tools/call", &json!({ "name": "propose_server", "arguments": { "name": "fs", "type": "stdio", "command": "x" } }));
    assert!(
        bad.unwrap()["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("already")
    );

    let items = s.cooee.json(&format!("/threads/{id}/messages"));
    let card = items["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["form"] == "mcp")
        .expect("the card")
        .clone();
    let exact = format!("{} --port '9 9'", web.display());
    assert_the_card_shows_the_exact_command(&card, &exact);
    assert!(
        !ran.exists() && !s.ls().contains("web"),
        "nothing runs before approval"
    );

    let approve = |passkey: Value| {
        let values = &card["block"]["blocks"][2]["props"]["values"];
        s.cooee.post(
            &format!("/threads/{id}/approve"),
            &json!({ "item": values["item"], "option": values["choice"], "passkey": passkey }),
        )
    };
    let (status, body) = approve(Value::Null);
    assert_eq!(
        (status, body.as_str()),
        (403, "Approving this needs Face ID.")
    );
    let phone = Authenticator::new(&s.cooee);
    assert_eq!(approve(phone.get(&s.cooee)).0, 403, "an unknown passkey");
    assert!(!s.ls().contains("web"));
    phone.save(&s.cooee);
    let (status, body) = approve(phone.get(&s.cooee));
    assert_eq!(status, 200, "{body}");
    let line = serde_json::from_str::<Value>(&body).unwrap()["line"].clone();
    assert_eq!(line, "added web, off: turn it on in settings › mcp servers");
    let ls = s.ls();
    assert!(ls.contains(&format!("[stdio] web · off · {exact}")), "{ls}");
    assert!(!ran.exists(), "approving adds it off; nothing ran");
}

fn assert_the_card_shows_the_exact_command(card: &Value, exact: &str) {
    let island = &card["block"]["blocks"][2];
    assert_eq!(
        (&island["name"], &island["props"]["mode"], &island["events"]),
        (&json!("device-key"), &json!("approve"), &json!(["answer"]))
    );
    assert_eq!(card["detail"], exact);
    assert_eq!(card["block"]["kind"], "proposal");
    assert_eq!(
        card["block"]["blocks"][0]["markdown"],
        format!("```\n{exact}\n```")
    );
    assert!(
        card["tool"]
            .as_str()
            .unwrap()
            .contains("add MCP server web (stdio): browse the docs"),
        "{card}"
    );
}
