use serde_json::{Value, json};

use crate::setup::{TOKEN_A, me, new_thread, setup};

#[test]
fn row_32_config_card_waits_applies_with_the_plugins_line_or_goes_stale() {
    let s = setup("card-32");
    let id = new_thread(&s, "Say hello");
    let no_diff = json!({ "thread": id, "text": "config c1", "form": "config" });
    assert_eq!(s.card(&no_diff).0, 400);
    let diff = "-max_impl = 2\n+max_impl = 3";
    let (_, c1) =
        s.card(&json!({ "thread": id, "text": "config c1", "form": "config", "detail": diff }));
    assert_eq!(
        c1["options"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| &o["id"])
            .collect::<Vec<_>>(),
        ["apply", "discard"]
    );
    assert_eq!(
        s.cooee.json(&format!("/threads/{id}/messages"))["waiting"]["id"],
        c1["id"]
    );
    let row = |list: &Value| -> Value {
        let rows = list.as_array().unwrap().iter();
        rows.into_iter().find(|r| r["id"] == id).unwrap().clone()
    };
    assert_eq!(row(&s.cooee.json("/threads"))["last"], "config c1 waiting");

    let line = "config c1 applied by owner, backup kept, doctor ✓";
    *s.a.choice.lock().unwrap() = Some((200, json!({ "line": line })));
    let applied = s.tap(&id, &json!({ "item": c1["id"], "option": "apply" }));
    assert_eq!(
        (&applied["chosen"], &applied["line"]),
        (&json!("apply"), &json!(line))
    );
    assert!(s.cooee.json(&format!("/threads/{id}/messages"))["waiting"].is_null());

    let (_, c2) =
        s.card(&json!({ "thread": id, "text": "config c2", "form": "config", "detail": diff }));
    *s.a.choice.lock().unwrap() = Some((409, json!({ "line": "stale: fleet.toml changed since" })));
    let stale = s.tap(&id, &json!({ "item": c2["id"], "option": "apply" }));
    assert_eq!(
        (&stale["chosen"], &stale["line"]),
        (&Value::Null, &json!("stale: fleet.toml changed since"))
    );
    assert!(s.cooee.json(&format!("/threads/{id}/messages"))["waiting"].is_null());
    assert_eq!(
        s.tap(&id, &json!({ "item": c2["id"], "option": "apply" }))["line"],
        stale["line"]
    );
    assert_eq!(s.a.choices().len(), 2, "a stale card takes no second tap");
}

#[test]
fn row_33_a_secrets_value_reaches_its_plugin_only() {
    let s = setup("card-33");
    let id = new_thread(&s, "Say hello");
    let no_name = json!({ "thread": id, "text": "for the bot", "form": "secret" });
    assert_eq!(s.card(&no_name).0, 400);
    let (_, card) = s.card(
        &json!({ "thread": id, "text": "for the bot", "form": "secret", "detail": "BOT_TOKEN" }),
    );
    let value = "v4lue-only-the-plugin-sees";
    let saved = s.tap(
        &id,
        &json!({ "item": card["id"], "option": "save", "value": value }),
    );
    assert_eq!(
        (&saved["chosen"], &saved["detail"]),
        (&json!("save"), &json!("BOT_TOKEN"))
    );
    assert_eq!(s.a.choices()[0]["value"], value);
    for f in std::fs::read_dir(s.dir.join("data/threads")).unwrap() {
        let text = std::fs::read_to_string(f.unwrap().path()).unwrap();
        assert!(!text.contains(value));
    }
    assert!(
        !s.cooee
            .get(&format!("/threads/{id}/messages"))
            .1
            .contains(value)
    );
}

