# Personas answer in chat with Blocks (#776)

## Summary

Decision: a persona asks for blocks by calling one new tool of cooee's, and its text replies stay as they are. cooee ships two catalogs, one that only shows and one that may also ask, and the owner picks one per persona on the persona screen (none: text only, as today). Each reply is checked against the catalog before it is stored; a refused one goes back to the persona once, then shows as plain text. A reply takes one answer, checked against what the persona wrote, and the persona gets it as its next turn.
Slices: six: (1) the blocks pin; (2) cooee's own tools and their routes into folders, no behaviour change; (3) the catalogs, the persona's pick and the prompt; (4) the tool: check, one retry, plain text, drawn in chat; (5) answers back as turns; (6) the eval recordings and their PR on blocks.
Open questions: (1) The iPhone and Mac apps draw these replies only once #620 and #716 land, and then with no app code; until then the app shows the turn's text only. Wait (my pick), or build a stopgap in the app that #716 deletes? (2) Record the evals on the default model of each of cooee's three providers, at most 72 short turns, once (my pick), or name other models? (3) Two catalogs as above (my pick), or other levels?

## Read in this order

1. [`adr.md`](adr.md): the question, what the code says, the options, the fifteen decisions, the consequences, the
   hot files, the slices, the `.drover/arch.toml` lines.
2. [`callers.md`](callers.md): the owner's key, the prompt, a `show` call and its four answers, the hub's loop, the
   chat's arm, an answer coming back, the tests, the eval run. Written first.
3. [`shapes.md`](shapes.md): the two catalogs, the key, the tool, the types, the check of an answer, the wire, the
   tool folder, the eval command.
4. [`boundaries.md`](boundaries.md): which module may call which, what holds each rule, the statecharts of a turn's
   tries and of one reply.

## Where the issue's points are answered

| The issue asks | ADR decision |
|---|---|
| 1. A catalog for each persona, read with `Catalog::read` | 2, 3; `shapes.md` § The catalogs |
| 2. The session prompt includes `model::prompt` | 4 |
| 3. Every reply goes through `model::read` before it is drawn | 5, 8; `boundaries.md`, the first rule |
| 4. One retry, then plain text; cooee owns the model call | 1, 6, 7; `boundaries.md` statechart 1 |
| 5. Actions go back to the persona, after `own`, `sends` and cooee's check | 9 to 12; `shapes.md` § The check of an answer |
| 6. Eval replies recorded once, as a PR on blocks | 14; `shapes.md` § The eval command |
| How the persona asks: a tool call or a JSON-only turn | 1; options 1 |
| Where the catalog is stored and edited | 2, 3; options 2 |
| No catalog: as today. Plugin screens, cards, screenshots unchanged | 3; hot files (`blocks.rs` untouched) |

## What was checked, and what was not

- Read in cooee at this branch's base (877cd75c8): `agents.rs`, `chat/` (items, the log, a turn's end), `blocks.rs`,
  `hub/` (`send`, `ask`, `handoff`, `cards`, `session`, `chief`, the token), `mcp/own.rs`, `routes/mcp.rs`,
  `routes/chief.rs`, `screens/chat.rs`, `screens/persona.rs`, `turns.rs`, `scripts/gen.sh` and `gen.d/`,
  `.drover/arch/`, the test support's fake agent, the app's `Item.swift` and `Blocks.swift`, `drover cap cooee`.
- Read in blocks: the model-written ADR at 68fc638b, and at staging cf626c6 the merged `catalog/mod.rs`,
  `model/mod.rs`, `model/prompt.rs`, `docs/model.md` and `blocks-cli`'s `eval`. The names match the ADR.
- Not run: cooee was not built against the new pin, which is 30 commits ahead of 3fe5208. No model was called.
  The catalogs' field lists and prompt sizes are not written yet. Which approval rule answers a call to one of
  cooee's own tools was not traced. That each harness returns a failed tool result to its model within the turn
  is assumed from MCP, not tested.
