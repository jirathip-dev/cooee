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
pub(super) mod tests;
