use std::fs;

use cooee_core::agents::SPOKEN;
use serde_json::json;

use crate::setup::{cook, items, say, setup};

#[test]
fn a_voice_message_is_marked_voice_and_prompted_with_the_personas_spoken_hint() {
    let s = setup("personas-voice");
    let id = cook(&s);
    let (item, prompt) = say(&s, &id, "what's for dinner", Some("voice"));
    assert_eq!(
        (&item["kind"], &item["via"]),
        (&json!("you"), &json!("voice"))
    );
    assert_eq!(prompt, format!("{SPOKEN}\n\nwhat's for dinner"));
    let stored = items(&s, &id);
    assert!(stored.contains(&item), "{stored:?}");
    let (item, prompt) = say(&s, &id, "typed", None);
    assert!(item.get("via").is_none(), "{item}");
    assert_eq!(prompt, "typed");

    let new = json!({ "name": "Caller", "agent": "fake-acp", "folder": "scratch", "spoken": "Answer in one word." });
    assert_eq!(s.call("POST", "/personas", Some(&new)).0, 200);
    let file = fs::read_to_string(s.dir.join("agents/caller/agent.toml")).unwrap();
    assert!(file.contains("spoken = \"Answer in one word.\""), "{file}");
    let id = s.start("caller").0;
    let (_, prompt) = say(&s, &id, "hello", Some("voice"));
    assert_eq!(prompt, "Answer in one word.\n\nhello");
}
