use cooee_core::{Body, Change, Chat, Thread};
use serde::Serialize;

/// One notification: a line about a thread; tapping it opens `url`. `tag` (the thread) makes a newer one replace
/// the older, and the page closes it once read past `item`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Note {
    /// The thread's title.
    pub title: String,
    /// What happened, one line.
    pub body: String,
    /// The thread's id.
    pub tag: String,
    /// The thread's page.
    pub url: String,
    /// The item it is about.
    pub item: u64,
    /// An approval the owner may deny with their text from the notification (#351).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub say: bool,
}

impl Note {
    /// `thread`'s note of `key` (`needs`, `done`, `fail`): `theme`'s words, then `who: line`, about item `item`.
    #[must_use]
    pub fn new(
        thread: &Thread,
        (key, who, line): (&str, &str, &str),
        item: u64,
        theme: &crate::look::Theme,
    ) -> Self {
        Self {
            title: thread.title.clone(),
            body: theme
                .push_body(key, &format!("{who}: {line}"))
                .chars()
                .take(120)
                .collect(),
            tag: thread.id.clone(),
            url: format!("/t/{}", thread.id),
            item,
            say: false,
        }
    }

    /// This member session's note as room `room` says it (#158): titled `Room · name`, opening the room at `item`.
    #[must_use]
    pub fn in_room(self, room: &Thread, name: &str, item: u64) -> Self {
        Self {
            title: format!("{} · {name}", room.title),
            tag: room.id.clone(),
            url: format!("/t/{}", room.id),
            item,
            ..self
        }
    }
}

/// The notification for `change` in `thread`, if it is one: an approval waiting, a turn done while no page
/// watches the thread (`watched`), or an error. `was` is the state before the change. The body is `theme`'s word,
/// the plain word and the agent's line: `at the gate · needs you · acme: Edit src/pair.rs`.
#[must_use]
pub fn note(
    thread: &Thread,
    chat: &Chat,
    change: &Change,
    was: &str,
    watched: bool,
    theme: &crate::look::Theme,
) -> Option<Note> {
    let last = chat.items.last().map_or(0, |i| i.id);
    let say = matches!(change, Change::Item(i) if i.sayable().is_some());
    let (key, line, item) = match change {
        Change::Item(i) => match &i.body {
            Body::Approval {
                tool,
                tapped: None,
                chosen: None,
                ..
            } => ("needs", tool.clone(), i.id),
            _ => return None,
        },
        Change::State(t) if t.state == was => return None,
        Change::State(t)
            if t.state == "done"
                && !watched
                && thread.workflow.is_none()
                && thread.routine.is_none()
                && thread.room.is_empty() =>
        {
            let reply = chat.items.iter().rev().find_map(|i| match &i.body {
                Body::Agent { text, .. } => text.lines().find(|l| !l.trim().is_empty()),
                _ => None,
            });
            ("done", reply.unwrap_or("the turn ended").to_owned(), last)
        }
        Change::State(t) if t.state == "error" => (
            "fail",
            t.note.as_deref().unwrap_or("the agent failed").to_owned(),
            last,
        ),
        Change::State(_) | Change::Text(_) => return None,
    };
    let note = Note::new(thread, (key, &thread.agent, &line), item, theme);
    Some(Note { say, ..note })
}

#[cfg(test)]
pub(super) mod tests {
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

    #[test]
    fn a_push_for_each_trigger_and_none_otherwise() {
        let mut chat = Chat::default();
        chat.push(
            1,
            0,
            Body::You {
                text: "hi".to_owned(),
                random_id: "r".to_owned(),
                files: Vec::new(),
                routine: None,
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
        let approval = |tapped: Option<&str>| {
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
        };
        let n = |body: &str, item| Note {
            title: "Fix the build".to_owned(),
            body: body.to_owned(),
            tag: "t1".to_owned(),
            url: "/t/t1".to_owned(),
            item,
            say: false,
        };
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
}
