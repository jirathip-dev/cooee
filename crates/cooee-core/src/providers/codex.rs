use std::sync::Arc;

use crate::{
    Entry, Provider, Spec,
    action::{Action, Call, Kind},
};

/// Codex: codex-acp titles an MCP call `mcp.<server>.<tool>` and a command with its shell prefix stripped.
#[derive(Debug)]
pub struct Codex(Spec);

pub const ENTRY: Entry = Entry {
    name: "codex",
    defaults,
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
