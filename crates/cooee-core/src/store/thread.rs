use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Member, RoomTurn, Workflow};

/// A conversation with one agent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Thread {
    /// The thread's id: 1 to 64 of `A-Z a-z 0-9 - _` (it names the file).
    pub id: String,
    /// The title shown in the thread list.
    pub title: String,
    /// The messages, oldest first (0.1.0; a chat's rows are its items, in `cooee.db`).
    pub messages: Vec<Message>,
    /// The ACP agent (`shed agents` id).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub agent: String,
    /// The working folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<PathBuf>,
    /// The provider account its agent runs with, by name (empty: the system default, or no provider's agent; before
    /// #48 its label, `System default` for that one).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub account: String,
    /// Its agent profile's id (empty: Plain), fixed for its life.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub profile: String,
    /// The shed terminal's name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub terminal: String,
    /// The model its agent runs on (#553 row 34; empty: the agent's default).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
    /// The effort, as `model`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub effort: String,
    /// The machine it runs on (#148): its persona's, or the one picked at its start; empty: this host.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub machine: String,
    /// The newest item id the owner has seen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read: Option<u64>,
    /// The thread was closed: its terminal is killed and removed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub closed: bool,
    /// The page's id for the start that made it, so a retried start adopts this thread instead of starting another.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub random_id: String,
    /// Archived: listed apart, under the thread list's archived threads, until restored.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub archived: bool,
    /// The owner's messages queued during a turn, sent in order as each turn ends.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub queue: Vec<Queued>,
    /// Where a session a tool started came from (a Workflow item, #33); none: the owner's own chat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<Workflow>,
    /// Sent ahead of the next message after a folder was attached (#86): the agent started again there.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub handover: String,
    /// A chat, or a room (#154).
    #[serde(default, skip_serializing_if = "Kind::is_chat")]
    pub kind: Kind,
    /// A room's members, in room order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<Member>,
    /// A room's turns still to take on the owner's last message, in order: the first is the one taking it (#155).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub round: Vec<RoomTurn>,
    /// How many member turns the owner's last message has started (at most 6, G2).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub taken: u8,
    /// When the owner's last message in a room was sent (ms): its round stops 10 minutes on (G2).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub asked_at: u64,
    /// A member's session: the room it is a member of (hidden from the thread list).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub room: String,
    /// A linked exchange (#156): the thread whose persona asked this one's (`ask_persona`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub asked_by: String,
    /// A handed chat (#802): Chief's chat that handed the owner to this one's agent.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub handed_by: String,
    /// The agent's open handoff (#164): its card's item id, 0 for none; until the owner hands back, nothing they
    /// send reaches the agent.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub handoff: u64,
    /// A setup chat (#270): its persona's brief asks it to interview the owner and write its Character and
    /// Instructions, and its `update_persona` saves at once.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub setup: bool,
    /// The routine whose run started it (#75).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routine: Option<crate::routines::Mark>,
    /// The agent's last reported session id, which a restart resumes (#613).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub session: String,
    /// The agents that hit a usage limit in it (#76), first capped first, each until it next replies.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capped: Vec<crate::agents::Capped>,
    /// A plugin agent's chat attached to its plugin (#823).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attach: Option<crate::plugins::rows::Attach>,
}

impl Thread {
    /// The id its plugin knows it by: the adopted one, else its own.
    #[must_use]
    pub fn plugin_thread(&self) -> &str {
        match &self.attach {
            Some(a) if !a.adopt.is_empty() => &a.adopt,
            _ => &self.id,
        }
    }
}

/// What a [`Thread`] is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A conversation with one agent.
    #[default]
    Chat,
    /// The owner with 2 to 5 member personas, each in its own session.
    Room,
}

impl Kind {
    fn is_chat(&self) -> bool {
        *self == Self::Chat
    }
}

fn is_zero<T: Default + PartialEq>(n: &T) -> bool {
    *n == T::default()
}

/// A message queued during a turn.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Queued {
    /// The text.
    pub text: String,
    /// The page's id for the send: the item it becomes keeps it.
    pub random_id: String,
    /// Its files' stored names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    /// The routine that sent it (#75).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routine: Option<crate::routines::Mark>,
    /// How it was sent (#663).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<crate::Via>,
    /// The approval it denies (#351): once sent it shows under that card, not as a message of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denied: Option<u64>,
    /// The blocks reply it answers (#776): once sent it shows under that reply, not as a message of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acted: Option<u64>,
}

/// One message in a thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    /// Who wrote it.
    pub role: Role,
    /// The message text.
    pub text: String,
}

/// The author of a [`Message`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// The owner.
    User,
    /// The coding agent.
    Agent,
}
