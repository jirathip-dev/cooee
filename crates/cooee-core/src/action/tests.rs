use serde_json::json;

use super::*;

fn call<'a>(kind: Option<&'a str>, locations: &'a [PathBuf], raw: Option<&'a Value>) -> Call<'a> {
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
    let rows: Vec<(&str, crate::Body)> = providers.iter().map(|(a, u)| (*a, row(a, u))).collect();
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
    same_reads_and_edits();
    same_runs_and_tools();
}

fn same_reads_and_edits() {
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
}

fn same_runs_and_tools() {
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
