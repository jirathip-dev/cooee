use serde::{Deserialize, Serialize};

/// A room member's turn (#155): whose, and why it has one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomTurn {
    /// The member's persona id.
    pub member: String,
    /// Why it has the turn.
    pub reason: Reason,
}

/// Why a room member has a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reason {
    /// The owner's message named it.
    Mention,
    /// The owner's message named no one: every member gets one turn, in room order (G1).
    Round,
    /// Another member's reply named it.
    Chained,
}

/// A room's member: its persona, its session (a hidden thread, empty until its first turn) and the newest room
/// item it was given.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    /// The persona's id.
    pub persona: String,
    /// Its session's thread id.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub session: String,
    /// The newest room item id its session was given.
    #[serde(default)]
    pub heard: u64,
}
