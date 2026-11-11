# Module boundaries, statechart and the neutral-core check (#200)

## Who may call whom

```mermaid
flowchart LR
  subgraph core[cooee-core, neutral]
    config[config.rs] --> provider[provider.rs: trait, Entry, Providers, registry!]
    login[login.rs: Spec, runner, Status, redact] --> provider
    accounts[accounts.rs: folders, Usage cache]
    shed[shed.rs]
  end
  subgraph named[cooee-core/src/providers/, may name providers]
    reg[mod.rs: registry! line] --> claude[claude.rs] & codex[codex.rs] & gemini[gemini.rs] & fake["fake.rs (test)"]
  end
  provider -- REGISTRY only --> reg
  claude & codex & gemini & fake -- impl Provider, Spec, Usage --> provider
  server[cooee-server login.rs: Logins] --> provider & accounts
  cli[cooee accounts.rs, setup.rs, main.rs] --> provider & accounts
  web[web compose.rs] -- Sheet, Spec, Status, Usage --> login
```

Rules:

1. Only `provider.rs` reads `providers::REGISTRY`, and only through `Providers::new`, `defaults()`, `agents_fix()`.
   `providers::*` modules are `pub(crate)`; nothing outside `cooee-core` can name `providers::claude`.
2. A provider module may use `provider.rs`, `login.rs` (`Spec`, `Ran`, `Status`, `one_line`) and `Usage`/`Window`;
   never `accounts.rs`, `shed.rs`, the store or another provider module.
3. `cooee-server`, `cooee` and `web` hold `Providers` / `Arc<dyn Provider>` / `Sheet` and call trait methods; they
   never compare a provider's `name` to a literal. (The neutral check makes such a literal fail CI.)
4. `web` never sees the trait: `Sheet.provider` stays a `Spec` clone; it shows `name`, `install` and `Status`.
5. Clients (web JS/TS, apple) get a provider as an opaque `id` and render it; no client branches on it.

## Statechart: a provider from config to a reading

Provider resolution happens once, at config load. The per-account check is today's (`Check` in
`cooee-server/src/login.rs`), shown only to place the hooks.

```mermaid
stateDiagram-v2
  [*] --> Spec: [[providers]] entry, or registry defaults()
  Spec --> Refused: base names no Entry (ConfigError at load)
  Spec --> Registered: Entry named base, else name
  Spec --> Plain: no Entry, no base
  Registered --> Built
  Plain --> Built
  state Built {
    [*] --> Unchecked
    Unchecked --> Running: tick due (start, 10 min, sheet opened, after login/sign-out)
    Running --> Unknown: status command timed out
    Running --> Known: run(status, scrub()) then read_status(Ran)
    Known --> Reading: signed_in and spec.usage non-empty and cache older than 5 min
    Reading --> Known: run(usage, scrub()) then read_usage(stdout, now), cached in .cooee.json (or last kept)
    Known --> Running: next tick due
    Unknown --> Running: next tick due (backed off)
  }
```

## The neutral-core check

`scripts/neutral-core.sh`, run by `just neutral` (in `just check`) and a `ci.yml` step after `hygiene`:

- Files: `git ls-files 'crates/*/src/**' 'web/src/**' 'web/ts/**'` minus `crates/cooee-core/src/providers/**`.
- In a `.rs` file, lines from its first `#[cfg(test)]` on are skipped.
- Fails, printing `file:line`, on `grep -inE 'claude|anthropic|codex|openai|gemini|hermes'`; else prints
  `core names no provider`.
- `ci/neutral-core-test.sh` (run by `just neutral` too) builds a fixture tree: a core file naming one fails; the same
  word under `providers/`, after `#[cfg(test)]`, or under `crates/x/tests/` passes.
- `.herdsman/rules.md` gains: "Don't name a provider (claude, anthropic, codex, openai, gemini, hermes) outside
  crates/cooee-core/src/providers/ and tests — scripts/neutral-core.sh".
