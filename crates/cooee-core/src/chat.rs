//! A thread's chat: its committed items and the turn in progress, built from shed's ACP events.
//!
//! Every shed event maps through one exhaustive `match` (no wildcard arm): a new shed event kind fails the build
//! here instead of being guessed at. An event line this build can't read is dropped and logged by the client.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use shed_proto::{ApprovalOption, EventKind};

/// One row of a thread, as stored and as the page draws it. A later version of an item replaces the earlier one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    /// Counts up from 1 per thread.
    pub id: u64,
    /// When it was made, in Unix milliseconds.
    pub at: u64,
    /// The shed event that last changed it (0 for the owner's own messages).
    #[serde(default)]
    pub seq: u64,
    /// What it is.
    #[serde(flatten)]
    pub body: Body,
}

/// What an [`Item`] holds; `kind` names it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Body {
    /// The owner's message.
    You {
        /// The text.
        text: String,
        /// The page's id for the send, so a retried send is stored once.
        random_id: String,
    },
    /// The agent's whole reply to a turn.
    Agent {
        /// The text.
        text: String,
        /// Why the turn ended (`end_turn`, `cancelled`, …).
        stop_reason: String,
    },
    /// A tool call.
    Tool {
        /// The agent's tool call id.
        call: String,
        /// What it does, in the agent's words.
        title: String,
        /// `pending`, `in_progress`, `completed`, `failed` or `cancelled`.
        status: String,
        /// When it ended, in Unix milliseconds.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ended: Option<u64>,
    },
    /// The agent asks before running a tool.
    Approval {
        /// The tool call's title.
        tool: String,
        /// The agent's choices.
        options: Vec<ApprovalOption>,
        /// The option the owner tapped, once sent to shed (so a second tap sends nothing).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tapped: Option<String>,
        /// The option shed reports chosen (the tap, or the bridge's policy).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        chosen: Option<String>,
        /// A plugin's choice card: the plugin that asked, which gets the answer instead of shed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        plugin: Option<String>,
        /// Who answered and when, and a plugin card's form.
        #[serde(flatten, default)]
        card: Card,
    },
    /// The agent's plan.
    Plan {
        /// The entries (`content`, `priority`, `status`).
        entries: Vec<Value>,
    },
    /// A line linked to another thread (#553 row 35): a handoff's `continued in`, or a line `from` it.
    Link {
        /// The line.
        text: String,
        /// The other thread's id.
        thread: String,
        /// Its title.
        title: String,
        /// Whether it points onward (`continued in`) rather than back (`from`).
        #[serde(default)]
        out: bool,
    },
    /// A line from cooee itself (`model sonnet · effort low from the next start`).
    Note {
        /// The line.
        text: String,
    },
    /// The session ended (the agent exited, or the thread was closed).
    Ended {
        /// Why.
        text: String,
    },
}

/// An approval's answer and a plugin card's form (#553 rows 30–33); every field is optional.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    /// Who answered: the device's label, or the plugin's own (a conductor's answer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// When it was answered, in Unix milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answered: Option<u64>,
    /// `config` (a diff to apply or discard), `secret` (a masked value) or `thread` (pick a thread); none: choices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    /// A config card's diff, or a secret card's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The plugin's line once settled (`applied, doctor ✓`), or why it can't be answered (`stale: …`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    /// Until when a conductor's answer may be changed, in Unix milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<u64>,
}

/// The turn in progress: what the page shows beyond the committed items.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Turn {
    /// The reply so far.
    pub text: String,
    /// shed's reported state (`working`, `blocked`, `done`, `error`), or `ended`.
    pub state: String,
    /// The state's note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// One row of the thread list.
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    /// The thread's id.
    pub id: String,
    /// Its title.
    pub title: String,
    /// The agent.
    pub agent: String,
    /// The working folder.
    pub folder: PathBuf,
    /// The state: `working`, `blocked`, `done`, `error`, `ended` or empty.
    pub state: String,
    /// The last item's text, short.
    pub last: String,
    /// When the last item was made, in Unix milliseconds.
    pub at: u64,
    /// Whether it has items the owner hasn't seen.
    pub unread: bool,
}

