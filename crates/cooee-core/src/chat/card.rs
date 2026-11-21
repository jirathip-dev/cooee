use serde::{Deserialize, Serialize};

/// An approval's answer and a plugin card's form (#553 rows 30–33); every field is optional.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    /// Who answered: the device's label, or the plugin's own (a conductor's answer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// When it was answered, in Unix milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answered: Option<u64>,
    /// `config` (a diff to apply or discard), `secret` (a masked value), `thread` (pick a thread) or `run-card` (a
    /// herdsman run card's command to approve or deny); none: choices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    /// A config card's diff, a secret card's name, or a run card's command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// A run card's command sha, sent back with its `approve` (herdsman's `card approve --sha`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    /// The plugin's line once settled (`applied, doctor ✓`), or why it can't be answered (`stale: …`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    /// Until when a conductor's answer may be changed, in Unix milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<u64>,
    /// The owner's text sent with a deny (#351): the agent's next message, shown under the denial.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub said: Option<String>,
    /// A persona harness request's kind of action (#503).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<crate::action::Kind>,
    /// The harness request behind a persona's three-button card, kept so a restart can still answer it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<Box<crate::rules::Asked>>,
}
