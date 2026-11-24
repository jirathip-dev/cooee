# ADR: personas answer in chat with Blocks (#776)

## Summary

Decision: a persona asks for blocks by calling one new tool of cooee's, and its text replies stay as they are. cooee ships two catalogs, one that only shows and one that may also ask, and the owner picks one per persona on the persona screen (none: text only, as today). Each reply is checked against the catalog before it is stored; a refused one goes back to the persona once, then shows as plain text. A reply takes one answer, checked against what the persona wrote, and the persona gets it as its next turn.
Slices: six: (1) the blocks pin; (2) cooee's own tools and their routes into folders, no behaviour change; (3) the catalogs, the persona's pick and the prompt; (4) the tool: check, one retry, plain text, drawn in chat; (5) answers back as turns; (6) the eval recordings and their PR on blocks.
Open questions: (1) The iPhone and Mac apps draw these replies only once #620 and #716 land, and then with no app code; until then the app shows the turn's text only. Wait (my pick), or build a stopgap in the app that #716 deletes? (2) Record the evals on the default model of each of cooee's three providers, at most 72 short turns, once (my pick), or name other models? (3) Two catalogs as above (my pick), or other levels?

Files: this ADR, [`callers.md`](callers.md) (how the callers read, written first), [`shapes.md`](shapes.md) (the
catalogs, the tool, the types, the check of an answer), [`boundaries.md`](boundaries.md) (who calls whom, what
holds each rule, the two statecharts).

## Question

A persona is a harness session that cooee prompts through shed, not a model cooee calls. How does it answer with
blocks that are checked before anyone sees them, where does its catalog live, what happens to a refused reply, and
how does a tap on its blocks get back to it?

## What the code says (cooee 877cd75c8, blocks cf626c6)

1. **cooee calls no model.** It sends a prompt to an ACP session and reads events: text, tool calls, the reply
   (`chat/apply.rs`). The provider, the model, its limits and its network retries are the harness's and shed's.
2. **The session prompt is `Agent::brief`:** Character, Instructions, notes, skills. It goes in the harness's
   system channel or ahead of the first message, once per session (`hub/session.rs`).
3. **A persona already has tools of cooee's.** `cooee mcp serve --thread <id>` lists them and posts each call to
   `/mcp/agent/…` with the thread's token (`mcp/own.rs`, `routes/mcp.rs`). Chief's are asked from the server.
4. **Much reads a turn's reply as text:** push, history search, `ask_persona`'s return, a heartbeat's last line,
   a call's spoken reply, the streaming `prose`.
5. **A chat item is a `Body` variant, stored as JSON.** The chat screen builds a turn as one `message`
   (`screens/chat.rs`); the JSON routes send the same items.
6. **A turn is a queued or sent message, stored once per `random_id`.** A "deny and say" is a queued message that
   shows under its card, not as a message (`Queued.denied`).
7. **`POST /screens/action` is the one door for a blocks `Action`.** The chat's own actions (`stop`, `answer`,
   `archive`, `close` …) are matched by name.
8. **blocks' model-written API is merged** (#359, #361, #362, #363) with the names its ADR gave. cooee pins
   3fe5208, which is before it.
9. **A persona's keys are `agent.toml`,** edited on the persona screen. The persona's own `update_persona`
   reaches only Character and Instructions.
10. **`mcp/own.rs` and `routes/mcp.rs` took 10 PRs each in 14 days:** every new tool edits both lists.
11. **The apps draw chat items with their own Swift views.** #620 (adopt blocks' SwiftUI package) and #716 (draw
    the chat from the server's screen) are queued.

## Options

### 1. How a persona asks for blocks

| | A. A tool call whose argument is the reply | B. A JSON-only turn | C. Fenced JSON inside the text reply |
|---|---|---|---|
| Text replies, streaming, everything in fact 4 | unchanged | every reply of that persona becomes JSON | unchanged |
| Who hands a refusal back | the tool's result, in the same turn: the harness does it | cooee sends a hidden second prompt | the same hidden prompt |
| What the owner sees of a refused try | a failed tool row | a reply to hide, then another | text with raw JSON in it until the turn ends |
| New machinery | one tool | hidden turns, and a text form of every reply for fact 4 | a parser in the stream, hidden turns |

### 2. Where the catalog lives

| | A. cooee's files, picked by name per persona | B. A file per persona, written by hand | C. One catalog for every persona |
|---|---|---|---|
| Who decides what a block may send | cooee, which knows what it does with an action | each owner, in JSON | cooee |
| Editing | a picker on the persona screen | a JSON editor on a phone | none |
| A typo | cannot happen | a persona that refuses every reply | |
| Which persona gets which | the owner | the owner | nobody |

### 3. What an answer is

| | A. One per reply, shown under it | B. The controls stay live | C. The owner's message |
|---|---|---|---|
| A double tap, a retried post | one turn | two turns: an `Action` carries no id | two turns |
| What the chat shows | the reply, then `✓ Pro · iPhone` | nothing | a bubble of JSON |
| Asking again | the persona sends a new reply | | |

