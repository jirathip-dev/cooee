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
    },
    /// The agent's plan.
    Plan {
        /// The entries (`content`, `priority`, `status`).
        entries: Vec<Value>,
    },
    /// The session ended (the agent exited, or the thread was closed).
    Ended {
        /// Why.
        text: String,
    },
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

    fn put(&mut self, item: Item) -> Item {
        match self.items.iter_mut().rev().find(|i| i.id == item.id) {
            Some(old) => old.clone_from(&item),
            None => self.items.push(item.clone()),
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
        let item = self.items.iter_mut().rev().find(|i| i.id == id)?;
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
                };
                vec![Change::Item(self.push(at, seq, body))]
            }
            EventKind::Approved { option } => {
                let open = self
                    .items
                    .iter()
                    .find(|i| matches!(i.body, Body::Approval { chosen: None, .. }));
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

    /// Records the owner's tap of `option` on approval `id`. `None` when it isn't an approval or was already
    /// tapped or answered, so the tap must not be sent again.
    pub fn tap(&mut self, id: u64, option: &str) -> Option<Item> {
        let open = |b: &Body| {
            matches!(
                b,
                Body::Approval {
                    tapped: None,
                    chosen: None,
                    ..
                }
            )
        };
        if !self.items.iter().any(|i| i.id == id && open(&i.body)) {
            return None;
        }
        self.update(id, 0, |b| {
            if let Body::Approval { tapped, .. } = b {
                *tapped = Some(option.to_owned());
            }
        })
    }
}
