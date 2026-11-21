use std::sync::Arc;

use serde_json::json;

use crate::{
    Entry, Provider, Spec,
    action::{Action, Call, Kind},
    provider::{Builtin, Channel, Gap, Setup, Want},
};

/// Codex: the brief as developer instructions and its built-ins off through `CODEX_CONFIG`; its global files have no
/// switch; codex-acp titles an MCP call `mcp.<server>.<tool>` and a command with its shell prefix stripped.
#[derive(Debug)]
pub struct Codex(Spec);

pub const ENTRY: Entry = Entry {
    name: "codex",
    defaults,
    keep: &["login"],
    build: |s| Arc::new(Codex(s)),
};

impl Provider for Codex {
    fn spec(&self) -> &Spec {
        &self.0
    }

    fn action(&self, call: &Call) -> Action {
        let mcp = call.title.strip_prefix("mcp.").and_then(|t| t.split_once('.'));
        match (mcp, Action::plain(call)) {
            (Some((server, tool)), _) if !server.is_empty() && !tool.is_empty() => {
                Action::tool(server, tool)
            }
            (_, Action { kind: Kind::Run, .. }) => Action {
                kind: Kind::Run,
                subject: call.title.to_owned(),
            },
            (_, plain) => plain,
        }
    }

    fn session(&self, want: &Want) -> Setup {
        let on = |b| !want.off.contains(&b);
        let mut config = json!({
            "developer_instructions": want.brief,
            "tools": { "web_search": on(Builtin::Web) },
            "features": { "multi_agent": on(Builtin::SubAgents), "memories": on(Builtin::Memory) },
            "skills": { "bundled": { "enabled": on(Builtin::Skills) } },
        });
        let (mut gaps, mut env) = (vec![Gap::PersonalFiles], Vec::new());
        if want.ask_all {
            gaps.extend([Kind::Read, Kind::Search, Kind::Run].map(Gap::AsksNot));
            let policy = ("approval_policy".to_owned(), json!("on-request"));
            config.as_object_mut().map(|c| c.insert(policy.0, policy.1));
            env.push(("INITIAL_AGENT_MODE".to_owned(), "read-only".to_owned()));
        }
        env.push(("CODEX_CONFIG".to_owned(), config.to_string()));
        Setup {
            env,
            brief: Channel::System,
            gaps,
            ..Setup::default()
        }
    }
}

fn defaults() -> Spec {
    let v = |s: &str| s.split(' ').map(str::to_owned).collect();
    Spec {
        name: "codex".into(),
        agent: "codex-acp".into(),
        login: v("codex login --device-auth"),
        status: v("codex login status"),
        install: "npm install -g @openai/codex".into(),
        home: "CODEX_HOME".into(),
        logout: v("codex logout"),
        efforts: v("low medium high"),
        effort: v("-c model_reasoning_effort={}"),
        cheapest: "mini".into(),
        ..Spec::default()
    }
}

#[cfg(test)]
mod tests {
    use crate::Providers;

    #[test]
    fn picks_fill_the_providers_words() {
        let p = Providers::new(&[super::defaults()]);
        let codex = p.named("codex").unwrap();
        assert_eq!(
            codex.picks("high"),
            Some(vec![
                "-c".to_owned(),
                "model_reasoning_effort=high".to_owned()
            ])
        );
        assert_eq!(codex.picks(""), Some(Vec::new()));
        assert_eq!(codex.picks("max"), None);
    }
}
