# ADR: the provider contract: one behaviour on every harness (#503)

## Summary

Decision: One contract sets up every persona session. cooee owns the instructions, which built-in tools are on, the
approval rules and how a tool row reads; each provider's module only says how its harness is told, or that it can't be.
Slices: the conformance suite; one session setup per provider; tool rows by kind of action; cooee's approval rules with
Always allow; rules in Settings; limits; switching provider mid-thread; web and browser connectors. A shed change first.
Open questions: four, each with a default: personal files on the default Codex and Gemini login; starter rules; the
nightly run on your plans; web search until you pick a connector.

Amended (#783): Antigravity (`antigravity.rs`, agent `antigravity-acp`) replaces Gemini, which is dropped; its
switches and gaps are in [`capabilities.md`](capabilities.md).

Files: this ADR, [`capabilities.md`](capabilities.md) (the table and each harness's switches),
[`callers.md`](callers.md) (how the code reads, written first), [`shapes.md`](shapes.md) (the types and files),
[`boundaries.md`](boundaries.md) (who calls whom, the statecharts), [`conformance.md`](conformance.md) (the suite).

## Question

A persona should behave the same on Claude Code, Codex and Gemini, each run through its vendor's own harness. Today
cooee's side is already neutral in places (the brief, cooee's MCP tools, the MCP proxy, `scripts/neutral-core.sh`), but
the session itself is set up per provider by accident: one hook, `Provider::no_personal_setup`, which only Claude
fills. Where does the line between harness and cooee run, how is each harness told, and what do we do where a harness
can't be told?

## What the code and the harnesses force

1. **cooee never speaks ACP.** shed does (`shed run --acp`). cooee can pass a session four things: the adapter's
   environment and arguments, `--meta` (sent as ACP `_meta`), `--mcp` and `--policy` (`shed.rs`, `Launch`). Every
   switch in this ADR is one of those four, or a file one of them names.
2. **shed's events are too thin for rules.** `approval_request` carries a title and the harness's options, no request
   id, no kind, no command or path; `approve` answers "the oldest request". `tool_call` carries a title and ACP's
   kind. Rules and rows need the facts ACP already sends (`kind`, `locations`, `rawInput`), so shed must pass them on.
3. **The brief is prepended to the first message** (`Hub::connect`, `before`). It is rendered once, which is right.
   A harness's compaction may summarise it away, and it sits outside the cached system prompt.
4. **Only Claude is isolated** (#416): `settingSources`, `strictMcpConfig`, `claudeMdExcludes` in `_meta`.
   `codex.rs` and `gemini.rs` build `Plain`: a persona there reads the owner's global files.
5. **Approvals are the harness's.** The card shows the harness's own options; `policy` (`ask`, `allow`, `deny`) is
   shed's blanket answer. A finding, read in the adapter's source and not run: with no default mode set,
   claude-agent-acp 0.101.0 starts a session in `auto` when the account offers it (`session-mode.js`), so a Claude
   persona may run today under the vendor's own approval classifier, not the owner's taps.
6. **A relaunch already hands over** (`hub/relaunch.rs`): a new terminal gets the brief and the last six turns as
   `handover`, sent with the next message. A provider switch is the same move with another agent.
7. **CI has no shed and no harness.** Integration tests run on `FakeShed` and its scripted agent
   (`crates/cooee/tests/support/`). Real harnesses exist only on the owner's Macs.
8. **What was checked, and how.** Claude's switches were read in the adapter and SDK installed on this Mac
   (claude-agent-acp 0.101.0, Agent SDK 0.3.318). Codex's and Gemini's come from the vendors' docs, read 2026-11-17;
   neither CLI is on this Mac's PATH. Nothing was run against a real harness in this lane: no tokens were spent.
   [`capabilities.md`](capabilities.md) marks every cell S (source), D (docs) or P (to prove), and slice 1 proves them.

## Options

| | Shape | For | Against |
|---|---|---|---|
| A | Keep patching per provider: each feature asks "which harness?" | No new types | Features × harnesses branches; nothing says what is the same; today's state |
| B | **One contract: cooee states what it wants (`Want`), each provider module compiles it into its harness's switches (`Setup`) and names what it couldn't do (`Gap`); rules, rows and handoff work on neutral facts in cooee** | One place per harness; gaps are data, shown and tested; adding a harness is one file | Needs shed to pass ACP's facts on; the table must be kept true by a nightly run |
| C | cooee sits between shed and the adapter and offers ACP's file and terminal methods itself | Total control of file and shell | A harness of our own under another name (dropped by the owner); ACP v2 removes those methods |
| D | Rules written into each harness's own list (q713) | No engine in cooee | Reversed by the owner 2026-11-17: three syntaxes and precedences, and no list at all on one harness |

## Decision

**B.** The harness is an engine: its loop, its file and shell tools, its todo list, its sign-in, its compaction.
Everything else is cooee's. A plain chat with no persona stays outside the contract: it is the harness as the owner
set it up (q1700), with the harness's own approval options on its cards; only its tool rows change.

1. **One setup call.** `Provider::session(&Want) -> Setup` replaces `no_personal_setup`. `Want` is the same for every
   provider: the brief, the built-ins to turn off, ask for everything, a folder for files. `Setup` is only what
   `Launch` already carries (meta, env, args, files), where the brief goes (`Channel`), and the `Gap`s. `Plain` and any
   provider that doesn't override it return every gap. `hub/session.rs` is its only caller.
2. **Instructions.** No persona session reads the owner's global instruction files, settings, plugins or MCP servers
   where the harness has a switch; where it has none, the gap is `PersonalFiles` (question 1). The brief is rendered
   once per session from the persona's files, with no clock, thread id or random order in it, and never again for that
   session: a note kept mid-session reaches the next one. It goes in the harness's system channel where one exists
   that keeps the vendor's prompt (Claude: appended, with the vendor's dynamic sections moved out so the prefix is
   static; Codex: developer instructions), else as the first prompt (Gemini; today's way for all). shed replays the
   same `_meta` bytes on every wake, so the cached prefix holds.
3. **Tools.** File, shell and todo tools stay the harness's. Memory, sub-agents, harness skills and any browser are
   off for every persona, replaced by `remember`/`history`, `ask_persona`, `skill` and a connector. Web stays the
   harness's until the persona picks a registry server marked `provides = ["web"]`; then it is off (question 4). A
   harness with an allow-list of built-ins (Claude, Gemini) gets the list, so a tool its vendor adds later is off
   until the contract names it; Codex has per-feature switches only.
4. **Approvals: cooee holds the rules** (owner, 2026-11-17, reverses q713). Every persona session starts with shed's
   `--policy ask` and the harness's ask-for-everything switches. `hub/approvals.rs` reads each request as an `Action`
   (kind and subject), asks the persona's `Rules` (deny > ask > allow, scoped to persona and folder), and answers the
   harness with a one-time allow or reject; only an unmatched action becomes a card. The card has cooee's three
   buttons, the same on every harness: **Allow**, **Always allow** (saves the rule it shows in plain words), **Deny**.
   cooee never sends a harness's own "always" option, so no harness keeps a list. `policy` in `agent.toml` stays as the
   answer to an unmatched action (`ask`: a card). Rules live in `<config>/agents/<id>/rules.toml`; a persona with no
   file has the starter rules (question 2). A deny rule on an MCP tool is also enforced in cooee's MCP proxy, so it
   holds on a harness that never asks. Ask-for-everything is reachable on Claude, not on Codex (what stays inside its
   read-only sandbox never asks), unproven on Gemini: [`capabilities.md`](capabilities.md) says what is lost.
5. **Tool rows.** A row is `action` (read, edit, run, search, fetch, tool, other) and a subject (the path, command,
   query, URL or `server/tool`), read from ACP's facts by the provider's `action()`; its neutral default reads ACP's
   own fields. No client draws a harness's tool name. The wire keeps the field `title` (it now holds the subject) and
   adds `action`, so an older app still draws.
6. **Switching provider mid-thread** is a relaunch with another agent: a new session set up by the new provider's
   `session()`, the brief through its channel, and a handoff that cooee writes without a model call (the first owner
   message, the last turns up to 4,000 characters, and a line pointing at the `history` tool for the rest), so it
   works when the old provider is capped. The thread gets one line saying where later turns run. #76 builds its chain
   and its limit card on this; #221 changes where the turns are read from, not the shape.
7. **Limits.** shed's `rate-limited` state and its reset note become one neutral turn state; context use comes from
   ACP's `usage_update`; plan windows stay `Usage` and are `None` where a provider has no command for them.
8. **The conformance suite** is one scenario as data, driven through `cooee serve`'s HTTP API: on `FakeShed` in CI for
   every PR, and nightly against each real harness on the owner's Mac with a stated ceiling
   ([`conformance.md`](conformance.md)). A declared gap reports `gap`, not a failure; a gap that passes reports
   `closed`, so the table is corrected.

## What cannot be made the same

Written down so nobody chases it:

1. **The harness's system prompt.** The vendor's; cooee adds to it and never replaces it (that is a harness of our own).
2. **Compaction.** When and how each harness summarises. The brief survives where it is in the system channel; as a
   first prompt (Gemini) it may be summarised on a long thread.
3. **The model's habits:** tone, which tool it reaches for, how readily it plans or asks.
4. **The file and shell tools' own shapes:** edit formats, output limits, Codex's OS sandbox around every command.
5. **What asks.** Codex never asks for what stays inside its sandbox; rules on those actions do nothing there.
6. **The project instruction file.** Each harness reads its own name from the working folder (`CLAUDE.md`,
   `AGENTS.md`, `GEMINI.md`). cooee does not write into the owner's folders to even that out.
7. **Web search results,** until the persona has a web connector.
8. **Plan usage windows:** only where the provider has a command for them (Claude today).
9. **Waking and resuming:** how long a hibernated session takes to come back, and whether a harness can load one.

## Consequences

- `no_personal_setup` goes; its callers move to `session()` in the same PR. `Launch.policy` is always `ask` for a
  persona; `Body::Approval.options` are cooee's three for a harness request; `Body::Tool` gains `action`.
- Changing a persona's agent changes only `agent` (and clears `model`, which names one provider's model) in
  `agent.toml`. Skills, notes, rules, connectors and routines are keyed by the persona's folder and never by a
  provider; a test saves a persona with another agent and compares its folder.
- A Claude persona stops running in `auto`; without rules it would ask for every read, hence the starter rules.
- More requests cross shed: one round trip on its socket per tool call. Background cards multiply until #505
  (expiry as Deny) is in, so #505 lands before slice 4.
- cooee pins a new shed rev (slice 0); an older shed is refused at `hello` as today.
- Stored threads need no migration: an old tool row reads as `action: other` with its title as the subject.
- #72 scopes 1 and 2 and its acceptance 3 are superseded by slices 4 and 5; #76's handoff is slice 7.

## The impl issues, in order

| # | Issue | Needs | Notes |
|---|---|---|---|
| 0 | shed: `approval_request` carries its id and ACP's tool call, `approve` names the request; `tool_call` carries `locations` and `raw_input` | | shed's repo; cooee bumps the `shed-proto` rev |
| 1 | conformance: the scenario, `FakeShed` in CI, the nightly workflow and its report (steps 1, 2, 3, 6 first) | | proves the P cells; later slices add their steps |
| 2 | contract: `Provider::session` (instructions, brief channel, built-ins off) for Claude, Codex and Gemini; gaps; step 8 | 1 | deletes `no_personal_setup` |
| 3 | tool rows by action: `action()` and the row; the apple rows | 0 | `--design` for the rows |
| 4 | rules: `rules.rs`, `hub/approvals.rs`, ask for everything, the three-button card, deny at the proxy, the rules API; steps 4, 5; the agent-change test | 0, 2, #505 | supersedes #72 scopes 1–2 |
| 5 | rules in Settings: listed in plain words, removed, added; a provider's gaps in the agent picker | 4 | `--design`; supersedes #72 acceptance 3 |
| 6 | limits: the neutral limit state and context use; step 7 | 2 | Gemini's limit line is a data entry in shed |
| 7 | switch provider mid-thread: `Hub::switch`, `history::handoff`, the header action | 2 | #76 is amended to build on it |
| 8 | web and browser connectors: `provides` on a registry server turns the harness's own off | 2 | waits for question 4 |

3 and 4 both edit `chat.rs` and `hub/events.rs`: queue them `--serial`. 6, 7 and 8 are independent after 2.

## Open questions for the owner

1. **Personal files on the default Codex and Gemini login.** Neither harness has a switch to skip its global
   instruction and settings files; an account cooee adds (Settings › Sign-ins) has a clean folder. Default: a persona
   chat on the default login of those two starts, with one line in the thread saying it reads your personal setup
   and that an added account doesn't. Alternative: refuse it.
2. **Starter rules.** Default: a persona with no rules file has two listed, removable rules: reading and searching
   inside its own folder are allowed. Without them every file read asks on your phone.
3. **The nightly run** uses your plans on the Mac runner: at most 10 prompts, 10 minutes and 40,000 context tokens per
   provider per night, on the smallest model each offers. Default: Claude only at first. `codex` and `gemini` are not
   on this Mac's PATH and shed has no Gemini agent; installing and signing them in is a host change that is yours.
4. **Web search** stays each harness's own, so results differ, until you name a web connector. None is chosen here:
   one costs money or adds a dependency. Default: leave it.

## `.drover/arch.toml`

This ADR converts no hot file. Three choices keep its slices off shared files: the whole `Want`/`Setup` shape lands in
slice 2, so later slices fill provider files and never the trait; rules get their own route area
(`routes/rules.rs`, a collected folder) and module; the nightly is its own workflow file
(`.github/workflows/conformance.yml`), not a job in `ci.yml`. Slice 4 adds one ban, in the PR that adds
`hub/approvals.rs`:

```toml
[[ban]]
pattern = "shed.approve"
paths = ["crates/cooee-server/src/**"]
except = ["crates/cooee-server/src/hub/approvals.rs"]
reason = "#503: a harness's permission request is answered only where cooee's rules decided: hub/approvals.rs"
```
