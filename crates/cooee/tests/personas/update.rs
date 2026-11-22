use serde_json::{Value, json};

use crate::setup::{cook, cook_text, items, setup};

#[test]
fn a_setup_chat_interviews_and_its_update_persona_saves_at_once_for_the_next_session() {
    let s = setup("personas-setup");
    let new = json!({ "name": "Cook", "agent": "fake-acp", "folder": "scratch" });
    assert_eq!(s.call("POST", "/personas", Some(&new)).0, 200);

    let start = json!({ "agent": "fake-acp", "profile": "cook", "text": "hi", "random_id": "setup-1", "setup": true });
    let (status, body) = s.call("POST", "/threads", Some(&start));
    assert_eq!(status, 200, "{body}");
    let id = body["id"].as_str().unwrap().to_owned();
    assert_eq!(s.row(&id)["setup"], true);
    let sent = s.shed.sent(&format!("cooee-{id}"));
    let prompt: Value = serde_json::from_str(sent[0].trim_start_matches('\x1e')).unwrap();
    let prompt = prompt["prompt"].as_str().unwrap();
    assert!(
        prompt.contains("This is a setup chat. Interview the owner, one question at a time")
            && prompt.ends_with("\n\nhi"),
        "{prompt}"
    );

    let args = json!({ "character": "Calm and warm.", "instructions": "Answer in one line." });
    let said = s.tool((&id, &id), "update_persona", &args);
    assert_eq!(said["isError"], false, "{said}");
    assert_eq!(
        cook_text(&s),
        (json!("Calm and warm."), json!("Answer in one line."))
    );
    let lines: Vec<Value> = items(&s, &id)
        .iter()
        .filter(|i| i["kind"] == "note")
        .map(|i| i["text"].clone())
        .collect();
    assert_eq!(
        lines,
        [json!("Updated Character"), json!("Updated Instructions")]
    );

    let (next, _, brief) = s.start("cook");
    assert!(
        brief.starts_with("Calm and warm.\n\nAnswer in one line.\n\n"),
        "{brief}"
    );
    assert_eq!(s.row(&next)["setup"], false);

    let plain = json!({ "agent": "fake-acp", "text": "hi", "random_id": "setup-2", "setup": true });
    let (status, body) = s.call("POST", "/threads", Some(&plain));
    assert_eq!(status, 400, "{body}");
}

#[test]
fn outside_setup_update_persona_is_a_card_saved_only_on_approve_and_never_for_another_persona() {
    let s = setup("personas-update");
    let id = cook(&s);
    let before = cook_text(&s);

    let args = json!({ "character": "Dry wit." });
    let said = s.tool((&id, &id), "update_persona", &args);
    assert_eq!(said["isError"], false, "{said}");
    assert_eq!(cook_text(&s), before);
    let card = |n: usize| {
        let cards: Vec<Value> = items(&s, &id)
            .into_iter()
            .filter(|i| i["form"] == "persona")
            .collect();
        cards[n].clone()
    };
    let first = card(0);
    assert_eq!(first["tool"], "update Cook's Character");
    assert_eq!(first["detail"], "Character\n+Dry wit.");
    let answer = |item: &Value, option: &str| {
        let path = format!("/threads/{id}/approve");
        s.call(
            "POST",
            &path,
            Some(&json!({ "item": item["id"], "option": option })),
        )
    };

    let (status, body) = answer(&first, "deny");
    assert_eq!(status, 200, "{body}");
    assert_eq!(cook_text(&s), before);
    assert_eq!(card(0)["line"], "denied");

    let said = s.tool((&id, &id), "update_persona", &args);
    assert_eq!(said["isError"], false, "{said}");
    let (status, body) = answer(&card(1), "approve");
    assert_eq!(status, 200, "{body}");
    assert_eq!(cook_text(&s).0, "Dry wit.");
    assert_eq!(card(1)["line"], "updated Character");

    let baker = json!({ "name": "Baker", "agent": "fake-acp", "folder": "scratch" });
    assert_eq!(s.call("POST", "/personas", Some(&baker)).0, 200);
    let theirs = s.start("baker").0;
    let other = json!({ "persona": "baker", "character": "Loud." });
    let named = s.tool((&id, &id), "update_persona", &other);
    let text = |v: &Value| v["content"][0]["text"].as_str().unwrap().to_owned();
    assert_eq!(named["isError"], true, "{named}");
    assert!(text(&named).contains("unknown field `persona`"), "{named}");
    let stolen = s.tool(
        (&id, &theirs),
        "update_persona",
        &json!({ "character": "Loud." }),
    );
    assert_eq!(stolen["isError"], true, "{stolen}");
    assert!(text(&stolen).contains("isn't thread"), "{stolen}");
    assert_eq!(s.cooee.json("/personas/baker")["character"], "");
    assert_eq!(cook_text(&s).0, "Dry wit.");
    assert!(items(&s, &theirs).iter().all(|i| i["form"] != "persona"));
}