## Decision

**1A, 2A, 3A.**

1. **The tool is `show`.** Its arguments are the reply, `{"blocks": […]}`. It is one of cooee's own tools and is
   listed only for a thread whose persona has a catalog. The model call is the session's turn, as for every turn:
   cooee adds no model client, no key and no hidden prompt.
2. **Two catalogs, cooee's files:** `show` (prose, lists, rows, stats, tables, steps, charts; sends nothing) and
   `ask` (those, and cards, buttons, choices and forms). Links may start with `https://`; no file is loaded, so a
   reply cannot fetch an image from anywhere. Neither offers a secret. `shapes.md` has the table.
3. **The persona's pick is one `agent.toml` key, `blocks`,** set by a picker on the persona screen. No key is
   today's persona: no tool, and a brief that is byte for byte the same.
4. **The prompt is a part of `Agent::brief`:** three sentences of cooee's about the tool, then `model::prompt`'s
   text whole. It reaches the next session, as every change to a brief does.
5. **Nothing unread is stored.** A reply is read with `model::read` under the persona's catalog as it is now, in
   one function, and only that function makes the type a `Blocks` item holds. What is stored is what was read.
6. **One retry, then plain text, counted per turn.** The first refusal in a turn answers the tool with
   `no.say()`. The next call is the retry: it is drawn, or its `no.plain()` is stored. After that no call in the
   turn is read. `boundaries.md`, statechart 1.
7. **A persona that does not try again still shows.** A turn that ends after one refusal stores that refusal's
   `plain()`.
8. **The reply is a chat item, `blocks`.** On the chat screen its blocks sit in the agent's message, so every
   blocks renderer draws them with no cooee code. The JSON routes send the same item.
9. **An answer is told apart by its block, not its name.** A model-written block's id is `b<item>.<n>`; none of
   cooee's holds a `.`. The chat's door sends such an action to the answer path before it looks at its name, so a
   persona's button can never reach `archive` or `stop`.
10. **An answer passes four checks:** `model::own`, `catalog.sends` under the catalog as it is now, cooee's check
    of the values against the block the persona wrote (`shapes.md`), and the gate, which lets only a paired device
    act.
11. **A reply takes one answer.** It then shows the answer under it and refuses another. The same answer sent
    again does nothing.
12. **The answer is the persona's next turn,** through the queue the owner's messages use: at once between turns,
    after the turn that is running, once per reply. The persona gets a fixed line with the action's name and the
    values as one line of JSON.
13. **Not in a room.** There `show` answers `NOT_HERE`: a room's log is built from its members' sessions, and
    drawing a member's blocks there is another issue.
14. **The evals use cooee's real path to a model:** a scratch cooee, one persona per model with no catalog and an
    empty brief, and `cooee send --new` under blocks' `--model` contract. The replies are the models' text only.
    One PR on blocks carries the sets and names the models.
15. **The pin moves first,** in Cargo and SwiftPM together, as a slice that changes nothing else.

## Consequences

- **A blocks reply does not stream.** It appears whole when the tool call lands; the turn's text streams as today.
- **The apps do not draw it yet** (question 1). Until #716 the phone shows the turn's text and a tool row, and
  nothing for the blocks. A persona the owner mostly uses from the phone should keep no catalog until then.
- **A turn has at most one refused reply chain.** After a refusal and its retry, a later good reply in the same
  turn is not read. That is the price of "never a third try" with no way to stop a harness's model.
- **A catalog tightened later takes effect at once for reading and for answers,** and at the next session for the
  prompt. Old replies stay drawn; an answer the catalog no longer sends is refused.
- **Stored replies are typed blocks.** A reply read under an older blocks stays as stored; the pin's schema rule is
  what keeps it readable, as for every stored card.
- **History search and push do not read a blocks reply.** They read the turn's text, which the persona still
  writes.
- **The answer's turn is terse and literal.** The values are the owner's input through a form a persona designed;
  they go back to that persona and nowhere else, as JSON, so they cannot read as instructions from cooee.
- **Each harness must hand a failed tool result back to its model in the same turn.** MCP's error results exist for
  that, but it is not tested here per provider. The eval sets show how often a retry is needed at all.
- **The server must answer `show` with no approval card,** as it answers cooee's other own tools. Slice 4 confirms
  which rule does.
- **Lines:** about 600 of the 2,130 free, and about 40 of Swift's 2,739 (`drover cap cooee`). No raise.
- **Not done:** an owner-written catalog; blocks in a room; a blocks reply that changes after it is sent; drawing
  while the persona still writes; a text form of a blocks reply for search.

## Hot files

- **`mcp/own.rs`** (a central list: the tools, `own_tools`, `route`) **becomes a folder.** One file per tool in
  `crates/cooee-server/src/mcp/tools/`, each a `TOOL`; `scripts/gen.sh` writes `mcp/tools/mod.rs`, as it writes
  Chief's (its `tools` takes the entry's type). `own.rs` keeps `serve` and `call` and is frozen.
