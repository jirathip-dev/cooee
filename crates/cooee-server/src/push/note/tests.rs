use cooee_core::{Item, Turn};

use super::*;

pub(in crate::push) fn test_note() -> Note {
    Note {
        title: "Fix the build".to_owned(),
        body: "needs you".to_owned(),
        tag: "t1".to_owned(),
        url: "/t/t1".to_owned(),
        item: 7,
        say: false,
    }
}

fn thread() -> Thread {
    Thread {
        id: "t1".to_owned(),
        title: "Fix the build".to_owned(),
        agent: "codex".to_owned(),
        ..Thread::default()
    }
}

fn turn(state: &str, note: Option<&str>) -> Change {
    Change::State(Turn {
        state: state.to_owned(),
        note: note.map(str::to_owned),
        ..Turn::default()
    })
}

#[test]
fn only_an_approval_the_agent_can_deny_asks_for_text_from_its_push() {
    let (looks, _) = crate::look::Looks::load(None, None);
    let theme = &looks.themes[&looks.default];
    let reject = cooee_core::shed_proto::ApprovalOption {
        id: "reject".to_owned(),
        name: "Reject".to_owned(),
        kind: "reject_once".to_owned(),
    };
    let says = |options: Vec<_>, plugin: Option<&str>| {
        let ask = Change::Item(Item {
            id: 3,
            at: 3,
            seq: 2,
            speaker: String::new(),
            body: Body::Approval {
                tool: "Edit src/lib.rs".to_owned(),
                options,
                tapped: None,
                chosen: None,
                plugin: plugin.map(str::to_owned),
                card: cooee_core::Card::default(),
            },
        });
        let n = note(&thread(), &Chat::default(), &ask, "working", true, theme);
        n.unwrap().say
    };
    assert!(says(vec![reject.clone()], None));
    assert!(!says(Vec::new(), None));
    assert!(!says(vec![reject], Some("drover")));
}

fn finished_chat() -> Chat {
    let mut chat = Chat::default();
    chat.push(
        1,
        0,
        Body::You {
            text: "hi".to_owned(),
            random_id: "r".to_owned(),
            files: Vec::new(),
            routine: None,
            via: None,
        },
    );
    chat.push(
        2,
        1,
        Body::Agent {
            text: "\nAll tests pass.\nMore.".to_owned(),
            stop_reason: "end_turn".to_owned(),
        },
    );
    chat
}

fn approval(tapped: Option<&str>) -> Change {
    Change::Item(Item {
        id: 3,
        at: 3,
        seq: 2,
        speaker: String::new(),
        body: Body::Approval {
            tool: "Edit src/lib.rs".to_owned(),
            options: Vec::new(),
            tapped: tapped.map(str::to_owned),
            chosen: None,
            plugin: None,
            card: cooee_core::Card::default(),
        },
    })
}

fn n(body: &str, item: u64) -> Note {
    Note {
        title: "Fix the build".to_owned(),
        body: body.to_owned(),
        tag: "t1".to_owned(),
        url: "/t/t1".to_owned(),
        item,
        say: false,
    }
}

#[test]
fn a_push_for_each_trigger_and_none_otherwise() {
    let chat = finished_chat();
    let t = thread();
    let packs = crate::look::tests::first_party("note-words");
    let (looks, _) = crate::look::Looks::load(Some(&packs), None);
    let (farm, plain) = (&looks.themes["farm"], &looks.themes["plain"]);
    assert_eq!(
        note(&t, &chat, &approval(None), "working", true, farm),
        Some(n("at the gate · needs you · codex: Edit src/lib.rs", 3))
    );
    assert_eq!(
        note(&t, &chat, &approval(Some("allow")), "working", false, farm),
        None
    );
    assert_eq!(
        note(&t, &chat, &turn("done", None), "working", false, farm),
        Some(n("in the stable · done · codex: All tests pass.", 2))
    );
    assert_eq!(
        note(&t, &chat, &turn("done", None), "working", true, farm),
        None
    );
    assert_eq!(
        note(&t, &chat, &turn("done", None), "done", false, farm),
        None
    );
    assert_eq!(
        note(
            &t,
            &chat,
            &turn("error", Some("rate limited")),
            "working",
            true,
            plain
        ),
        Some(n("failed · codex: rate limited", 2))
    );
    assert_eq!(
        note(&t, &chat, &turn("working", None), "done", false, farm),
        None
    );
    assert_eq!(
        note(
            &t,
            &chat,
            &Change::Text("x".to_owned()),
            "working",
            false,
            farm
        ),
        None
    );
    assert_eq!(
        serde_json::to_value(n("x", 2)).unwrap(),
        serde_json::json!({"title": "Fix the build", "body": "x", "tag": "t1", "url": "/t/t1", "item": 2})
    );
}
