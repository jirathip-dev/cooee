use serde::{Deserialize, Serialize};
use serde_json::Value;
use shed_proto::ApprovalOption;

use super::Card;
use crate::action::Kind;

/// One row of a thread, as stored and as the page draws it. A later version of an item replaces the earlier one.
/// An approval is sent with its card block (`blocks.rs`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Item {
    /// Counts up from 1 per thread.
    pub id: u64,
    /// When it was made, in Unix milliseconds.
    pub at: u64,
    /// The shed event that last changed it (0 for the owner's own messages).
    #[serde(default)]
    pub seq: u64,
    /// Who said it in a room (#154): `owner` or a member's persona id; empty outside rooms.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub speaker: String,
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
        /// The files sent with it, by their stored names (`GET /threads/<id>/files/<name>`).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        files: Vec<String>,
        /// The routine that sent it (#75), not the owner.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        routine: Option<crate::routines::Mark>,
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
        /// What it acts on: a path, a command, a query, a URL, `server/tool`, or the agent's title.
        #[serde(rename = "title")]
        subject: String,
        /// Its kind of action; a row stored before kinds were read is `other`.
        #[serde(default)]
        action: Kind,
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
    /// The thread's persona asked another through `ask_persona` (#177): the card in the asker's chat.
    Ask {
        /// The asked persona's id.
        persona: String,
        /// Its name.
        name: String,
        /// The question.
        text: String,
        /// The linked exchange thread; empty when it was never sent.
        thread: String,
        /// `asking`, `replied`, or the refusal's code in lower case (`not_allowed`, `depth`, `timeout`, `offline`).
        outcome: String,
        /// How long the reply took, in seconds.
        #[serde(default)]
        secs: u64,
        /// The owner's note to the asked persona, which the asker got with the reply.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        note: String,
    },
    /// The agent handed a log-in or 2FA to the owner (#164, W5): its turn stopped until the owner hands back.
    Handoff {
        /// What the owner is to do (`Sign in to linkedin.com`).
        text: String,
        /// `waiting` or `handed_back`.
        outcome: String,
        /// The owner's note, which the agent resumed with.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        note: String,
    },
    /// The agent made a routine (#75): `⟳ Created routine: <name> · <when> · <place>`.
    Routine {
        /// Its id.
        routine: String,
        /// Its name.
        name: String,
        /// Its schedule in words.
        when: String,
        /// Where it posts, in words.
        place: String,
    },
    /// A browser action's replay frame (#161): its caption and the frame (`GET /threads/<id>/frames/<name>`).
    Frame {
        /// The caption, like `click "Sign in"`.
        text: String,
        /// The frame's name.
        name: String,
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
