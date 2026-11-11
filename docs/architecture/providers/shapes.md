# Data shapes (#200)

A sketch: signatures and fields, not product code.

```rust

// ---- crates/cooee-core/src/login.rs (neutral) ----------------------------------------------------------------

/// A provider as data: what `[[providers]]` sets and each module's `defaults()` returns. Today's `Provider` struct,
/// renamed; every field as today plus `base`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub name: String,          // opaque id: sheet, login terminal, accounts folder, API `{id}`
    pub agent: String,         // its ACP agent id
    pub login: Vec<String>,
    pub status: Vec<String>,
    #[serde(default)] pub install: String,
    #[serde(default)] pub home: String,      // config-folder variable; empty: system default account only
    #[serde(default)] pub logout: Vec<String>,
    #[serde(default)] pub settings: String,  // enforced-settings variable for agent profiles
    #[serde(default)] pub usage: Vec<String>,
    #[serde(default)] pub efforts: Vec<String>,
    #[serde(default)] pub effort: Vec<String>,
    /// NEW. The registered provider whose behaviour this spec gets (default: the one named `name`, else none).
    #[serde(default)] pub base: Option<String>,
}

/// One command's run: exit 0, stdout, stderr. `None` from the runner when it timed out (as today).
pub struct Ran { pub ok: bool, pub stdout: String, pub stderr: String }

/// `Status` unchanged (signed_in, unknown, line, install, who, usage).

// ---- crates/cooee-core/src/provider.rs (neutral, new) --------------------------------------------------------

/// A provider's behaviour. Every method but `spec` defaults to the plain reading.
pub trait Provider: Send + Sync + fmt::Debug {
    fn spec(&self) -> &Spec;
    /// login: signed-in state from the status command's run. Default: `ok`, the output (stdout, else stderr) on
    /// one line, redacted.
    fn read_status(&self, ran: &Ran) -> Status { plain_status(ran) }
    /// login: variables removed from every command this provider runs.
    fn scrub(&self) -> &'static [&'static str] { &[] }
    /// accounts: the variable a hand-placed `<data>/accounts/<name>/<account>.token` fills; none: no token files.
    fn token_var(&self) -> Option<&'static str> { None }
    /// usage/limits: the usage command's stdout read at `now` (s); none: no window read.
    fn read_usage(&self, _stdout: &str, _now: u64) -> Option<Usage> { None }
}

/// The neutral runner, callers unchanged: `p.check(home)`, `p.sign_out(home)`, `p.usage(home, now)`,
/// `p.with_home(cmd, home)`, `p.env(home)`, `p.picks(effort)`, `p.missing()`.
impl dyn Provider { /* today's Provider methods, reading self.spec() and calling the hooks above */ }

/// A spec with no registered behaviour.
#[derive(Debug)]
pub struct Plain(pub Spec);
impl Provider for Plain { fn spec(&self) -> &Spec { &self.0 } }

/// One registry line's worth: a module's name, its defaults and its constructor.
pub struct Entry {
    pub name: &'static str,
    pub defaults: fn() -> Spec,
    pub build: fn(Spec) -> Arc<dyn Provider>,
}

/// `registry! { a, b, #[cfg(test)] c }` → `mod a; mod b; #[cfg(test)] mod c;` and
/// `pub(crate) const REGISTRY: &[Entry] = &[a::ENTRY, b::ENTRY, #[cfg(test)] c::ENTRY];`
#[macro_export] macro_rules! registry { /* … */ }

/// The built providers, in config (else registry) order.
#[derive(Debug, Clone)]
pub struct Providers(Vec<Arc<dyn Provider>>);
impl Providers {
    /// Each spec built by its `base`, else its `name`, else `Plain`. Infallible: `Config::load` already refused an
    /// unknown `base` (ConfigError::Invalid "providers: no provider <base> to base <name> on").
    pub fn new(specs: &[Spec]) -> Self;
    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Provider>>;
    pub fn named(&self, name: &str) -> Option<&Arc<dyn Provider>>;
    pub fn for_agent(&self, agent: &str) -> Option<&Arc<dyn Provider>>;
}

/// Refuses a spec whose `base` names no registered provider (Config::load calls it).
pub fn check(specs: &[Spec]) -> Result<(), String>;
/// Every registered provider's defaults, in registry order (`Config::default().providers`).
pub fn defaults() -> Vec<Spec>;
/// The fix `cooee doctor` and the agent picker print when shed has no agent: `shed agents add <first agent>`.
pub fn agents_fix() -> String;

// ---- crates/cooee-core/src/accounts.rs (moved beside the trait only if it reads better; shape unchanged) ------

/// Usage/limits, the common shape each provider fills: the spans herdsman schedules on (docs/accounts.md).
pub struct Usage { pub five_hour: Option<Window>, pub seven_day: Option<Window>, pub read_at: u64 }
pub struct Window { pub used_pct: u32, pub resets_at: u64 }

// ---- crates/cooee-core/src/providers/<name>.rs (one per provider; exempt from the neutral check) ------------

#[derive(Debug)]
pub struct Claude(Spec);
pub const ENTRY: Entry = Entry { name: "claude", defaults, build: |s| Arc::new(Claude(s)) };
fn defaults() -> Spec { /* today's list[0], incl. usage and settings */ }
impl Provider for Claude {
    fn spec(&self) -> &Spec { &self.0 }
    fn read_status(&self, ran: &Ran) -> Status { /* `auth status` JSON: loggedIn, authMethod, email|orgName */ }
    fn scrub(&self) -> &'static [&'static str] { &["CLAUDE_CODE_OAUTH_TOKEN"] }
    fn token_var(&self) -> Option<&'static str> { Some("CLAUDE_CODE_OAUTH_TOKEN") }
    fn read_usage(&self, out: &str, now: u64) -> Option<Usage> { /* `Current session` / `Current week (all models)` */ }
}
// codex.rs, gemini.rs: `defaults()` and `spec()` only. fake.rs (#[cfg(test)]): overrides read_status/read_usage.
```
