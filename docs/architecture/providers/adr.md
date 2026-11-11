# ADR: providers behind one trait; core never names one (#200)

Files: this ADR, [`shapes.md`](shapes.md) (the types), [`boundaries.md`](boundaries.md) (who calls whom, the
statechart, the CI check), [`callers.md`](callers.md) (how the callers read, written first).

## Question

`crates/cooee-core/src/login.rs` holds Claude, Codex and Gemini in one table (`Provider::defaults`) with Claude's
quirks inline: `claude auth status`'s JSON, `claude -p /usage`'s `N% used · resets …` lines, the removal of
`CLAUDE_CODE_OAUTH_TOKEN` before every command, `CLAUDE_CONFIG_DIR` as the settings variable. `crates/cooee/src/
accounts.rs` keys token files on the name `claude`; `setup.rs`, `shed.rs` (`AGENTS_FIX`) and doc comments name
providers. How do we put each provider in one module behind a common trait (login, accounts, usage/limits, models),
register it in one line, keep `[[providers]]` config working, and keep every caller and client provider-neutral?

## Constraints found in the code

1. `[[providers]]` is data the owner and ~10 integration tests set; setting it replaces the defaults. Tests define
   `fake` providers whose status prints Claude's JSON (`tests/accounts.rs`) or whose usage prints `/usage` lines and
   expects the inherited `CLAUDE_CODE_OAUTH_TOKEN` gone (`tests/accounts_env.rs`). So today every configured
   provider gets Claude's parsers; that is the coupling to cut.
2. `cooee accounts ls --json`'s `usage` (`five_hour`, `seven_day`, `read_at`) is herdsman's contract
   (docs/accounts.md), and `.cooee.json` caches the same shape. It names spans, not a provider.
3. Models are already neutral: an agent's models come from its ACP sessions (`models.rs`); only its effort picks
   (`efforts`, `effort`) are per provider, and they are data.
4. The web client gets `Sheet { provider, accounts, why }` server-side and shows `provider.name`; the API's
   `/logins/{id}` takes the name as an opaque id. The apple app has no provider endpoint yet (only fixtures).

## Options

| | Shape | For | Against |
|---|---|---|---|
| A | Trait objects only; drop `[[providers]]` | Purest | Breaks the owner's config and ~10 tests' fakes; no way to point a provider at another binary |
| B | **Data `Spec` (config, serde) + a behaviour trait with defaults; a registry maps a name to its module** | Config and fakes keep working; quirks move out; adding one is a file and a line | Two names (`Spec`, `Provider`) to learn |
| C | `enum ProviderKind { Claude, Codex, Gemini, Plain }` in core | No `dyn` | Core names every provider: fails the issue's own check |
| D | Usage as `Vec<{label, span, window}>` | Any provider's windows | Breaks herdsman's `five_hour`/`seven_day` contract and the cache for no provider that needs more today |

## Decision

**B, with usage kept as today's span-named shape (not D).**

