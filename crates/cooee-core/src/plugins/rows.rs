use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `[threads]`: where a plugin serves its rows, on its backend; `{id}` in `said` is a chat's id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct Threads {
    /// The rows after a seq (`?since=`), or every open one (`?open=1`).
    pub rows: String,
    /// The rows as server-sent events, resumed from `Last-Event-ID`.
    pub stream: String,
    /// Where an attached chat's owner lines are copied.
    pub said: String,
    /// Where a new chat attaches (`POST {fleet, thread}`); empty: its chats don't.
    pub attach: String,
}

/// A chat's attachment to its plugin (#823): a repo, `all` or `none`, and the old thread id it adopted.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Attach {
    /// A repo, `all` or `none`.
    pub repo: String,
    /// The plugin's old thread id it carries on; empty: the chat's own id.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub adopt: String,
}

/// What `attach` answers: the agent's brief, settings file and env; `ok` false with a refusal's `code` and `message`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Attached {
    /// It attached.
    pub ok: bool,
    /// The refusal's code, `OK` when attached.
    pub code: String,
    /// One line on what happened.
    pub message: String,
    /// Told to the agent ahead of the first message.
    pub brief: String,
    /// The agent's settings file; empty: none.
    pub settings: String,
    /// Set in the agent's environment.
    pub env: BTreeMap<String, String>,
}

/// One of a manifest's `[[groups]]` (#852): where its open rows sort in a chat's Asks, in the manifest's order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    /// The id its rows name.
    pub id: String,
    /// Its heading.
    pub label: String,
    /// Its rows sort first and mark the chip red.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub urgent: bool,
}

/// A page of a plugin's rows: `seq` is its newest.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Rows {
    /// The newest row's seq.
    pub seq: u64,
    /// The rows, oldest first.
    pub rows: Vec<Row>,
}

/// One row a plugin made: a later one with its `key` replaces it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Row {
    /// Its place in the plugin's log.
    pub seq: u64,
    /// Its identity.
    pub key: String,
    /// The chat it goes to, or the cards place.
    pub to: To,
    /// One plain line.
    pub text: String,
    /// The block the plugin built.
    pub block: Value,
    /// It still waits for the owner.
    pub open: bool,
    /// The manifest group it sorts under in a chat's Asks (#852); empty: none.
    #[serde(default)]
    pub group: String,
    /// The line typed into the chat's agent.
    #[serde(deserialize_with = "Option::deserialize")]
    pub prompt: Option<String>,
    /// The notification sent to the owner's devices.
    #[serde(deserialize_with = "Option::deserialize")]
    pub push: Option<Push>,
}

/// Where a row goes: a chat by its id, the cards place, or the owner's devices only.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "k", rename_all = "kebab-case")]
#[expect(missing_docs, reason = "the enum's line names each variant in order")]
pub enum To {
    Thread { id: String },
    Desk {},
    Owner,
}

/// A row's notification; a newer one with its `tag` replaces it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Push {
    /// Its title.
    pub title: String,
    /// Its line.
    pub body: String,
    /// What it replaces.
    pub tag: String,
}

/// An owner line of an attached chat, as `said` takes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Said {
    /// The item's id: posting it twice logs it once.
    pub id: String,
    /// When it was said, in Unix seconds.
    pub ts: u64,
    /// `owner`.
    pub from: String,
    /// The line.
    pub text: String,
}
