use serde_json::{Value, json};

use crate::setup::{Backend, TOKEN_A, TOKEN_B, checked, me, new_thread, setup, thread_of};
use crate::support::snapshot;

#[test]
fn a_choice_card_goes_into_the_thread_and_its_answer_to_its_own_plugin() {
    let s = setup("plugin-card");
    let id = new_thread(&s, "Say hello");

    let card = |token: &str, text: &str| {
        let card = json!({ "thread": id, "text": text, "options": ["yes", "no"] });
        let (code, body) = s.as_backend("/plugin/card", &format!("Bearer {token}"), &card);
        assert_eq!(code, 200, "{body}");
        serde_json::from_str::<Value>(&body).unwrap()
    };
    let a = card(TOKEN_A, "Ship it?");
    let b = card(TOKEN_B, "Merge it?");
    assert_eq!(
        (&a["kind"], &a["plugin"]),
        (&json!("approval"), &json!("a"))
    );
    let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    assert!(
        items
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["tool"] == "Merge it?" && i["plugin"] == "b"),
        "{items}"
    );

    let tap = json!({ "item": a["id"], "option": "yes" });
    let path = format!("/threads/{id}/approve");
    let (code, body) = s.cooee.post(&path, &tap);
    assert_eq!(code, 200, "{body}");
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["chosen"],
        "yes"
    );
    assert_eq!(s.cooee.post(&path, &tap).0, 200);
    let choices = |b: &Backend| -> Vec<(String, String)> {
        b.seen()
            .into_iter()
            .filter(|(h, _)| h.starts_with("post /cooee/choice "))
            .collect()
    };
    let sent = choices(&s.a);
    assert_eq!(sent.len(), 1, "a second tap sends nothing");
    assert!(sent[0].0.contains(TOKEN_A), "{}", sent[0].0);
    assert_eq!(
        serde_json::from_str::<Value>(&sent[0].1).unwrap(),
        json!({ "thread": id, "item": a["id"], "option": "yes", "by": me(&s) })
    );
    assert!(
        choices(&s.b).is_empty(),
        "plugin b's backend hears nothing of a's card"
    );
    assert!(b["chosen"].is_null());
    let to_shed = s.shed.sent(&format!("cooee-{id}"));
    assert!(
        to_shed.iter().all(|t| !t.contains("approve")),
        "{to_shed:?}"
    );
}

#[test]
fn row_30_choice_rows_recommended_first_other_text_and_who_and_when() {
    let s = setup("card-30");
    let id = new_thread(&s, "Say hello");
    let (code, card) = s.card(
        &json!({ "thread": id, "text": "Which first?", "options": ["#43", "#44"], "recommend": "#44" }),
    );
    assert_eq!(code, 200, "{card}");
    assert_eq!(
        card["options"],
        json!([{ "id": "#44", "name": "#44", "kind": "recommended" }, { "id": "#43", "name": "#43", "kind": "" }])
    );
    let answer = s.tap(
        &id,
        &json!({ "item": card["id"], "option": "neither: #48 first" }),
    );
    assert_eq!(
        s.a.choices(),
        [json!({ "thread": id, "item": card["id"], "option": "neither: #48 first", "by": me(&s) })]
    );
    assert_eq!(
        (&answer["chosen"], &answer["by"]),
        (&json!("neither: #48 first"), &json!(me(&s)))
    );
    assert!(answer["answered"].as_u64().unwrap() >= card["at"].as_u64().unwrap());
}

#[test]
fn row_31_a_conductors_answer_takes_one_change_from_the_owner() {
    let s = setup("card-31");
    let id = new_thread(&s, "Say hello");
    let (_, card) = s.card(&json!({
        "thread": id, "text": "answered q12 for #44: split", "options": ["split", "keep"],
        "answered": "split", "by": "conductor-t6"
    }));
    assert_eq!(
        (&card["chosen"], &card["by"]),
        (&json!("split"), &json!("conductor-t6"))
    );
    let at = card["answered"].as_u64().unwrap();
    assert_eq!(card["until"].as_u64(), Some(at + 30 * 60 * 1000));
    assert!(
        s.a.choices().is_empty(),
        "the conductor's own answer isn't sent back"
    );

    let changed = s.tap(&id, &json!({ "item": card["id"], "option": "keep" }));
    assert_eq!(
        (&changed["chosen"], &changed["by"], &changed["until"]),
        (&json!("keep"), &json!(me(&s)), &Value::Null)
    );
    let again = s.tap(&id, &json!({ "item": card["id"], "option": "split" }));
    assert_eq!(again["chosen"], "keep");
    assert_eq!(
        s.a.choices(),
        [json!({ "thread": id, "item": card["id"], "option": "keep", "by": me(&s) })]
    );
}

