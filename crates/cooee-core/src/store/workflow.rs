use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A session a tool started, as its shed labels say (#33): `source` (the tool), `group` (its repo), `item`
/// (`#issue`), `role`, `round` and an optional `url`. Any tool may set them; cooee names none.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workflow {
    /// The tool that started it.
    pub source: String,
    /// What it works on (a repo).
    #[serde(default)]
    pub group: String,
    /// Which item of it (`#33`).
    #[serde(default)]
    pub item: String,
    /// Its role (`impl`, `rev`, …).
    #[serde(default)]
    pub role: String,
    /// Its round.
    #[serde(default)]
    pub round: String,
    /// A link to what it works on.
    #[serde(default)]
    pub url: String,
}

impl Workflow {
    /// The session's labels as a Workflow item: one with a `source` that cooee didn't start itself.
    #[must_use]
    pub fn from_labels(labels: &BTreeMap<String, String>) -> Option<Self> {
        if labels.keys().any(|k| k.starts_with("cooee")) {
            return None;
        }
        let get = |k: &str| labels.get(k).cloned().unwrap_or_default();
        let source = get("source");
        (!source.is_empty()).then(|| Self {
            source,
            group: get("group"),
            item: get("item"),
            role: get("role"),
            round: get("round"),
            url: get("url"),
        })
    }
}
