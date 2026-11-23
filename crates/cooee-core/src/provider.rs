//! A provider's behaviour behind one trait, each provider one module in `providers/` and one line in its registry.

use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::Deserialize;
use serde_json::Value as Json;
use toml::{Table, Value};

use shed_catalog::Catalog;

use crate::{
    Calls as _, Shed, Spec,
    action::{Action, Call, Kind},
    providers::REGISTRY,
    shed_proto,
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
    pub meta: Option<Json>,
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

    /// How a persona session is set up (#503). Default: nothing set, the brief as the first prompt, every gap.
    fn session(&self, want: &Want) -> Setup {
        Setup::plain(want)
    }

    /// A tool call's kind and subject from ACP's facts; the default reads ACP's own fields only.
    fn action(&self, call: &Call) -> Action {
        Action::plain(call)
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
    /// Its spec where `[[providers]]` sets no field.
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

/// The built providers, in config (else registry) order, and shed's records of their agents.
#[derive(Debug, Clone, Default)]
pub struct Providers {
    built: Vec<Arc<dyn Provider>>,
    catalog: Catalog,
}

impl Providers {
    /// Each spec built by the entry its `base`, else its `name`, names; else [`Plain`]; over the built-in records.
    #[must_use]
    pub fn new(specs: &[Spec]) -> Self {
        Self::with(specs, Catalog::built_in())
    }

    fn with(specs: &[Spec], catalog: Catalog) -> Self {
        let build = |s: &Spec| -> Arc<dyn Provider> {
            let name = s.base.as_deref().unwrap_or(&s.name);
            match REGISTRY.iter().find(|e| e.name == name) {
                Some(e) => (e.build)(s.clone()),
                None => Arc::new(Plain(s.clone())),
            }
        };
        Self {
            built: specs.iter().map(build).collect(),
            catalog,
        }
    }

    /// Every provider, in order.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Provider>> {
        self.built.iter()
    }

    /// Every provider over shed's records (the built-in ones when shed can't be asked), by `rows` (see [`specs`]).
    #[must_use]
    pub fn load(rows: Option<&[Table]>, shed: &Shed) -> Self {
        let catalog = shed.agents().map_or_else(|_| Catalog::built_in(), |a| Catalog::served(&a));
        Self::with(&specs(rows, &catalog).unwrap_or_default(), catalog)
    }

    /// The provider named `name`.
    #[must_use]
    pub fn named(&self, name: &str) -> Option<&Arc<dyn Provider>> {
        self.iter().find(|p| p.spec().name == name)
    }

    /// A tool call of ACP agent `agent` as its provider reads it; an agent no provider runs: ACP's own fields.
    #[must_use]
    pub fn action(&self, agent: &str, call: &Call) -> Action {
        self.for_agent(agent)
            .map_or_else(|| Action::plain(call), |p| p.action(call))
    }

    /// The provider that signs in ACP agent `agent`.
    #[must_use]
    pub fn for_agent(&self, agent: &str) -> Option<&Arc<dyn Provider>> {
        self.iter().find(|p| p.spec().agent == agent)
    }

    /// shed's record of ACP agent `agent`: its login, status, install and efforts.
    #[must_use]
    pub fn record(&self, agent: &str) -> Option<&shed_proto::Provider> {
        self.catalog.by_id(agent)
    }
}

/// The providers: each registered one's defaults, and one named for its program for each record of `catalog` that
/// logs in and has no provider; with `[[providers]]` set, its rows instead, each over the provider of its name.
///
/// # Errors
/// A row that names no registered `base`, or that leaves a spec incomplete or with a field it doesn't know.
pub fn specs(rows: Option<&[Table]>, catalog: &Catalog) -> Result<Vec<Spec>, String> {
    let mut known: Vec<Table> = REGISTRY
        .iter()
        .filter_map(|e| Table::try_from((e.defaults)()).ok())
        .collect();
    let field = |t: &Table, k: &str| t.get(k).and_then(Value::as_str).map(str::to_owned);
    for r in catalog.iter().map(|e| &e.provider) {
        let known_agent = known.iter().any(|t| field(t, "agent").as_ref() == Some(&r.id));
        if !known_agent && r.login.is_some() && r.status.is_some() {
            let name = if r.agent.is_empty() { &r.id } else { &r.agent };
            let mut t = Table::new();
            t.insert("name".into(), name.as_str().into());
            t.insert("agent".into(), r.id.as_str().into());
            known.push(t);
        }
    }
    let Some(rows) = rows else {
        return finish(known);
    };
    let over = |row: &Table| {
        let name = field(row, "name");
        let base = known
            .iter()
            .find(|t| name.is_some() && field(t, "name") == name);
        let mut t = base.cloned().unwrap_or_default();
        t.extend(row.clone());
        t
    };
    finish(rows.iter().map(over).collect())
}

const RETIRED: [&str; 8] = [
    "home", "logout", "usage", "login", "status", "install", "efforts", "effort",
];

fn finish(picked: Vec<Table>) -> Result<Vec<Spec>, String> {
    let specs = picked
        .into_iter()
        .map(|mut t| {
            t.retain(|k, _| !RETIRED.contains(&k.as_ref()));
            Spec::deserialize(t).map_err(|e| format!("providers: {}", e.message()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let unknown = |b: &&String| !REGISTRY.iter().any(|e| e.name == b.as_str());
    match specs
        .iter()
        .find_map(|s| Some((s, s.base.as_ref().filter(unknown)?)))
    {
        Some((s, base)) => Err(format!(
            "providers: no provider {base} to base {} on",
            s.name
        )),
        None => Ok(specs),
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
