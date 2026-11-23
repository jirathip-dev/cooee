use serde_json::{Value, json};

use crate::{
    setup::{Setup, items, setup},
    support::is_item,
};

const CHOICE: &str = r#"{"blocks": [{"type": "prose", "markdown": "Which plan?"},
    {"type": "choice", "action": "answer", "options": [{"value": "pro", "label": "Pro"}, {"value": "team", "label": "Team"}]}]}"#;

fn persona(s: &Setup, name: &str, blocks: Option<&str>) -> (String, String) {
    let mut new = json!({ "name": name, "agent": "fake-acp", "folder": "scratch" });
    if let Some(b) = blocks {
        new["blocks"] = json!(b);
    }
    let (status, body) = s.call("POST", "/personas", Some(&new));
    assert_eq!(status, 200, "{body}");
    let (id, _, brief) = s.start(body["id"].as_str().unwrap());
    let got = s.cooee.json(&format!("/threads/{id}/messages"));
    if !got["items"].as_array().unwrap().iter().any(|i| i["kind"] == "agent") {
        let mut events = s.cooee.events(&id, got["cursor"].as_str().unwrap(), false);
        events.until(|e| is_item(e, "agent"));
    }
    (id, brief)
}

fn show(s: &Setup, id: &str, reply: &str) -> Value {
    let args: Value = serde_json::from_str(reply).unwrap();
    s.tool((id, id), "show", &args)
}

fn tools(s: &Setup, id: &str) -> Vec<String> {
    let listed = s.rpc((id, id), "tools/list", &json!({}));
    let names = listed["tools"].as_array().unwrap().iter();
    names
        .map(|t| t["name"].as_str().unwrap().to_owned())
        .collect()
}

fn blocks(s: &Setup, id: &str) -> Vec<Value> {
    let all = items(s, id).into_iter().filter(|i| i["kind"] == "blocks");
    all.collect()
}

fn find(v: &Value, id: &str) -> Option<Value> {
    match v {
        Value::Object(o) if o.get("id") == Some(&json!(id)) => Some(v.clone()),
        Value::Object(o) => o.values().find_map(|x| find(x, id)),
        Value::Array(l) => l.iter().find_map(|x| find(x, id)),
        _ => None,
    }
}

fn act(s: &Setup, id: &str, block: &str, action: &str, values: &Value) -> (u16, String) {
    let a = json!({ "screen": format!("/screens/chat/{id}"), "block": block, "action": action, "values": values });
    s.cooee.post("/screens/action", &a)
}

fn told(s: &Setup, id: &str) -> Vec<String> {
    let sent = s.shed.sent(&format!("cooee-{id}"));
    let prompts = sent.iter().map(|t| {
        let v: Value = serde_json::from_str(t.trim_start_matches('\x1e')).unwrap();
        v["prompt"].as_str().unwrap_or_default().to_owned()
    });
    prompts
        .filter(|p| p.starts_with("[The owner answered your blocks]"))
        .collect()
}

#[test]
fn a_choice_is_drawn_and_its_pick_is_the_personas_next_turn_once() {
    let s = setup("personas-show-pick");
    let (id, brief) = persona(&s, "Travel", Some("ask"));
    let catalog = cooee_core::agents::show::catalog("ask").unwrap();
    assert!(brief.contains(&blocks::model::prompt(catalog)), "{brief}");
    assert!(tools(&s, &id).contains(&"show".to_owned()));

    assert_eq!(show(&s, &id, CHOICE)["content"][0]["text"], "Shown.");
    let item = blocks(&s, &id)[0]["id"].as_u64().unwrap();
    let screen = s.cooee.json(&format!("/screens/chat/{id}"));
    let choice = find(&screen, &format!("b{item}.1")).unwrap();
    assert_eq!(choice["type"], "choice", "{screen}");

    let block = format!("b{item}.1");
    let pick = json!({ "choice": "pro" });
    let (status, body) = act(&s, &id, &block, "answer", &pick);
    assert_eq!(status, 200, "{body}");
    assert_eq!(act(&s, &id, &block, "answer", &pick).0, 200);
    assert_eq!(
        told(&s, &id),
        ["[The owner answered your blocks]: answer\n{\"choice\":\"pro\"}"]
    );
    let screen = s.cooee.json(&format!("/screens/chat/{id}"));
    let line = find(&screen, &format!("b{item}-sent")).unwrap();
    assert!(
        line["text"].as_str().unwrap().starts_with("✓ Pro · "),
        "{line}"
    );
    let other = act(&s, &id, &block, "answer", &json!({ "choice": "team" }));
    assert_eq!(other.0, 400, "{}", other.1);
}