/// What applying an event changed, for the page.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// An item was added or replaced; it is stored before the page hears of it.
    Item(Item),
    /// A chunk of the reply in progress.
    Text(String),
    /// The state changed.
    State(Turn),
}

/// A thread's items and its turn in progress.
#[derive(Debug, Clone, Default)]
pub struct Chat {
    /// Every committed item, by id.
    pub items: Vec<Item>,
    /// The turn in progress.
    pub turn: Turn,
    /// Where the current turn's items start in `items`.
    start: usize,
}

impl Chat {
    /// A chat from its stored item versions (oldest first; a later version of an id replaces the earlier).
    #[must_use]
    pub fn from_versions(versions: Vec<Item>) -> Self {
        let mut chat = Self::default();
        for item in versions {
            chat.put(item);
        }
        chat.start = chat.items.len();
        chat
    }

    /// The newest shed event seq the items reflect.
    #[must_use]
    pub fn seq(&self) -> u64 {
        self.items.iter().map(|i| i.seq).max().unwrap_or(0)
    }

    /// Adds or replaces `item`. `items` stay sorted by id, so each lookup is a binary search: folding a long
    /// history is never quadratic (herdsman#447), and a repeated id is one row.
    fn put(&mut self, item: Item) -> Item {
        match self.items.binary_search_by_key(&item.id, |i| i.id) {
            Ok(at) => self.items[at].clone_from(&item),
            Err(at) => self.items.insert(at, item.clone()),
        }
        item
    }

    /// Adds a new item with the next id.
    pub fn push(&mut self, at: u64, seq: u64, body: Body) -> Item {
        let id = self.items.last().map_or(1, |i| i.id + 1);
        self.put(Item { id, at, seq, body })
    }

    /// Replaces item `id`'s body; `None` when there is no such item.
    pub fn update(&mut self, id: u64, seq: u64, f: impl FnOnce(&mut Body)) -> Option<Item> {
        let at = self.items.binary_search_by_key(&id, |i| i.id).ok()?;
        let item = &mut self.items[at];
        f(&mut item.body);
        item.seq = item.seq.max(seq);
        Some(item.clone())
    }

    /// The id of the current turn's last item matching `f`.
    fn find(&self, f: impl Fn(&Body) -> bool) -> Option<u64> {
        self.items[self.start.min(self.items.len())..]
            .iter()
            .rev()
            .find(|i| f(&i.body))
            .map(|i| i.id)
    }

    fn state(&mut self, state: &str, note: Option<String>) -> Change {
        state.clone_into(&mut self.turn.state);
        self.turn.note = note;
        Change::State(self.turn.clone())
    }

    /// Ends the turn: the reply so far is dropped and tool calls still going are written back as ended (and as
    /// `status`, if given).
    fn end(&mut self, at: u64, seq: u64, status: Option<&str>) -> Vec<Change> {
        self.turn.text.clear();
        let open: Vec<u64> = self.items[self.start.min(self.items.len())..]
            .iter()
            .filter(|i| matches!(&i.body, Body::Tool { ended: None, .. }))
            .map(|i| i.id)
            .collect();
        let changes = open
            .into_iter()
            .filter_map(|id| {
                self.update(id, seq, |b| {
                    if let Body::Tool {
                        status: s, ended, ..
                    } = b
                    {
                        if let Some(status) = status {
                            status.clone_into(s);
                        }
                        *ended = Some(at);
                    }
                })
            })
            .map(Change::Item)
            .collect();
        self.start = self.items.len();
        changes
    }

