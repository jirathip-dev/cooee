use std::sync::Arc;

use serde_json::json;

use crate::{
    Entry, Provider, Spec,
    provider::{Builtin, Gap, Setup, Want},
};

/// Gemini CLI: its built-ins off through a system settings file in the run folder; the brief as the first prompt, and
/// its global files have no switch.
#[derive(Debug)]
pub struct Gemini(Spec);

pub const ENTRY: Entry = Entry {
    name: "gemini",
    defaults,
    keep: &[],
    build: |s| Arc::new(Gemini(s)),
};

const KEPT: &str = "read_file write_file replace glob search_file_content list_directory run_shell_command write_todos";

const OFFABLE: [(Builtin, &[&str]); 2] = [
    (Builtin::Web, &["google_web_search", "web_fetch"]),
    (Builtin::Memory, &["save_memory"]),
];

const SETTINGS: &str = "gemini-settings.json";

impl Provider for Gemini {
    fn spec(&self) -> &Spec {
        &self.0
    }

    fn session(&self, want: &Want) -> Setup {
        let mut core = KEPT.split(' ').collect::<Vec<_>>();
        for (b, names) in OFFABLE {
            if !want.off.contains(&b) {
                core.extend(names);
            }
        }
        let settings = json!({
            "tools": { "core": core },
            "skills": { "enabled": !want.off.contains(&Builtin::Skills) },
            "experimental": { "enableAgents": !want.off.contains(&Builtin::SubAgents) },
        });
        let path = want.run.join(SETTINGS).display().to_string();
        Setup {
            env: vec![("GEMINI_CLI_SYSTEM_SETTINGS_PATH".to_owned(), path)],
            files: vec![(SETTINGS.into(), settings.to_string())],
            gaps: vec![Gap::PersonalFiles],
            ..Setup::default()
        }
    }
}

fn defaults() -> Spec {
    let creds = "\"${GEMINI_CLI_HOME:-$HOME}/.gemini/oauth_creds.json\"";
    let sh = |line: String| vec!["sh".into(), "-c".into(), line];
    Spec {
        name: "gemini".into(),
        agent: "gemini".into(),
        login: vec!["gemini".into()],
        status: sh(format!("test -s {creds} && echo signed in with Google")),
        install: "npm install -g @google/gemini-cli".into(),
        home: "GEMINI_CLI_HOME".into(),
        logout: sh(format!("rm -f {creds}")),
        cheapest: "flash".into(),
        ..Spec::default()
    }
}
