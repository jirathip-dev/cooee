//! A provider's behaviour behind one trait, each provider one module in `providers/` and one line in its registry.

use std::{fmt, sync::Arc};

use crate::{
    Usage,
    login::{Ran, Spec, Status, plain_status},
    providers::REGISTRY,
};

/// A provider's behaviour; every method but `spec` defaults to the plain, provider-free reading.
pub trait Provider: Send + Sync + fmt::Debug {
    /// Its data, as `[[providers]]` or its module's defaults set it.
    fn spec(&self) -> &Spec;

    /// Its signed-in state from the status command's run: exit 0 is signed in, the output its line.
    fn read_status(&self, ran: &Ran) -> Status {
        plain_status(ran)
    }

    /// The variables removed from every command it runs.
    fn scrub(&self) -> &'static [&'static str] {
        &[]
    }

    /// The variable a hand-placed `<data>/accounts/<name>/<account>.token` fills; none: no token files.
    fn token_var(&self) -> Option<&'static str> {
        None
    }

    /// Its usage command's stdout read at `now` (s); none: no window read.
    fn read_usage(&self, _stdout: &str, _now: u64) -> Option<Usage> {
        None
    }

    /// The session options (ACP `_meta`) that keep the owner's personal setup out of a profile's thread (#320).
    fn no_personal_setup(&self) -> Option<serde_json::Value> {
        None
    }
}

/// A spec with no registered behaviour.
#[derive(Debug)]
pub struct Plain(pub Spec);

impl Provider for Plain {
    fn spec(&self) -> &Spec {
        &self.0
    }
}

/// One registry line: a provider module's name, its defaults and its constructor.
pub struct Entry {
    /// The name a spec's `base` (else its `name`) picks it by.
    pub name: &'static str,
    /// Its spec when `[[providers]]` is not set.
    pub defaults: fn() -> Spec,
    /// Its behaviour over a spec.
    pub build: fn(Spec) -> Arc<dyn Provider>,
}

macro_rules! registry {
    ($($(#[$m:meta])* $name:ident),* $(,)?) => {
        $($(#[$m])* mod $name;)*
        pub(crate) const REGISTRY: &[$crate::provider::Entry] = &[$($(#[$m])* $name::ENTRY),*];
    };
}
pub(crate) use registry;

/// The built providers, in config (else registry) order.
#[derive(Debug, Clone, Default)]
pub struct Providers(Vec<Arc<dyn Provider>>);

impl Providers {
    /// Each spec built by the entry its `base`, else its `name`, names; else [`Plain`].
    #[must_use]
    pub fn new(specs: &[Spec]) -> Self {
        let build = |s: &Spec| {
            let name = s.base.as_deref().unwrap_or(&s.name);
            match REGISTRY.iter().find(|e| e.name == name) {
                Some(e) => (e.build)(s.clone()),
                None => Arc::new(Plain(s.clone())) as Arc<dyn Provider>,
            }
        };
        Self(specs.iter().map(build).collect())
    }

    /// Every provider, in order.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Provider>> {
        self.0.iter()
    }

    /// The provider named `name`.
    #[must_use]
    pub fn named(&self, name: &str) -> Option<&Arc<dyn Provider>> {
        self.iter().find(|p| p.spec().name == name)
    }

    /// The provider that signs in ACP agent `agent`.
    #[must_use]
    pub fn for_agent(&self, agent: &str) -> Option<&Arc<dyn Provider>> {
        self.iter().find(|p| p.spec().agent == agent)
    }
}

/// Refuses a spec whose `base` names no registered provider.
///
/// # Errors
/// The first such spec, said.
pub fn check(specs: &[Spec]) -> Result<(), String> {
    let unknown = |b: &&String| !REGISTRY.iter().any(|e| e.name == b.as_str());
    match specs
        .iter()
        .find_map(|s| Some((s, s.base.as_ref().filter(unknown)?)))
    {
        Some((s, base)) => Err(format!(
            "providers: no provider {base} to base {} on",
            s.name
        )),
        None => Ok(()),
    }
}

/// Every registered provider's defaults, in registry order.
#[must_use]
pub fn defaults() -> Vec<Spec> {
    REGISTRY.iter().map(|e| (e.defaults)()).collect()
}

/// The fix `cooee doctor` and the agent picker print when shed has no agent: the first registered one's.
#[must_use]
pub fn agents_fix() -> String {
    let first = REGISTRY.first().map(|e| (e.defaults)().agent);
    format!("shed agents add {}", first.unwrap_or_default())
}
