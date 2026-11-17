# The callers, as they will read (#503)

Written before the types. Sketches: signatures and flow, not product code.

## 1. A persona session starts (`cooee-server/src/hub/session.rs`, new)

The one place a provider is asked anything about a session. `Logins::agent` keeps building the account's part.

```rust
let brief = persona.brief(config);                    // once; the same files give the same bytes
let want = Want { brief: &brief, off: &persona.off(&registry), ask_all: true, run: &run_dir };
let setup = provider.session(&want);                  // Plain: nothing set, every gap

let (account, mut launch) = logins.agent(&persona, account, data)?;
launch.apply(&setup)?;                                // meta, env, args; files written 0600 under run_dir
launch.policy = Some("ask".to_owned());               // always: cooee answers, never shed

let before = match setup.brief {
    Channel::System => String::new(),                 // it is already in `launch`
    Channel::FirstPrompt => brief,                    // today's way
};
Recipe { account, launch, instructions: before, gaps: setup.gaps, ..recipe }
```

A plain chat (no persona) skips this: `logins.home(agent, account)` as today.

## 2. A provider says how its harness is told (`cooee-core/src/providers/<name>.rs`)

The only code that names a harness's keys and tools.

```rust
impl Provider for Claude {
    fn session(&self, want: &Want) -> Setup {
        let mut tools = KEPT.to_vec();                             // file, shell, todo
        if !want.off.contains(&Builtin::Web) { tools.extend(WEB) }
        let settings = json!({ "claudeMdExcludes": EXCLUDES, "autoMemoryEnabled": false,
                               "permissions": ask_all(&tools, want.ask_all) });
        Setup {
            meta: Some(json!({
                "systemPrompt": { "append": want.brief, "excludeDynamicSections": true },
                "claudeCode": { "options": { "settingSources": ["project", "local"], "strictMcpConfig": true,
                                             "tools": tools, "settings": settings } } })),
            brief: Channel::System,
            gaps: Vec::new(),
            ..Setup::default()
        }
    }
    // action(): the default reading, plus its MCP tool name `mcp__<server>__<tool>` read as Kind::Tool
}

impl Provider for Codex {
    fn session(&self, want: &Want) -> Setup {
        Setup {
            env: vec![("INITIAL_AGENT_MODE".into(), "read-only".into()), ("CODEX_CONFIG".into(), config(want))],
            brief: Channel::System,
            gaps: vec![Gap::PersonalFiles, Gap::AsksNot(Kind::Read), Gap::AsksNot(Kind::Search),
                       Gap::AsksNot(Kind::Run)],
            ..Setup::default()
        }
    }
}
```

## 3. The harness asks (`cooee-server/src/hub/approvals.rs`, new)

`hub/events.rs` hands every `approval_request` here before the chat sees it.

```rust
pub(super) fn asked(&self, live: &mut Live, seq: u64, req: Request) -> Vec<Change> {
    let action = self.provider_of(&live.meta).action(&req.call);       // kind and subject, neutral from here on
    let rules = Rules::of(self.config_dir(), &live.meta.profile);      // the persona's
    match rules.decide(&action, &live.folder(), live.default_effect()) {
        Verdict { effect: Effect::Allow, rule } => self.answer(live, &req, Answer::Allow, rule),
        Verdict { effect: Effect::Deny, rule } => self.answer(live, &req, Answer::Deny, rule),
        Verdict { effect: Effect::Ask, .. } => {
            let proposed = Rule::proposed(&action, &live.folder());    // what Always allow would save
            vec![Change::Item(live.chat.card(now(), seq, &req, action, proposed.words()))]
        }
    }
}

fn answer(&self, live: &mut Live, req: &Request, a: Answer, rule: Option<Rule>) -> Vec<Change> {
    self.shed.approve(&live.meta.terminal, req.option(a), &req.id)?;   // a one-time option, by request id
    live.chat.ruled(&req.call.id, rule.map(|r| r.words()))             // the tool row says which rule answered
}
```

A plain chat (no persona) has no rules and is not asked for everything: its request becomes today's card, with the
harness's own options, answered through the same `answer`.

## 4. The owner taps the card (`Hub::approve`, `hub/cards.rs` → `approvals.rs`)

```rust
match option {
    "allow" => self.answer(live, &req, Answer::Allow, None),
    "deny" => self.answer(live, &req, Answer::Deny, None),
    "always" => {
        let rule = Rules::add(home, Rule::proposed(&action, &folder).by("card"))?;   // saved first
        self.answer(live, &req, Answer::Allow, Some(rule))
    }
}
```

The next matching request finds the rule in step 3 and shows no card.

## 5. Settings lists, adds and removes rules (`cooee-server/src/routes/rules.rs`, new)

```text
GET    /personas/<id>/rules            → [{ id, effect, kind, pattern, folder, by, at, words }]
POST   /personas/<id>/rules            { effect, kind, pattern, folder? } → the rule, or 400 with what is wrong
DELETE /personas/<id>/rules/<rule id>  → 204; the next matching request is a card again
```

`words` is the sentence the list shows: `Allow running cargo test … in ~/code/cooee`. The app never builds it.

## 6. A cooee tool call passes the MCP proxy (`Hub::grant`, `hub/mcp.rs`)

```rust
let action = Action { kind: Kind::Tool, subject: format!("{server}/{tool}") };
if rules.decide(&action, &folder, Effect::Ask).effect == Effect::Deny {
    return Err(HubError::Refused(format!("{server}/{tool} is denied by your rule")));
}
```

Only deny is enforced here; allow and ask are answered where the harness asks (step 3).

## 7. A client draws a tool row (apple, `CooeeUI`)

```swift
switch item.action {                 // "read" | "edit" | "run" | "search" | "fetch" | "tool" | "other"
case .run: Row(symbol: .terminal, text: item.title)        // title holds the subject: the command
case .read: Row(symbol: .doc, text: item.title)            // the path
// one case per kind; no case names a harness or one of its tools
}
```

## 8. The owner switches provider mid-thread (`Hub::switch`, `hub/relaunch.rs`)

```rust
pub fn switch(&self, id: &str, to: &Recipe /* built by step 1 for the new agent */) -> Result<Item, HubError> {
    let handoff = history::handoff(&self.store, id, 4_000)?;     // no model call
    self.relaunch(id, Place::Agent(&to.agent), to, &handoff)     // as attach and move_to: new terminal, then
}                                                                // the old one closes; a note says where turns run
```

The persona is not saved: its folder, rules, notes, skills, connectors and routines are untouched.

## 9. The conformance suite (`crates/cooee/tests/conformance.rs`, new)

```rust
#[test]
fn the_contract_holds_on_the_fake() { assert_eq!(run(&Fake::server(), SCENARIO).failed(), []); }

#[test] #[ignore = "nightly: a real harness on the owner's plan"]
fn the_contract_holds_on_a_real_harness() {
    let report = run(&Real::server(provider_from_env()), SCENARIO);    // stops at the ceiling
    report.write_summary();                                            // one row per step: pass, fail, gap, closed
    assert_eq!(report.failed(), []);
}
```
