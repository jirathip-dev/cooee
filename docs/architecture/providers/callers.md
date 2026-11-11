# The callers, as they will read (#200)

Written before the types: each block is how the code that uses providers reads after the change.

## Adding a provider (AC 3)

```rust
// crates/cooee-core/src/providers/mod.rs: the whole file
crate::registry! { claude, codex, gemini, #[cfg(test)] fake }
```

```rust
// crates/cooee-core/src/providers/fake.rs (test-only; a real one is the same minus the cfg)
use crate::{Entry, Provider, Ran, Spec, Status, Usage, Window};

#[derive(Debug)]
pub struct Fake(Spec);

pub const ENTRY: Entry = Entry { name: "fake", defaults, build: |s| std::sync::Arc::new(Fake(s)) };

fn defaults() -> Spec {
    Spec { name: "fake".into(), agent: "fake-acp".into(), status: vec!["true".into()], ..Spec::default() }
}

impl Provider for Fake {
    fn spec(&self) -> &Spec { &self.0 }
    fn read_usage(&self, out: &str, now: u64) -> Option<Usage> {
        let used_pct = out.trim().parse().ok()?;
        Some(Usage { five_hour: Some(Window { used_pct, resets_at: now + 60 }), seven_day: None, read_at: now })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_new_file_and_one_registry_line_is_a_provider() {
        assert!(crate::providers::defaults().iter().any(|s| s.name == "fake"));
        let p = crate::Providers::new(&[Spec { usage: vec!["echo".into(), "40".into()], ..defaults() }]);
        assert_eq!(p.named("fake").unwrap().usage(None, 0).unwrap().five_hour.unwrap().used_pct, 40);
    }
}
```

## Config and the server

```rust
// cooee-core config.rs
providers: providers::defaults(),                    // Config::default
// Config::load, after parsing: refuse a `base` no entry has
providers::check(&config.providers).map_err(ConfigError::Invalid)?;

// cooee-server lib.rs, at start
let logins = Logins::new(shed, Providers::new(&config.providers), &data);

// cooee-server login.rs: unchanged in shape, typed by the trait
let p = self.providers.named(name).ok_or_else(|| HubError::Refused(format!("no provider {name}")))?;
let mut s = p.check(a.dir.as_deref());
if s.signed_in { s.usage = logins.accounts.usage(p, &a.name, a.dir.as_deref(), at); }
let var = self.providers.for_agent(&prof.agent).map(|p| p.spec().settings.as_str()).filter(|v| !v.is_empty());
Sheet { provider: p.spec().clone(), accounts, why }
```

## The CLI

```rust
// cooee accounts.rs: token files are any provider's that names a token variable (today only one does)
for provider in providers.iter() {
    let spec = provider.spec();
    let mut list = folder_entries(&accounts, provider);
    if let Some(var) = provider.token_var() {
        list.extend(token_entries(&data.join("accounts").join(&spec.name), provider, var));
    }
}
// accounts env for a token file:
json!({ var: token })

// cooee setup.rs: the first registered agent unasked, the rest asked by name
let mut specs = providers::defaults().into_iter();
let first = specs.next();
want.extend(first.filter(|s| !have.contains(&s.agent)).map(|s| s.agent));
for s in specs.filter(|s| !have.contains(&s.agent)) {
    if self.ask(&format!("also {}? [y/N]", s.name), "n").starts_with(['y', 'Y']) { want.push(s.agent); }
}

// cooee main.rs (doctor) and web agents.rs (picker)
println!("warn no agents installed in shed\n     fix: {}", cooee_core::agents_fix());
```

## The web client

```rust
// web compose.rs: unchanged; it reads Spec and Usage only
pub fn named(agent: &str, providers: &[Sheet]) -> String { /* s.provider.name, else agent minus `-acp` */ }
fn usage_line(u: Usage) -> String { /* "5h {} · 7d {} · read {}" */ }
```

## Tests that drive the binary

```toml
# a fake that prints Claude-shaped status or usage says so (tests may name providers)
[[providers]]
name = "fake"
base = "claude"
agent = "fake-acp"
home = "FAKE_HOME"
status = ["sh", "-c", "cat \"$FAKE_HOME/status.json\""]
usage = ["sh", "-c", "cat \"$FAKE_HOME/usage.txt\""]
```
