use std::sync::Arc;

use crate::{Entry, Provider, Ran, Spec, Status, Usage, Window};

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
        status: vec!["true".into()],
        ..Spec::default()
    }
}

impl Provider for Fake {
    fn spec(&self) -> &Spec {
        &self.0
    }

    fn read_status(&self, ran: &Ran) -> Status {
        Status::said(ran.stdout.trim() == "yes", "fake reading")
    }

    fn read_usage(&self, out: &str, now: u64) -> Option<Usage> {
        let used_pct = out.trim().parse().ok()?;
        Some(Usage {
            five_hour: Some(Window {
                used_pct,
                resets_at: now + 60,
            }),
            seven_day: None,
            read_at: now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Providers, provider};

    #[test]
    fn a_new_file_and_one_registry_line_is_a_provider() {
        assert!(provider::defaults().iter().any(|s| s.name == "fake"));
        let sh = |line: &str| vec!["sh".into(), "-c".into(), line.into()];
        let spec = Spec {
            status: sh("echo yes; exit 1"),
            usage: sh("echo 40"),
            ..defaults()
        };
        let p = Providers::new(std::slice::from_ref(&spec));
        let fake = p.named("fake").unwrap();
        let s = fake.check(None);
        assert_eq!((s.signed_in, s.line.as_str()), (true, "fake reading"));
        let u = fake.usage(None, 7).unwrap();
        assert_eq!((u.five_hour.unwrap().used_pct, u.read_at), (40, 7));
        let based = Spec {
            name: "other".into(),
            base: Some("fake".into()),
            ..spec.clone()
        };
        assert_eq!(Providers::new(&[based]).named("other").unwrap().check(None).line, "fake reading");
        let plain = Spec {
            name: "other".into(),
            ..spec
        };
        let plain = Providers::new(&[plain]);
        let s = plain.named("other").unwrap().check(None);
        assert_eq!((s.signed_in, s.line.as_str()), (false, "yes"));
        assert_eq!(plain.named("other").unwrap().usage(None, 7), None);
    }

    #[test]
    fn a_base_no_provider_has_is_refused() {
        let spec = |base: &str| Spec {
            base: Some(base.into()),
            ..defaults()
        };
        assert_eq!(provider::check(&[spec("fake")]), Ok(()));
        assert_eq!(
            provider::check(&[spec("nope")]),
            Err("providers: no provider nope to base fake on".to_owned())
        );
    }
}
