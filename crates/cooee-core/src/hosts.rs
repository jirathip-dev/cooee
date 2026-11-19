//! What the owner says of each machine besides shed's health (`[machines.<name>]` in config.toml, #383).

use serde::Deserialize;

/// `[machines.<name>]`: a machine's tags and, for a cloud machine, its monthly budget and how to read its spend.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Host {
    /// Its tags, shown on its card.
    pub tags: Vec<String>,
    /// Its budget this month, in USD.
    pub budget: Option<f64>,
    /// A command printing its spend this month in USD, run each poll; with `budget`, what makes it a cloud machine.
    pub spend_cmd: Vec<String>,
}
