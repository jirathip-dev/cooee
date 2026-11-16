use std::sync::Arc;

use crate::{Entry, Plain, Spec};

pub const ENTRY: Entry = Entry {
    name: "codex",
    defaults,
    build: |s| Arc::new(Plain(s)),
};

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