    /// Applies shed event `seq` of this thread's terminal at time `at`.
    pub fn apply(&mut self, at: u64, seq: u64, event: EventKind) -> Vec<Change> {
        match event {
            // Not shown: the start, resources and the agent's mode; the prompt is stored when it is sent.
            EventKind::Started
            | EventKind::Resources(_)
            | EventKind::Mode { .. }
            | EventKind::Turn { .. } => Vec::new(),
            EventKind::State(r) => vec![self.state(&r.state, r.note)],
            EventKind::Text { text } => {
                self.turn.text.push_str(&text);
                vec![Change::Text(text)]
            }
            EventKind::ToolCall { id, title, status } => {
                let done = matches!(status.as_str(), "completed" | "failed");
                let ended = done.then_some(at);
                let call = |b: &Body| matches!(b, Body::Tool { call, .. } if *call == id);
                let item = match self.find(call) {
                    Some(item) => self.update(item, seq, |b| {
                        *b = Body::Tool {
                            call: id,
                            title,
                            status,
                            ended,
                        };
                    }),
                    None => Some(self.push(
                        at,
                        seq,
                        Body::Tool {
                            call: id,
                            title,
                            status,
                            ended,
                        },
                    )),
                };
                item.map(Change::Item).into_iter().collect()
            }
            EventKind::Plan { entries } => {
                let plan = |b: &Body| matches!(b, Body::Plan { .. });
                let item = match self.find(plan) {
                    Some(item) => self.update(item, seq, |b| *b = Body::Plan { entries }),
                    None => Some(self.push(at, seq, Body::Plan { entries })),
                };
                item.map(Change::Item).into_iter().collect()
            }
            EventKind::ApprovalRequest { tool, options } => {
                let body = Body::Approval {
                    tool,
                    options,
                    tapped: None,
                    chosen: None,
                    plugin: None,
                    card: Card::default(),
                };
                vec![Change::Item(self.push(at, seq, body))]
            }
            EventKind::Approved { option } => {
                let open = self.items.iter().find(|i| {
                    matches!(
                        i.body,
                        Body::Approval {
                            chosen: None,
                            plugin: None,
                            ..
                        }
                    )
                });
                let item = open.map(|i| i.id).and_then(|id| {
                    self.update(id, seq, |b| {
                        if let Body::Approval { chosen, .. } = b {
                            *chosen = Some(option);
                        }
                    })
                });
                item.map(Change::Item).into_iter().collect()
            }
            EventKind::Reply { text, stop_reason } => {
                let status = (stop_reason == "cancelled").then_some("cancelled");
                let mut changes = self.end(at, seq, status);
                changes.push(Change::Item(self.push(
                    at,
                    seq,
                    Body::Agent { text, stop_reason },
                )));
                changes
            }
            EventKind::Exited { code } => {
                let text = code.map_or("The agent stopped.".to_owned(), |c| {
                    format!("The agent exited ({c}).")
                });
                self.ended(at, seq, text)
            }
            EventKind::Killed => self.ended(at, seq, "The thread was closed.".to_owned()),
        }
    }

    /// The session is over: the turn ends and an `ended` row says why.
    pub fn ended(&mut self, at: u64, seq: u64, text: String) -> Vec<Change> {
        let mut changes = self.end(at, seq, Some("cancelled"));
        changes.push(Change::Item(self.push(at, seq, Body::Ended { text })));
        changes.push(self.state("ended", None));
        changes
    }