#[test]
fn a_value_the_block_does_not_offer_or_a_chat_action_from_a_persona_block_is_refused() {
    let s = setup("personas-show-refused");
    let (id, _) = persona(&s, "Travel", Some("ask"));
    show(&s, &id, CHOICE);
    let block = format!("b{}.1", blocks(&s, &id)[0]["id"]);

    let (status, body) = act(
        &s,
        &id,
        &block,
        "answer",
        &json!({ "choice": "enterprise" }),
    );
    assert_eq!(status, 400, "{body}");
    assert_eq!(act(&s, &id, &block, "archive", &json!({})).0, 400);
    assert_eq!(told(&s, &id), Vec::<String>::new());
    assert_eq!(s.row(&id)["archived"], false);
    assert!(blocks(&s, &id)[0]["sent"].is_null());
}

#[test]
fn a_kind_or_action_outside_the_catalog_is_never_drawn() {
    let s = setup("personas-show-catalog");
    let (shows, _) = persona(&s, "Charts", Some("show"));
    let said = show(&s, &shows, CHOICE);
    assert_eq!(said["isError"], true, "{said}");
    let (asks, _) = persona(&s, "Travel", Some("ask"));
    let wipe = r#"{"blocks": [{"type": "button", "label": "Wipe", "action": "wipe"}]}"#;
    assert_eq!(show(&s, &asks, wipe)["isError"], true);
    let drawn: Vec<Value> = [&shows, &asks]
        .into_iter()
        .flat_map(|id| blocks(&s, id))
        .flat_map(|i| i["blocks"].as_array().unwrap().clone())
        .filter(|b| b["type"] != "prose")
        .collect();
    assert_eq!(drawn, Vec::<Value>::new());
}

#[test]
fn a_reply_refused_twice_is_plain_text_after_exactly_one_retry() {
    let s = setup("personas-show-retry");
    let (id, _) = persona(&s, "Charts", Some("show"));
    let calls = [
        CHOICE,
        CHOICE,
        r#"{"blocks": [{"type": "prose", "markdown": "ok"}]}"#,
    ];
    let said: Vec<Value> = calls.iter().map(|c| show(&s, &id, c)).collect();
    let text = |n: usize| said[n]["content"][0]["text"].as_str().unwrap().to_owned();
    assert_eq!(said[0]["isError"], true);
    assert!(text(0).starts_with("Not drawn:"), "{}", text(0));
    assert_eq!(text(1), "Shown as plain text. Do not send it again.");
    assert!(text(2).starts_with("NO_RETRY"), "{}", text(2));
    let kept = blocks(&s, &id);
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!(kept[0]["blocks"][0]["type"], "prose");
    assert!(kept[0]["blocks"].to_string().contains("Which plan?"));
}

#[test]
fn a_persona_with_no_catalog_lists_no_show_tool_and_keeps_todays_brief() {
    let s = setup("personas-show-none");
    let (id, brief) = persona(&s, "Plain", None);
    assert!(!tools(&s, &id).contains(&"show".to_owned()));
    assert!(!brief.contains("`show` tool"), "{brief}");
    let said = show(&s, &id, CHOICE);
    let text = said["content"][0]["text"].as_str().unwrap();
    assert!(text.starts_with("NO_CATALOG"), "{said}");
    let bad = json!({ "name": "Odd", "agent": "fake-acp", "blocks": "everything" });
    assert_eq!(s.call("POST", "/personas", Some(&bad)).0, 400);
}
