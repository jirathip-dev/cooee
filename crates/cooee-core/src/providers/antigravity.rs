use std::sync::Arc;

use serde_json::json;

use crate::{
    Entry, Provider, Spec,
    action::Kind,
    provider::{Builtin, Channel, Gap, Setup, Want},
};

/// Google Antigravity's ACP server: the brief and its built-ins off through `_meta.antigravity`.
#[derive(Debug)]
pub struct Antigravity(Spec);

pub const ENTRY: Entry = Entry {
    name: "antigravity",
    defaults,
    keep: &[],
    build: |s| Arc::new(Antigravity(s)),
};

const OFFABLE: [(Builtin, &[&str]); 2] = [
    (Builtin::Web, &["search_web", "read_url_content"]),
    (Builtin::SubAgents, &["invoke_subagent", "run_workflow"]),
];

const STILL_ON: [Builtin; 2] = [Builtin::Memory, Builtin::Skills];

impl Provider for Antigravity {
    fn spec(&self) -> &Spec {
        &self.0
    }

    fn session(&self, want: &Want) -> Setup {
        let off = |b: &Builtin| want.off.contains(b);
        let disabled: Vec<&str> = (OFFABLE.iter().filter(|(b, _)| off(b)))
            .flat_map(|(_, names)| names.iter().copied())
            .collect();
        let mut gaps = vec![Gap::PersonalFiles];
        gaps.extend(STILL_ON.iter().filter(|b| off(b)).copied().map(Gap::StillOn));
        if want.ask_all {
            gaps.extend([Kind::Read, Kind::Search].map(Gap::AsksNot));
            gaps.extend((!off(&Builtin::Web)).then_some(Gap::AsksNot(Kind::Fetch)));
        }
        Setup {
            meta: Some(json!({ "antigravity": {
                "systemPrompt": { "append": want.brief },
                "disabledTools": disabled,
            } })),
            brief: Channel::System,
            gaps,
            ..Setup::default()
        }
    }
}

fn defaults() -> Spec {
    let tokens = "\"${GEMINI_HOME:-$HOME/.gemini}/antigravity-acp/acp_token.json\"";
    let sh = |line: String| vec!["sh".into(), "-c".into(), line];
    Spec {
        name: "antigravity".into(),
        agent: "antigravity-acp".into(),
        login: vec!["agy".into()],
        status: sh(format!("test -s {tokens} && echo signed in with Google")),
        install: "curl -fsSL https://antigravity.google/cli/install.sh | bash".into(),
        cheapest: "flash".into(),
        ..Spec::default()
    }
}