#[test]
fn row_35_handoff_chips_flow_back_lines_and_pick_a_thread() {
    let s = setup("card-35");
    let (t5, t6) = (
        new_thread(&s, "Plan the week"),
        new_thread(&s, "Fix the build"),
    );
    let auth = format!("Bearer {TOKEN_A}");
    let handoff = json!({ "from": t5, "to": t6, "text": "take the build" });
    let (code, body) = s.as_backend("/plugin/handoff", &auth, &handoff);
    assert_eq!(code, 200, "{body}");
    let last = |id: &str| {
        s.cooee.json(&format!("/threads/{id}/messages"))["items"]
            .as_array()
            .unwrap()
            .last()
            .cloned()
            .unwrap()
    };
    let link = |v: Value| {
        (
            v["kind"].clone(),
            v["text"].clone(),
            v["thread"].clone(),
            v["title"].clone(),
            v["out"].clone(),
        )
    };
    assert_eq!(
        link(last(&t5)),
        link(
            json!({ "kind": "link", "text": "", "thread": t6, "title": "Fix the build", "out": true })
        )
    );
    assert_eq!(
        link(last(&t6)),
        link(
            json!({ "kind": "link", "text": "take the build", "thread": t5, "title": "Plan the week", "out": false })
        )
    );
    let flow = json!({ "thread": t5, "from": t6, "text": "filed #61: build cache" });
    assert_eq!(s.as_backend("/plugin/line", &auth, &flow).0, 200);
    assert_eq!(
        link(last(&t5)),
        link(
            json!({ "kind": "link", "text": "filed #61: build cache", "thread": t6, "title": "Fix the build", "out": false })
        )
    );

    let (_, pick) =
        s.card(&json!({ "thread": t5, "text": "Continue in which thread?", "form": "thread" }));
    assert_eq!(
        pick["options"],
        json!([{ "id": t6, "name": "Fix the build", "kind": "" }])
    );
}

#[test]
fn a_run_card_approves_once_with_its_sha_and_shows_the_tail() {
    let s = setup("card-83");
    let id = new_thread(&s, "Say hello");
    let sha = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
    let run = |sha: Option<&str>| {
        let mut c = json!({ "thread": id, "text": "run r1: re-tag the release", "form": "run-card", "detail": "git tag -f v1" });
        if let Some(sha) = sha {
            c["sha"] = sha.into();
        }
        c
    };
    assert_eq!(s.card(&run(None)).0, 400, "a run card needs its sha");
    let (_, r1) = s.card(&run(Some(sha)));
    assert_eq!(
        (&r1["form"], &r1["detail"], &r1["sha"]),
        (&json!("run-card"), &json!("git tag -f v1"), &json!(sha))
    );
    assert_eq!(
        r1["options"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| &o["id"])
            .collect::<Vec<_>>(),
        ["approve", "deny"]
    );

    let line = "run r1 approved by owner, ran in /repo: exit 0\n```\nUpdated tag 'v1'\n```";
    *s.a.choice.lock().unwrap() = Some((200, json!({ "line": line })));
    let ran = s.tap(
        &id,
        &json!({ "item": r1["id"], "option": "approve", "value": "0000" }),
    );
    assert_eq!(
        (&ran["chosen"], &ran["line"], &ran["by"]),
        (&json!("approve"), &json!(line), &json!(me(&s)))
    );
    let again = s.tap(&id, &json!({ "item": r1["id"], "option": "approve" }));
    assert_eq!(again["line"], line);
    assert_eq!(
        s.a.choices(),
        [
            json!({ "thread": id, "item": r1["id"], "option": "approve", "by": me(&s), "value": sha })
        ],
        "approved once, with the card's sha"
    );

    let (_, r2) = s.card(&run(Some(sha)));
    let denied = s.tap(&id, &json!({ "item": r2["id"], "option": "deny" }));
    assert_eq!(denied["chosen"], "deny");
    assert_eq!(
        s.a.choices()[1],
        json!({ "thread": id, "item": r2["id"], "option": "deny", "by": me(&s) })
    );

    let (_, r3) = s.card(&run(Some(sha)));
    let expired = "r3 expired 15 min after it was posted; post a new card";
    *s.a.choice.lock().unwrap() = Some((409, json!({ "line": expired })));
    let stale = s.tap(&id, &json!({ "item": r3["id"], "option": "approve" }));
    assert_eq!(
        (&stale["chosen"], &stale["line"]),
        (&Value::Null, &json!(expired))
    );
    s.tap(&id, &json!({ "item": r3["id"], "option": "approve" }));
    assert_eq!(
        s.a.choices().len(),
        3,
        "a stale card takes no second approve"
    );
}
