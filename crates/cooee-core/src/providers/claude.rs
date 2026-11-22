use std::sync::Arc;

use serde_json::json;

use crate::{
    Entry, Provider, Spec,
    action::{Action, Call},
    provider::{Builtin, Channel, Setup, Want},
};

/// Claude Code: a persona's session options and a profile's settings in claude-acp's `_meta.claudeCode.options`, and
/// its MCP tools read from their `mcp__<server>__<tool>` titles.
#[derive(Debug)]
pub struct Claude(Spec);

pub const ENTRY: Entry = Entry {
    name: "claude",
    defaults,
    keep: &[],
    build: |s| Arc::new(Claude(s)),
};

const KEPT: &str = "Read Write Edit NotebookEdit Glob Grep Bash TaskStop TodoWrite TaskCreate TaskGet TaskUpdate TaskList";

const ASKS: [&str; 7] = ["Read", "Glob", "Grep", "Bash", "Write", "Edit", "NotebookEdit"];

const OFFABLE: [(Builtin, &[&str]); 3] = [
    (Builtin::Web, &["WebSearch", "WebFetch"]),
    (Builtin::SubAgents, &["Agent", "Workflow"]),
    (Builtin::Skills, &["Skill"]),
];

fn defaults() -> Spec {
    let v = |s: &str| s.split(' ').map(str::to_owned).collect();
    Spec {
        name: "claude".into(),
        agent: "claude-acp".into(),
        login: v("claude auth login"),
        status: v("claude auth status"),
        install: "npm install -g @anthropic-ai/claude-code".into(),
        settings: "claudeCode.options.settings".into(),
        cheapest: "haiku".into(),
        ..Spec::default()
    }
}

impl Provider for Claude {
    fn spec(&self) -> &Spec {
        &self.0
    }

    fn session(&self, want: &Want) -> Setup {
        let mut tools = KEPT.split(' ').collect::<Vec<_>>();
        for (b, names) in OFFABLE {
            if !want.off.contains(&b) {
                tools.extend(names);
            }
        }
        let excludes = ["{/Users/*,/home/*,/root}/.claude/{CLAUDE.md,rules/**}"];
        let mut settings = json!({ "claudeMdExcludes": excludes,
                                   "autoMemoryEnabled": !want.off.contains(&Builtin::Memory) });
        if want.ask_all {
            let ask: Vec<&str> = tools.iter().copied().filter(|t| ASKS.contains(t) || t.starts_with("Web")).collect();
            let ask = json!({ "defaultMode": "default", "disableAutoMode": "disable",
                              "disableBypassPermissionsMode": "disable", "ask": ask });
            settings.as_object_mut().map(|s| s.insert("permissions".to_owned(), ask));
        }
        let options = json!({
            "settingSources": ["project", "local"],
            "strictMcpConfig": true,
            "tools": tools,
            "settings": settings,
        });
        Setup {
            meta: Some(json!({
                "systemPrompt": { "append": want.brief, "excludeDynamicSections": true },
                "claudeCode": { "options": options },
            })),
            brief: Channel::System,
            ..Setup::default()
        }
    }

    fn action(&self, call: &Call) -> Action {
        let mcp = call.title.strip_prefix("mcp__").and_then(|t| t.split_once("__"));
        match mcp {
            Some((server, tool)) if !server.is_empty() && !tool.is_empty() => {
                Action::tool(server, tool)
            }
            _ => Action::plain(call),
        }
    }
}
