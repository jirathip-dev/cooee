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
    /// The line typed into the chat's agent.
    #[serde(deserialize_with = "Option::deserialize")]
    pub prompt: Option<String>,
    /// The notification sent to the owner's devices.
    #[serde(deserialize_with = "Option::deserialize")]
    pub push: Option<Push>,
}

/// Where a row goes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "k", rename_all = "kebab-case")]
pub enum To {
    /// A chat, by its id.
    Thread {
        /// The chat's id.
        id: String,
    },
    /// The cards place.
    Desk {},
    /// The owner's devices only.
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