- **`routes/mcp.rs`** (a central list: thirteen routes and their chain) **becomes a folder.**
  `crates/cooee-server/src/routes/mcp/<area>.rs`, the chain written by `gen.sh`'s `routes`. The old file is
  deleted and frozen.
- **`agents.rs`** (18 PRs) gets one key, one check line, one part of the brief and one line in `save`. It is hot
  because `Agent` is the list of a persona's keys, one serde struct that refuses unknown keys, and that cannot be
  split by a folder. Not converted here; it belongs to the hotspots issue.
- **`chat/item.rs`** gets one `Body` variant; `chat/log.rs` a field and three lines in `end`; `store/thread.rs`
  one field. Everything else of the chat's is the new `chat/show.rs`, in a collected folder with a generated
  module file.
- **`screens/chat.rs`** gets one arm and one early return. **`hub.rs`** gets one `mod` line. **`hub/send.rs`** gets
  two short branches for `acted`, in `store_sent` beside `denied`'s and in `unqueue`.
- **`blocks.rs`** (core) and **`routes/threads.rs`:** untouched.
- **Swift:** the picker is a new file; `BlockPersonaEdit.swift` (9 PRs) gets the one line that places it.
- **Docs:** one new page, `docs/show.md`, with its `## Today` line. `docs/mcp.md` (12 PRs), `docs/chat.md` and the
  frozen `docs/api.md` get no line; `docs/personas.md` gets the key's row.
- **Tests:** one new file, `crates/cooee/tests/personas/show.rs`, in a generated folder; the fake agent
  (`tests/support/agent.rs`) gets one branch.

## The slices, in order

| # | Builds | Done when | Lines |
|---|---|---|---|
| 1 | the blocks pin at cf626c6 or later, Cargo and SwiftPM | every test and frame passes unchanged | about 0 |
| 2 | `mcp/tools/`, `routes/mcp/`, `Own`, `gen.sh`'s `tools` with a type | the same tools answer the same calls; `tools/list` is in name order | about 40 |
| 3 | `show/catalogs`, `show::brief`, the `blocks` key, the picker, `docs/show.md` | the brief holds `model::prompt`; no key, no change | 110, Swift 40 |
| 4 | `show::read`, `Body::Blocks`, `Chat::show`, `Hub::show`, the tool and its routes, the chat's arm | a choice is drawn; outside the catalog nothing is; twice refused is plain after one retry | 230 |
| 5 | `show::told`, `Chat::act`, `Hub::act`, `Again::act`, `Queued.acted`, the door | a pick is the persona's next turn; a bad value is a 400 and no turn | 170 |
| 6 | `cooee send --new`, `scripts/blocks-eval-model.sh`, the PR on blocks | the sets score in blocks' CI; the PR names the models | 50 |

Slice 2 is a refactor: tests only move. Slices 3 to 5 are #776's own and may be one PR. Slice 6 needs only slice
1 and the owner's answer to question 2; it spends tokens once.

## The `.drover/arch.toml` lines

`.drover/arch.toml` is written by `scripts/gen.sh` from `.drover/arch/`. Each slice adds its files there and runs
`just gen`.

Slice 2, `.drover/arch/own-tools.list` and `.drover/arch/own-tools.toml`:

```
frozen crates/cooee-server/src/mcp/own.rs
frozen crates/cooee-server/src/routes/mcp.rs
generated crates/cooee-server/src/mcp/tools/mod.rs
generated crates/cooee-server/src/routes/mcp/mod.rs
```

```toml
[[collect]]
dir = "crates/cooee-server/src/mcp/tools"
glob = "*.rs"

[[collect]]
dir = "crates/cooee-server/src/routes/mcp"
glob = "*.rs"
```

and `scripts/gen.d/own-tools`:

```
tools crates/cooee-server/src/mcp/tools crate::mcp::Own
routes crates/cooee-server/src/routes/mcp
```

Slice 3, `.drover/arch/show.list` and `scripts/gen.d/show`:

```
generated crates/cooee-core/src/show.rs
```

```
module crates/cooee-core/src/show
```

Slices 3 and 4, `.drover/arch/show.toml`:

```toml
[[ban]]
pattern = "Catalog::read"
paths = ["crates/*/src/**"]
except = ["crates/cooee-core/src/show/catalogs.rs"]
reason = "#776: a catalog is one of cooee's shipped files; no other code loads one"

[[ban]]
pattern = "model::read"
paths = ["crates/*/src/**"]
except = ["crates/cooee-core/src/show/read.rs"]
reason = "#776: a persona's blocks are read under its catalog in one place, the only maker of show::Checked"
```

In `.drover/arch.toml` these come out as `frozen = [… "crates/cooee-server/src/mcp/own.rs",
"crates/cooee-server/src/routes/mcp.rs" …]`, `generated = [… "crates/cooee-core/src/show.rs",
"crates/cooee-server/src/mcp/tools/mod.rs", "crates/cooee-server/src/routes/mcp/mod.rs" …]`, the two `[[collect]]`
tables and the two `[[ban]]` tables, with `generate = "scripts/gen.sh"` as it is.
