use std::sync::Arc;

use crate::{Entry, Plain, Spec};

pub const ENTRY: Entry = Entry {
    name: "gemini",
    defaults,
    build: |s| Arc::new(Plain(s)),
};

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
        ..Spec::default()
    }
}
