use std::sync::Arc;

use crate::{
    Entry, Provider, Spec,
    action::{Action, Call},
};

#[derive(Debug)]
pub struct Fake(Spec);

pub const ENTRY: Entry = Entry {
    name: "fake",
    defaults,
    keep: &[],
    build: |s| Arc::new(Fake(s)),
};

fn defaults() -> Spec {
    Spec {
        name: "fake".into(),
        agent: "fake-acp".into(),
        status: vec!["true".into()],
        ..Spec::default()
    }
}

impl Provider for Fake {
    fn spec(&self) -> &Spec {
        &self.0
    }

    fn action(&self, _call: &Call) -> Action {
        Action::tool("fake", "read")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Providers, provider};

    #[test]
    fn a_new_file_and_one_registry_line_is_a_provider() {
        assert!(provider::defaults().iter().any(|s| s.name == "fake"));
        let call = Call {
            title: "Read x",
            kind: Some("read"),
            locations: &[],
            raw_input: None,
        };
        let read = |spec: Spec| Providers::new(&[spec]).action("fake-acp", &call);
        assert_eq!(read(defaults()), Action::tool("fake", "read"));
        let based = Spec {
            name: "other".into(),
            base: Some("fake".into()),
            ..defaults()
        };
        assert_eq!(read(based), Action::tool("fake", "read"));
        let plain = Spec {
            name: "other".into(),
            ..defaults()
        };
        assert_eq!(read(plain), Action::plain(&call));
    }

    #[test]
    fn a_base_no_provider_has_is_refused() {
        let row = |base: &str| -> toml::Table {
            toml::from_str(&format!("name = \"fake\"\nbase = \"{base}\"")).unwrap()
        };
        assert!(provider::specs(Some(&[row("fake")]), &[]).is_ok());
        assert_eq!(
            provider::specs(Some(&[row("nope")]), &[]),
            Err("providers: no provider nope to base fake on".to_owned())
        );
    }

    fn shed(name: &str, agents: &str) -> crate::Shed {
        use std::os::unix::fs::PermissionsExt;
        let dir = crate::scratch::owned(
            std::env::temp_dir().join(format!("cooee-records-{name}-{}", std::process::id())),
        );
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("agents.json"), agents).unwrap();
        let bin = dir.join("shed");
        std::fs::write(&bin, "#!/bin/sh\ncat \"$(dirname \"$0\")/agents.json\"\n").unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        crate::Shed::new(bin, None)
    }

    fn specs(p: &Providers) -> Vec<Spec> {
        p.iter().map(|p| p.spec().clone()).collect()
    }

    #[test]
    fn a_shed_with_no_records_leaves_the_built_in_defaults() {
        let shed = shed(
            "none",
            r#"[{"id":"claude-acp","version":"1.0.0","distribution":"npx","cmd":["claude-acp"]}]"#,
        );
        assert_eq!(shed.agents().unwrap()[0].cmd, ["claude-acp"]);
        assert_eq!(specs(&Providers::load(None, &shed)), provider::defaults());
    }

    #[test]
    fn shed_records_set_the_program_facts_and_a_row_changes_only_its_fields() {
        let shed = shed(
            "some",
            r#"[{"id":"fake-acp","source":"built-in","installed":{"id":"fake-acp","version":"2.0.0","distribution":"npx","cmd":["fake-acp"]},
                 "provider":{"id":"fake-acp","agent":"fake","login":"fake auth login","status":"fake auth status",
                             "effort_args":["--effort","{effort}"],"efforts":["low","high"],"limit":["hit your limit"]}},
                {"id":"codex-acp","source":"built-in","installed":null,
                 "provider":{"id":"codex-acp","agent":"codex","login":"codex login","status":"codex login status","efforts":["xhigh"]}},
                {"id":"new-acp","source":"/x/new.toml","installed":null,"provider":{"id":"new-acp","agent":"newbie","login":"new in","status":"new st"}},
                {"id":"half-acp","source":"/x/half.toml","installed":null,"provider":{"id":"half-acp","agent":"half","login":"half in"}}]"#,
        );
        let agents = shed.agents().unwrap();
        assert_eq!(agents.len(), 1, "{agents:?}");
        assert_eq!((agents[0].id.as_str(), agents[0].version.as_str()), ("fake-acp", "2.0.0"));
        let all = Providers::load(None, &shed);
        let words = |s: &str| s.split(' ').map(str::to_owned).collect::<Vec<_>>();
        let fake = all.for_agent("fake-acp").unwrap();
        let read = Spec {
            login: words("fake auth login"),
            status: words("fake auth status"),
            efforts: words("low high"),
            effort: words("--effort {}"),
            ..defaults()
        };
        assert_eq!(fake.spec(), &read);
        assert_eq!(fake.picks("high"), Some(words("--effort high")));
        let codex = all.named("codex").unwrap().spec();
        assert_eq!((codex.login.as_slice(), codex.efforts.as_slice()), (&words("codex login --device-auth")[..], &words("xhigh")[..]));
        let newbie = all.named("newbie").unwrap().spec();
        assert_eq!((newbie.agent.as_str(), &newbie.login), ("new-acp", &words("new in")));
        assert!(all.named("half").is_none());

        let row: toml::Table = toml::from_str("name = \"fake\"\nefforts = [\"max\"]").unwrap();
        let records = shed.records().unwrap();
        let only = provider::specs(Some(std::slice::from_ref(&row)), &records).unwrap();
        assert_eq!(only, [Spec { efforts: words("max"), ..read }]);
    }
}