    /// Records `by`'s tap of `option` on approval `id` at `now`. `None` when it isn't an approval or was already
    /// tapped or answered, so the tap must not be sent again; a conductor's answer is open to a change until its
    /// `until`.
    pub fn tap(&mut self, id: u64, option: &str, by: &str, now: u64) -> Option<Item> {
        let open = |b: &Body| match b {
            Body::Approval {
                tapped: None,
                chosen,
                card,
                ..
            } => chosen.is_none() || card.until.is_some_and(|u| now < u),
            _ => false,
        };
        if !self.items.iter().any(|i| i.id == id && open(&i.body)) {
            return None;
        }
        self.update(id, 0, |b| {
            if let Body::Approval {
                tapped,
                chosen,
                card,
                ..
            } = b
            {
                *tapped = Some(option.to_owned());
                *chosen = None;
                card.by = Some(by.to_owned());
                card.answered = Some(now);
                card.until = None;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approval(plugin: Option<&str>) -> Body {
        Body::Approval {
            tool: "Ship it?".to_owned(),
            options: Vec::new(),
            tapped: None,
            chosen: None,
            plugin: plugin.map(str::to_owned),
            card: Card::default(),
        }
    }

    #[test]
    fn shed_answers_its_own_approval_never_a_plugins_card() {
        let mut chat = Chat::default();
        let card = chat.push(1, 0, approval(Some("drover"))).id;
        let ask = chat.push(2, 1, approval(None)).id;
        let option = "allow".to_owned();
        chat.apply(3, 2, EventKind::Approved { option });
        let chosen = |id| match &chat.items.iter().find(|i| i.id == id).unwrap().body {
            Body::Approval { chosen, .. } => chosen.clone(),
            _ => None,
        };
        assert_eq!(
            (chosen(card), chosen(ask)),
            (None, Some("allow".to_owned()))
        );
    }

    /// #553 row 31: a conductor's answer takes the owner's change until its `until`, then none; an owner's own
    /// answer takes no second tap.
    #[test]
    fn a_conductors_answer_may_be_changed_for_its_window_only() {
        let mut chat = Chat::default();
        let mut body = approval(Some("drover"));
        if let Body::Approval { chosen, card, .. } = &mut body {
            *chosen = Some("yes".to_owned());
            card.until = Some(100);
        }
        let late = chat.push(1, 0, body.clone()).id;
        let id = chat.push(1, 0, body).id;
        assert!(chat.tap(late, "no", "iPhone", 100).is_none(), "too late");
        let changed = chat.tap(id, "no", "iPhone", 99).unwrap();
        let Body::Approval {
            tapped,
            chosen,
            card,
            ..
        } = changed.body
        else {
            unreachable!()
        };
        assert_eq!(
            (
                tapped.as_deref(),
                chosen,
                card.by.as_deref(),
                card.answered,
                card.until
            ),
            (Some("no"), None, Some("iPhone"), Some(99), None)
        );
        assert!(chat.tap(id, "yes", "iPhone", 99).is_none(), "once");
    }

    /// R6 (herdsman#447): folding 30,000 stored versions of 1,000 rows (a thread's load) takes under 300 ms: each
    /// version finds its row by binary search, never by a scan.
    #[test]
    fn r6_folding_a_long_log_is_within_budget() {
        let versions: Vec<Item> = (0..30)
            .flat_map(|v| {
                (1..=1000).map(move |id| Item {
                    id,
                    at: id,
                    seq: v * 1000 + id,
                    body: Body::Agent {
                        text: format!("reply {id} v{v}"),
                        stop_reason: "end_turn".to_owned(),
                    },
                })
            })
            .collect();
        let at = std::time::Instant::now();
        let chat = Chat::from_versions(versions);
        let took = at.elapsed();
        assert_eq!((chat.items.len(), chat.seq()), (1000, 30_000));
        assert!(took < std::time::Duration::from_millis(300), "{took:?}");
    }

    /// R16 (herdsman#516): versions repeating one id fold into one row, however often they repeat.
    #[test]
    fn a_repeated_id_is_one_row() {
        let you = |text: &str| Item {
            id: 7,
            at: 1,
            seq: 0,
            body: Body::You {
                text: text.to_owned(),
                random_id: "r".to_owned(),
            },
        };
        let chat = Chat::from_versions((0..5).map(|i| you(&format!("v{i}"))).collect());
        assert_eq!(chat.items, [you("v4")]);
    }
}