1. **`Spec`** (today's `Provider` struct renamed, plus one field `base`) is the data: name, ACP agent, login,
   status, logout and usage commands, install line, `home` and `settings` variables, efforts. It stays the
   `[[providers]]` serde type. Neutral.
2. **`trait Provider`** (new `crates/cooee-core/src/provider.rs`) is the behaviour; every method but `spec` has a
   default that is the plain, provider-free reading:
   - login: `read_status(&Ran) -> Status` (default: exit 0 is signed in, the output its line), `scrub()` (variables
     removed from every command it runs; default none);
   - accounts: `token_var()` (the variable a hand-placed token file fills; default none, so no token files);
   - usage/limits: `read_usage(stdout, now) -> Option<Usage>` (default none);
   - models: `spec().efforts` / `effort` (data; no provider needs a model override today, so no method is added).
   The runner (`run`, `check`, `sign_out`, `usage`, `with_home`, `picks`, `missing`) stays neutral in `login.rs`
   as an inherent `impl dyn Provider`, so callers keep writing `p.check(dir)`.
3. **One module per provider** in `crates/cooee-core/src/providers/`: `claude.rs` (its JSON status, `/usage`
   parser with `window`/`civil_days`, `CLAUDE_CODE_OAUTH_TOKEN` as scrub and token variable, its defaults incl.
   `settings`), `codex.rs` (defaults incl. efforts), `gemini.rs` (defaults). Each exports `pub const ENTRY: Entry`.
4. **The registry** is `providers/mod.rs`, one line per provider:
   `registry! { claude, codex, gemini, #[cfg(test)] fake }`. The macro (in `provider.rs`) expands to `mod x;` and
   `REGISTRY: &[Entry]`. Registry order is the default order; the first entry is the default agent (`setup`
   installs it unasked, `cooee doctor`'s fix names it).
5. **Config to behaviour**: each `[[providers]]` `Spec` is built by the entry named by its `base`, else by its
   `name`, else it is `Plain` (spec only, defaults throughout). An unknown `base` is a config error at load. No
   `[[providers]]`: every entry's `defaults()`. Tests' fakes that want Claude's parsers say `base = "claude"`.
6. **Usage common shape**: `Usage { five_hour, seven_day, read_at }` unchanged, moved beside the trait. It names
   spans; each provider fills the spans it reads, `None` the rest. Herdsman's JSON and `.cooee.json` don't change.
7. **API and clients**: a provider is an opaque `id` (its `name`) plus the data to show it (`install`, accounts'
   `Status` with `usage`). Clients render, never branch on, an id. No new endpoint in this issue; the apple app
   keeps none until a later issue needs one, and then it serves `Sheet` as JSON, not a provider-specific shape.
8. **CI check** `scripts/neutral-core.sh` (`just neutral`, a step in `ci.yml`, a line in `.herdsman/rules.md`):
   case-insensitive `claude|anthropic|codex|openai|gemini|hermes` in tracked `crates/*/src/**`, `web/src/**`,
   `web/ts/**` fails, except `crates/cooee-core/src/providers/` and a `.rs` file's lines from its first
   `#[cfg(test)]` on (clippy's `items_after_test_module` keeps test modules last). `ci/neutral-core-test.sh`
   proves it on a fixture tree, like `ci/line-cap-test.sh`.

## Consequences

- Adding a provider: `providers/<name>.rs` (a `Spec` and an `impl Provider`) and its name in `registry!`. The
  test-only `providers/fake.rs` proves it: a unit test lists it in `defaults()`, builds it from a `[[providers]]`
  `Spec`, and reads its status and usage through its overrides.
- Behaviour unchanged for the owner: no `[[providers]]` or one naming `claude`/`codex`/`gemini` gets exactly
  today's commands, parsers and variables.
- Two intended differences, both narrower: (a) a configured provider with an unregistered name and no `base` no
  longer gets Claude's JSON status, `/usage` parsing or token scrub; the test fixtures that relied on it add
  `base = "claude"`; (b) `CLAUDE_CODE_OAUTH_TOKEN` is removed only from Claude's commands, not Codex's or
  Gemini's, which never read it. `setup` asks "also codex? [y/N]" by name (was "Codex").
- Doc comments in core lose their provider examples (`claude-acp` → `<agent>-acp`, `claude-opus-4-5` →
  `acme-large-4-5`); web's doc/CSS comments likewise.
- Public API: `cooee_core::Provider` becomes the trait; the struct is `Spec`; `Provider::defaults()` becomes
  `providers::defaults()`; `AGENTS_FIX` becomes `agents_fix()`. All callers migrate in the same PR; no alias.
- Size: roughly `src/ net +150` (trait, macro, registry, three small modules; the Claude code moves, not grows) plus
  the script and its fixture test.

## Open questions for the owner

1. **Apple Swift in the check?** `apple/Sources/CooeeUI/Fixtures.swift` and `MacSteps.swift` hold sample rows
   ("Claude Code", "Codex", "Hermes") for design frames; renaming them re-records snapshots. Default: the check
   covers Rust and web only; apple sample data is out of scope (it has no provider logic). Say if you want apple in.
2. **`base`**: OK as the one new config key (default above), or would you rather tests' fakes use the plain reading
   only and the Claude-shaped assertions move into `claude.rs` unit tests? The latter drops `base` but rewrites
   the integration tests that drive `/usage` through the real binary.
