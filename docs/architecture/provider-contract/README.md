# The provider contract (#503)

## Summary

Decision: One contract sets up every persona session. cooee owns the instructions, which built-in tools are on, the
approval rules and how a tool row reads; each provider's module only says how its harness is told, or that it can't be.
Slices: the conformance suite; one session setup per provider; tool rows by kind of action; cooee's approval rules with
Always allow; rules in Settings; limits; switching provider mid-thread; web and browser connectors. A shed change first.
Open questions: four, each with a default: personal files on the default Codex and Gemini login; starter rules; the
nightly run on your plans; web search until you pick a connector.

## Files

| File | What it holds |
|---|---|
| [`adr.md`](adr.md) | The question, what the code and the harnesses force, the options, the decision, what cannot be made the same, the impl issues in order, the open questions, the `arch.toml` lines |
| [`capabilities.md`](capabilities.md) | The capability table, and each harness's exact switches with how each was checked |
| [`callers.md`](callers.md) | How the code that uses the contract reads, written first |
| [`shapes.md`](shapes.md) | The contract's types, actions, rules and `rules.toml`, the chat's rows and cards, shed's new fields, the handoff |
| [`boundaries.md`](boundaries.md) | Which module may call which; the statecharts of a permission request and of a thread's session |
| [`conformance.md`](conformance.md) | The scenario, the fake in CI, the nightly run and its token ceiling |

## The line, at a glance

| The harness's (kept) | cooee's (the same on every harness) |
|---|---|
| Its loop and its system prompt | The brief: character, instructions, notes, skill list |
| File and shell tools, the todo list | `remember`, `history`, `skill`, `ask_persona`, `create_routine`, connectors through the proxy |
| Its sign-in and accounts | The approval rules and the card's three buttons |
| Compaction, models, efforts | How a tool row reads: the kind of action and its subject |
| Its project file in the working folder | The handoff when a thread moves to another provider |

## Where each harness falls short

| | Claude Code | Codex | Gemini CLI |
|---|---|---|---|
| The brief in the system channel | yes | yes, to prove | no: first prompt |
| Personal global files kept out | yes | only on an account cooee added | only on an account cooee added |
| Built-ins off | yes, by allow-list | yes, switch by switch | yes, by allow-list, to prove |
| Asks before everything | yes | no: what stays in its sandbox never asks | to prove |

Nothing here was run against a real harness: Claude's column was read in the installed adapter's source, the other
two in the vendors' docs. Slice 1's nightly run proves or corrects each cell.
