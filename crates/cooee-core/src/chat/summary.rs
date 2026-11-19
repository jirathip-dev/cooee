use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The turn in progress: what the page shows beyond the committed items.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Turn {
    /// The reply so far.
    pub text: String,
    /// shed's reported state (`working`, `blocked`, `done`, `error`), `limited` (shed's `rate-limited`), or `ended`.
    pub state: String,
    /// The state's note: `limited`'s is the reset time, when the harness says one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The session's context use, as the agent last reported it (ACP's `usage_update`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<shed_proto::AcpUsage>,
}

impl Turn {
    /// Takes shed's reported `state` and its `note`, shed's `rate-limited` as the neutral `limited`.
    pub fn report(&mut self, state: &str, note: Option<String>) {
        let state = if state == "rate-limited" {
            "limited"
        } else {
            state
        };
        state.clone_into(&mut self.state);
        self.note = note;
    }
}

/// One row of the thread list.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Summary {
    /// The thread's id.
    pub id: String,
    /// Its title.
    pub title: String,
    /// The agent.
    pub agent: String,
    /// The provider account it runs with (empty: the system default).
    pub account: String,
    /// Its agent profile's id (empty: Plain), or its persona's.
    pub profile: String,
    /// Its persona's id (#126; empty: none, or one deleted).
    pub persona: String,
    /// Its persona's name (#159; empty: none).
    pub persona_name: String,
    /// Its persona's role (#159).
    pub persona_role: crate::agents::Role,
    /// The working folder.
    pub folder: PathBuf,
    /// The machine it runs on (#148; empty: this host).
    pub machine: String,
    /// The state: `connecting`, `unconnected` (its start failed), `working`, `blocked`, `done`, `error`, `limited`,
    /// `ended` or empty.
    pub state: String,
    /// The last item's text, short.
    pub last: String,
    /// When the last item was made, in Unix milliseconds (none: no items).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<u64>,
    /// Whether it has items the owner hasn't seen.
    pub unread: bool,
    /// How many: the agent's items past the owner's read mark (the row's badge).
    pub unseen: usize,
    /// Archived: listed apart until restored.
    pub archived: bool,
    /// A setup chat (#270).
    pub setup: bool,
    /// A session a tool started (#33): its labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow: Option<crate::Workflow>,
    /// A room's member personas, in room order (#154).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<String>,
    /// A linked exchange's asker (#177): the thread whose persona asked this one's.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub asked_by: String,
    /// The routine that started it or sent its last message (#75; empty: none).
    pub routine: String,
}
