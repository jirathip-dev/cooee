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
mod tests {
    use serde_json::json;

    use super::*;

    fn call<'a>(
        kind: Option<&'a str>,
        locations: &'a [PathBuf],
        raw: Option<&'a Value>,
    ) -> Call<'a> {
        Call {
            title: "The title",
            kind,
            locations,
            raw_input: raw,
        }
    }

    #[test]
    fn acps_kinds_read_as_actions_with_their_subjects() {
        let at = [PathBuf::from("src/a.rs"), PathBuf::from("src/b.rs")];
        let run = json!({ "command": ["cargo", "test"] });
        let fetch = json!({ "url": "https://docs.rs/csv" });
        let cases = [
            (call(Some("read"), &at, None), Kind::Read, "src/a.rs"),
            (call(Some("move"), &at, None), Kind::Edit, "src/a.rs"),
            (call(Some("search"), &[], None), Kind::Search, "The title"),
            (
                call(Some("execute"), &[], Some(&run)),
                Kind::Run,
                "cargo test",
            ),
            (
                call(Some("fetch"), &[], Some(&fetch)),
                Kind::Fetch,
                "https://docs.rs/csv",
            ),
            (call(Some("think"), &at, None), Kind::Other, "The title"),
            (call(None, &[], None), Kind::Other, "The title"),
        ];
        for (c, kind, subject) in cases {
            let a = Action::plain(&c);
            assert_eq!((a.kind, a.subject.as_str()), (kind, subject), "{c:?}");
        }
    }

    type Update = (
        &'static str,
        Option<&'static str>,
        &'static [&'static str],
        Value,
    );

    const CSV: &[&str] = &["/w/crates/export/src/csv.rs"];

    fn row(agent: &str, updates: &[Update]) -> crate::Body {
        let providers = crate::Providers::new(&crate::provider::defaults());
        let read = |c: &Call| providers.action(agent, c);
        let mut chat = crate::Chat::default();
        for (seq, (title, kind, at, raw)) in (1..).zip(updates) {
            let event = shed_proto::EventKind::ToolCall {
                id: "t1".into(),
                title: (*title).into(),
                status: if seq == updates.len() {
                    "completed"
                } else {
                    "in_progress"
                }
                .into(),
                kind: kind.map(str::to_owned),
                locations: at.iter().map(PathBuf::from).collect(),
                raw_input: (!raw.is_null()).then(|| raw.clone()),
            };
            chat.apply(1, seq.try_into().unwrap(), event, &read);
        }
        chat.items.pop().unwrap().body
    }

    fn same(action: &str, providers: &[(&str, Vec<Update>)]) {
        let rows: Vec<(&str, crate::Body)> =
            providers.iter().map(|(a, u)| (*a, row(a, u))).collect();
        let crate::Body::Tool { action: kind, .. } = &rows[0].1 else {
            panic!("{action}: no tool row")
        };
        assert_eq!(serde_json::to_value(kind).unwrap(), json!(action));
        for (agent, got) in &rows {
            assert_eq!(got, &rows[0].1, "{action} from {agent}");
        }
    }

    #[test]
    fn the_same_action_from_each_providers_tool_name_gives_the_same_row() {
        let file = json!({ "file_path": CSV[0] });
        same(
            "read",
            &[
                (
                    "claude-acp",
                    vec![
                        (
                            "Read crates/export/src/csv.rs",
                            Some("read"),
                            CSV,
                            file.clone(),
                        ),
                        (
                            "Read crates/export/src/csv.rs",
                            Some("read"),
                            &[],
                            Value::Null,
                        ),
                    ],
                ),
                (
                    "codex-acp",
                    vec![(
                        "Read file '/w/crates/export/src/csv.rs'",
                        Some("read"),
                        CSV,
                        Value::Null,
                    )],
                ),
                (
                    "gemini",
                    vec![("crates/export/src/csv.rs", Some("read"), CSV, Value::Null)],
                ),
            ],
        );
        same(
            "edit",
            &[
                (
                    "claude-acp",
                    vec![("Edit `crates/export/src/csv.rs`", Some("edit"), CSV, file)],
                ),
                (
                    "codex-acp",
                    vec![(
                        "Edit crates/export/src/csv.rs",
                        Some("edit"),
                        CSV,
                        Value::Null,
                    )],
                ),
                (
                    "gemini",
                    vec![("csv.rs: let a => let b", Some("edit"), CSV, Value::Null)],
                ),
            ],
        );
        let shell = json!({ "command": "/bin/zsh -lc 'cargo test -p export'", "cwd": "/w" });
        same(
            "run",
            &[
                (
                    "claude-acp",
                    vec![
                        (
                            "cargo test -p export",
                            Some("execute"),
                            &[],
                            json!({ "command": "cargo test -p export" }),
                        ),
                        ("cargo test -p export", Some("execute"), &[], Value::Null),
                    ],
                ),
                (
                    "codex-acp",
                    vec![("cargo test -p export", Some("execute"), &[], shell)],
                ),
            ],
        );
        let input = json!({ "server": "cooee", "tool": "remember", "arguments": {} });
        same(
            "tool",
            &[
                (
                    "claude-acp",
                    vec![("mcp__cooee__remember", None, &[], json!({ "text": "q3" }))],
                ),
                (
                    "codex-acp",
                    vec![
                        ("mcp.cooee.remember", Some("execute"), &[], input),
                        ("mcp.cooee.remember", Some("execute"), &[], Value::Null),
                    ],
                ),
            ],
        );
    }

    #[test]
    fn a_row_stored_before_kinds_reads_as_other_with_its_title() {
        let old = json!({ "id": 1, "at": 1, "kind": "tool", "call": "c", "title": "Read a.rs", "status": "completed" });
        let item: crate::Item = serde_json::from_value(old).unwrap();
        let crate::Body::Tool {
            subject, action, ..
        } = item.body
        else {
            panic!("not a tool row")
        };
        assert_eq!((subject.as_str(), action), ("Read a.rs", Kind::Other));
    }
}
