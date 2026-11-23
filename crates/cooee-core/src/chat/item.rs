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

/// How the owner's message was sent, when not typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Via {
    /// Spoken in a call (#663): its reply is read aloud.
    Voice,
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
        /// How it was sent: `voice` for a call's utterance (#663), none when typed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        via: Option<Via>,
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
        /// The rule that answered its request, in plain words.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ruled: Option<String>,
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
        /// The agent took this screenshot itself, so it saw it (D3): the chat shows it as a `screen` block (#166).
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        asked: bool,
    },
    /// A persona's reply as blocks (#776), read under its catalog before it was stored.
    Blocks(crate::show::Reply),
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

impl Item {
    /// The answer that denies an approval with the owner's text as its value (#351).
    pub const SAY: &str = "say";

    /// An approval's deny: the agent's reject-once option, else its reject-always, else `decline`.
    #[must_use]
    pub fn deny(&self) -> Option<&ApprovalOption> {
        let Body::Approval { options, .. } = &self.body else {
            return None;
        };
        let reject = |kind: &str| options.iter().find(|o| o.kind == kind);
        reject("reject_once")
            .or_else(|| reject("reject_always"))
            .or_else(|| options.iter().find(|o| o.id == "decline"))
    }

    /// Shed's own approval (no plugin card) with a deny (#351): it may be denied with the owner's text.
    #[must_use]
    pub fn sayable(&self) -> Option<&ApprovalOption> {
        let shed =
            matches!(&self.body, Body::Approval { plugin: None, card, .. } if card.form.is_none());
        self.deny().filter(|_| shed)
    }
}