#[test]
fn a_card_to_a_thread_of_a_persona_its_plugin_isnt_attached_to_is_refused() {
    let s = setup("card-unattached");
    let only_b = json!({ "name": "Bee", "agent": "fake-acp", "plugins": ["b"] });
    assert_eq!(s.cooee.post("/personas", &only_b).0, 200);
    for id in [
        thread_of(&s, "bee", "Say hello"),
        thread_of(&s, "", "Say hi"),
    ] {
        let card = json!({ "thread": id, "text": "Ship it?", "options": ["yes"] });
        let (code, why) = s.as_backend("/plugin/card", &format!("Bearer {TOKEN_A}"), &card);
        assert_eq!(code, 400, "{why}");
        assert!(why.contains("plugin a is not attached"), "{why}");
        let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
        assert!(!items.to_string().contains("Ship it?"), "{items}");
    }
    let card =
        json!({ "thread": thread_of(&s, "bee", "Again"), "text": "Merge?", "options": ["yes"] });
    let (code, body) = s.as_backend("/plugin/card", &format!("Bearer {TOKEN_B}"), &card);
    assert_eq!(code, 200, "{body}");
}

#[test]
fn cooees_own_cards_go_out_as_blocks_and_settle_to_their_line() {
    let s = setup("card-blocks");
    let id = new_thread(&s, "Say hello");
    let cards = [
        json!({ "thread": id, "text": "Which first?", "options": ["#43", "#44"], "recommend": "#44" }),
        json!({ "thread": id, "text": "config c1", "form": "config", "detail": "+max_impl = 3" }),
        json!({ "thread": id, "text": "for the bot", "form": "secret", "detail": "BOT_TOKEN" }),
        json!({ "thread": id, "text": "re-tag", "form": "run-card", "detail": "git tag -f v1", "sha": "9f86d081" }),
    ];
    let items: Vec<Value> = cards.iter().map(|c| s.card(c).1).collect();
    let shape = |b: &Value| {
        let inner: Vec<&Value> = b["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| &x["type"])
            .collect();
        (b["kind"].clone(), json!(inner))
    };
    let blocks: Vec<&Value> = items.iter().map(|i| &i["block"]).collect();
    assert_eq!(
        blocks.iter().map(|b| shape(b)).collect::<Vec<_>>(),
        [
            (json!("ask"), json!(["choice"])),
            (json!("config"), json!(["text", "choice"])),
            (json!("secret"), json!(["secret"])),
            (json!("run"), json!(["run"])),
        ]
    );
    let choice = &blocks[0]["blocks"][0];
    assert_eq!(
        (
            &blocks[0]["id"],
            &blocks[0]["title"],
            &choice["recommended"],
            &choice["options"][0]
        ),
        (
            &json!(items[0]["id"].to_string()),
            &json!("Which first?"),
            &json!("#44"),
            &json!({ "value": "#44", "label": "#44" })
        )
    );
    assert_eq!(blocks[2]["blocks"][0]["name"], "BOT_TOKEN");
    assert_eq!(
        (
            &blocks[3]["blocks"][0]["command"],
            &blocks[3]["blocks"][0]["sha"]
        ),
        (&json!("git tag -f v1"), &json!("9f86d081"))
    );
    let (ok, said) = checked(&s, &json!(blocks));
    assert!(ok, "{said}");
    let slice = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let sent: Vec<&Value> = slice
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|i| i.get("block"))
        .collect();
    assert_eq!(sent, blocks);

    *s.a.choice.lock().unwrap() = Some((200, json!({ "line": "ran: exit 0" })));
    let ran = s.tap(&id, &json!({ "item": items[3]["id"], "option": "approve" }));
    assert_eq!(
        ran["block"]["blocks"],
        json!([{ "type": "text", "style": "footnote", "markdown": "ran: exit 0" }])
    );
    snapshot("cards", &json!([blocks, ran["block"]]), &[]);
}
