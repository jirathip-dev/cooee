//! What a tool call does and to what (#503): read from ACP's facts by the provider, never drawn from a harness's tool name.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The kind of action a tool row draws.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Reads a file.
    Read,
    /// Writes, deletes or moves a file.
    Edit,
    /// Runs a command.
    Run,
    /// Searches files.
    Search,
    /// Fetches a URL.
    Fetch,
    /// Calls an MCP server's tool.
    Tool,
    /// Anything else; also every row stored before kinds were read.
    #[default]
    Other,
}

impl Kind {
    /// Every kind.
    pub const ALL: [Self; 7] = [
        Self::Read,
        Self::Edit,
        Self::Run,
        Self::Search,
        Self::Fetch,
        Self::Tool,
        Self::Other,
    ];

    /// What an action of this kind does, as a verb phrase.
    #[must_use]
    pub const fn verb(self) -> &'static str {
        match self {
            Self::Read => "reads a file",
            Self::Edit => "edits a file",
            Self::Run => "runs a command",
            Self::Search => "searches files",
            Self::Fetch => "fetches a page",
            Self::Tool => "calls a tool",
            Self::Other => "acts",
        }
    }
}

/// What a tool call does and to what: a path, a command, a query, a URL, `server/tool`, or the call's title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    /// Its kind.
    pub kind: Kind,
    /// What it acts on.
    pub subject: String,
}

/// ACP's facts about a tool call, as shed passes them on.
#[derive(Debug, Clone, Copy)]
pub struct Call<'a> {
    /// The agent's title.
    pub title: &'a str,
    /// ACP's kind: read, edit, delete, move, search, execute, think, fetch, other.
    pub kind: Option<&'a str>,
    /// The paths ACP's `locations` name.
    pub locations: &'a [PathBuf],
    /// ACP's `rawInput`.
    pub raw_input: Option<&'a Value>,
}

impl Action {
    /// The reading of ACP's own fields: its kind, and the first location, the command or the URL as the subject.
    #[must_use]
    pub fn plain(call: &Call) -> Self {
        let kind = match call.kind {
            Some("read") => Kind::Read,
            Some("edit" | "delete" | "move") => Kind::Edit,
            Some("execute") => Kind::Run,
            Some("search") => Kind::Search,
            Some("fetch") => Kind::Fetch,
            _ => Kind::Other,
        };
        let input = |k: &str| call.raw_input.and_then(|v| v.get(k));
        let subject = match kind {
            Kind::Read | Kind::Edit | Kind::Search => {
                call.locations.first().map(|p| p.display().to_string())
            }
            Kind::Run => input("command").and_then(words),
            Kind::Fetch => input("url").and_then(Value::as_str).map(str::to_owned),
            Kind::Tool | Kind::Other => None,
        };
        Self {
            kind,
            subject: subject.unwrap_or_else(|| call.title.to_owned()),
        }
    }

    /// An MCP tool call on `server`.
    #[must_use]
    pub fn tool(server: &str, tool: &str) -> Self {
        Self {
            kind: Kind::Tool,
            subject: format!("{server}/{tool}"),
        }
    }
}

fn words(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Array(a) => Some(
            a.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" "),
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
