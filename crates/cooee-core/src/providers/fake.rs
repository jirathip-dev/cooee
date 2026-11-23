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
    build: |s| Arc::new(Fake(s)),
};

fn defaults() -> Spec {
    Spec {
        name: "fake".into(),
        agent: "fake-acp".into(),
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
    use serde_json::json;
    use shed_catalog::Catalog;
    use shed_client::{
        fake::Server,
        proto::{ErrorCode, Method, RpcError},
    };

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
        let none = Catalog::default();
        assert!(provider::specs(Some(&[row("fake")]), &none).is_ok());
        assert_eq!(
            provider::specs(Some(&[row("nope")]), &none),
            Err("providers: no provider nope to base fake on".to_owned())
        );
    }

    fn specs(p: &Providers) -> Vec<Spec> {
        p.iter().map(|p| p.spec().clone()).collect()
    }

    #[test]
    fn a_shed_that_cant_be_asked_leaves_the_defaults_over_the_built_in_records() {
        let server = Server::start().unwrap();
        server.fail(Method::Agents, RpcError::new(ErrorCode::Internal, "down"));
        let all = Providers::load(None, &crate::Shed::new(server.socket().parent()));
        let defaults = provider::defaults();
        assert_eq!(specs(&all)[..defaults.len()], defaults);
        assert!(all.record("claude-acp").is_some_and(|r| r.login.is_some()));
    }

    #[test]
    fn shed_records_add_a_provider_per_program_that_logs_in_and_a_row_changes_only_its_fields() {
        let server = Server::start().unwrap();
        server.answer(
            Method::Agents,
            json!([
                {"id":"fake-acp","provider":{"id":"fake-acp","agent":"fake","login":"fake auth login","status":"fake auth status","efforts":["low","high"]}},
                {"id":"new-acp","provider":{"id":"new-acp","agent":"newbie","login":"new in","status":"new st"}},
                {"id":"half-acp","provider":{"id":"half-acp","agent":"half","login":"half in"}}
            ]),
        );
        let all = Providers::load(None, &crate::Shed::new(server.socket().parent()));
        assert_eq!(all.for_agent("fake-acp").unwrap().spec(), &defaults());
        assert_eq!(all.record("fake-acp").unwrap().efforts, ["low", "high"]);
        assert_eq!(all.named("newbie").unwrap().spec().agent, "new-acp");
        assert!(all.named("half").is_none());

        let row: toml::Table =
            toml::from_str("name = \"fake\"\ncheapest = \"tiny\"\nefforts = [\"max\"]").unwrap();
        let catalog = Catalog::default();
        let only = provider::specs(Some(std::slice::from_ref(&row)), &catalog).unwrap();
        let cheapest = "tiny".to_owned();
        assert_eq!(only, [Spec { cheapest, ..defaults() }]);
    }
}
