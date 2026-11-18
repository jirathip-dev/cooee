//! A provider's behaviour behind one trait, each provider one module in `providers/` and one line in its registry.

use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde_json::Value;

use crate::{
    Usage,
    action::Kind,
    login::{Ran, Spec, Status, plain_status},
    providers::REGISTRY,
};

/// What cooee wants of a persona session: the same for every provider.
#[derive(Debug, Clone, Copy)]
pub struct Want<'a> {
    /// The persona's brief, rendered once; never changes for the session.
    pub brief: &'a str,
    /// The built-ins to turn off.
    pub off: &'a [Builtin],
    /// Ask before every action.
    pub ask_all: bool,
    /// The folder a provider's files for its harness go in.
    pub run: &'a Path,
}

/// A harness built-in cooee replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    /// Web search and fetch.
    Web,
    /// A browser.
    Browser,
    /// The harness's own memory.
    Memory,
    /// Sub-agents.
    SubAgents,
    /// The harness's own skills.
    Skills,
}

/// How one harness is told: only what `Launch` carries, where the brief goes, and what could not be done.
#[derive(Debug, Default)]
pub struct Setup {
    /// ACP `_meta`.
    pub meta: Option<Value>,
    /// Variables on the adapter's command line.
    pub env: Vec<(String, String)>,
    /// Words after the adapter's command.
    pub args: Vec<String>,
    /// Files relative to [`Want::run`], written 0600 before the start.
    pub files: Vec<(PathBuf, String)>,
    /// Where the brief goes.
    pub brief: Channel,
    /// What this harness can't be told.
    pub gaps: Vec<Gap>,
}

impl Setup {
    /// Nothing set, the brief as the first prompt, and every gap.
    #[must_use]
    pub fn plain(want: &Want) -> Self {
        let mut gaps = vec![Gap::PersonalFiles];
        gaps.extend(want.off.iter().copied().map(Gap::StillOn));
        if want.ask_all {
            gaps.extend(Kind::ALL.into_iter().map(Gap::AsksNot));
        }
        Self {
            gaps,
            ..Self::default()
        }
    }
}

/// Where a session's brief goes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Channel {
    /// The harness's system channel, kept with the vendor's prompt.
    System,
    /// Ahead of the first message.
    #[default]
    FirstPrompt,
}

/// Something this harness can't be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gap {
    /// It reads the owner's global instructions, settings and MCP servers.
    PersonalFiles,
    /// A built-in it can't turn off.
    StillOn(Builtin),
    /// It doesn't ask before an action of this kind.
    AsksNot(Kind),
}

impl Gap {
    /// The line the thread, the agent picker and the nightly report show.
    #[must_use]
    pub fn words(self) -> String {
        match self {
            Self::PersonalFiles => {
                "This chat reads your personal setup (your global instructions, settings and \
                                    MCP servers); an account added in Settings › Sign-ins doesn't."
                    .to_owned()
            }
            Self::StillOn(b) => format!("Its own {} stays on.", b.words()),
            Self::AsksNot(k) => format!("It doesn't ask before it {}.", k.verb()),
        }
    }
}

impl Builtin {
    const fn words(self) -> &'static str {
        match self {
            Self::Web => "web search",
            Self::Browser => "browser",
            Self::Memory => "memory",
            Self::SubAgents => "sub-agents",
            Self::Skills => "skills",
        }
    }
}

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

    /// How a persona session is set up (#503). Default: nothing set, the brief as the first prompt, every gap.
    fn session(&self, want: &Want) -> Setup {
        Setup::plain(want)
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
        let build = |s: &Spec| -> Arc<dyn Provider> {
            let name = s.base.as_deref().unwrap_or(&s.name);
            match REGISTRY.iter().find(|e| e.name == name) {
                Some(e) => (e.build)(s.clone()),
                None => Arc::new(Plain(s.clone())),
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
