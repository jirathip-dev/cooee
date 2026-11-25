# ADR: Chief routes to every agent, by declared purpose and mode (#802)

## Summary

Decision: every agent says in its own file what it is for and how Chief may use it: delegate (Chief asks and sums up) or hand-off (the agent's own chat opens and Chief steps aside). Chief gets one routing tool that takes only an agent that exists and cannot choose the mode. A message that starts with @name is routed by cooee before any model runs. A plugin's agent is hand-off by default, nobody can relay a hand-off agent, and a plugin's asks and cards are only ever drawn from the plugin's own screen. Your Orchestrator keeps its folder and takes its rulebook, guard and variables from drover's profile through one new line in its file.
Slices: six cooee issues in order (purpose and mode; the routing tool with delegate; hand-off; @name; an agent on a plugin's profile; the phone app follows a route), one small drover issue, and one later clean-up.
Open questions: (1) Should Chief's chat also show the open asks and cards of hand-off plugin agents (drover's), drawn from drover's own blocks (my pick), or only the Orchestrator's own chats as today?

Files: [`README.md`](README.md) (the callers, written first), [`shapes.md`](shapes.md) (keys, types, the tool, the
wire), [`boundaries.md`](boundaries.md) (who calls whom, what holds each rule, the statecharts).

## Question

How does Chief send the owner's request to the right agent, the owner's or a plugin's, so that the choice rests on
what each agent declares, an `@name` is obeyed without a model, "get me an answer" and "let me work with it" are
different paths, a plugin's asks and cards reach the owner exactly as the plugin made them, and drover's rulebook
has one source?

## What the code says (cooee staging `bd2a1352c`)

1. Chief reaches personas with `ask_persona` (its `can_message` check skipped, `hub/ask.rs`), `history` and `room`.
   Which persona it asks is its model's guess from `look`'s JSON. No agent says what it is for.
2. `ask_persona` waits in a linked exchange thread and returns the reply's text. A card the asked agent raises there
   never reaches the asker, so a relay can only describe it.
3. A plugin's profiles are lent as agents `<plugin>/<name>`: read-only, no folder. Drover serves its orchestrators
   this way (`GET /cooee/profiles`: the rulebook rebuilt from its role text on each read, the hook settings, the
   variables). Cooee tells drover of a new chat (`POST /cooee/thread-started`) only from `turns::start`: a chat the
   hub starts itself (an ask, a room seat, a routine) is never reported.
4. `GET /personas` and the phone app list only the owner's folders, so the lent Orchestrator is invisible there. The
   owner's `agents/orchestrator/` is a second, unrelated agent: its own short `INSTRUCTIONS.md`, no hook, no drover
   variables. `role = "orchestrator"` only attaches the drover plugin. The key that tied a persona to a profile
   (#417) went with agent folders (#359). `examples/agents/orchestrator/INSTRUCTIONS.md` is a 94-line copy of
   drover's text.
5. A plugin's asks and cards are not chat text. They are its `chat-cards` screen: cooee passes
   `GET /p/<plugin>/screens/cards` through, the client draws the blocks under the chats of the personas the plugin is
   attached to, and a tap goes to the plugin's `/action`. Every call is cooee calling the plugin.
6. Every owner message enters through `Hub::send` or `Hub::queue`. Rooms parse `@name` (`hub/rooms.rs`); a 1:1 chat
   does not.
7. The server-drawn chat names each agent message by its item's `speaker`, a `send` action's answer can `open` a
   screen, and a `link` item is a row that opens another thread (`Hub::link`). The phone app ignores `speaker` in a
   1:1 chat and never opens a thread by itself.
8. `Agent` and `Profile` refuse unknown keys: an older cooee drops a plugin's whole profile list if one carries a
   new key.
9. `agents.rs` has 797 of its 800 lines and 15 merged PRs in 14 days. `mcp/own.rs` is frozen. Headroom is 1,061 Rust
   lines and 2,627 Swift lines (`drover cap cooee`).

## Options

| | Choice | For | Against |
|---|---|---|---|
| Purpose A | In Chief's instructions or notes | No new key | One file every agent change must edit; plugins cannot declare |
| Purpose B | One routes file | One place to read | A central list; a plugin's agents are not in the owner's files |
| Purpose **C** | **On each agent: `agent.toml`, and a plugin's profile** | Owner and plugin agents alike; no shared file | Two new keys |
| Mode A | The model picks per call | Flexible | The model can relay an agent that must not be relayed |
| Mode **B** | **The agent declares it; a plugin's agent is hand-off unless its plugin says delegate** | The wrong path cannot be called | To chat with a delegate agent the owner opens it from Home |
| Tool A | Keep Chief's wide `ask_persona`, add a hand-off tool | Small | Two ways to one agent; the wide ask still relays anyone |
| Tool **B** | **One `route` tool; `ask_persona` loses Chief's bypass and refuses a hand-off agent for everyone** | One way; the rule is the hub's, not the prompt's | Chief's installed instructions name the old tool |
| `@name` A | Tell the model to obey mentions | No code | The model still chooses |
| `@name` **B** | **The hub reads a leading `@name` before any session is prompted** | No model call; one parser shared with rooms | Only at the start of a message |
| Hand-off chat A | The exchange thread, shown to the owner | Reuses the ask | It is a side thread: no Home row, ask rules apply |
| Hand-off chat **B** | **A chat of the agent's own, linked both ways** | The owner works there as in any chat | One new thread field |
| Blocks A | A Chief tool that forwards a block | Chief stays in the loop | A model handles the block; it can drop or reword it |
| Blocks B | The plugin posts cards into cooee (`/plugin/card`) | Exists | The plugin calls cooee: the owner ruled that out |
| Blocks **C** | **Never through a model: hand-off, plus the plugin's own screen through cooee's pass-through** | Byte for byte; cooee calls the plugin | Chief cannot comment on a card |
| Orchestrator A | Use the lent agent as it is | Nothing to build | Not on the phone: no pin, no notes, no persona row |
| Orchestrator B | Copy drover's text into the folder | Simple | Two sources; no hook, no variables |
| Orchestrator **C** | **A `profile` key: the folder says who it is, the profile how it runs** | One source; the phone keeps its agent | One resolver at start |
| Orchestrator D | Implied by `role = "orchestrator"` | No file change | A hidden rule that names drover in cooee's core |

## Decisions

1. **Two keys on every agent.** `[purpose]` (`role`, a few words, and `use`, one line) and `mode` (`delegate` or
   `hand-off`). Owner agents carry them in `agent.toml`; a plugin's in its profile. Both are checked: one line each,
   capped, and scanned like notes (#350), because they are put in front of Chief's model.
2. **The mode always has a value.** An owner agent with none is delegate. A plugin's agent with none is hand-off.
3. **The routing table is built from the agents that exist**, when it is read: every agent but Chief, a hidden one
   and a plugin profile an owner agent has taken. Only agents with a purpose are listed to Chief's model.
4. **One tool, `route(agent, message)`**, listed only in Chief's own chat like its other tools. The id is checked
   against the table at the call; an unknown one is `NO_AGENT` with the valid ids. The mode is not an argument.
5. **Delegate is today's ask**: a linked exchange thread, the ask card in Chief's chat naming the agent, the text
   back to Chief, which sums it up.
6. **Hand-off opens a chat of the agent's own.** The message is its first line (Chief's brief shown as Chief's, an
   `@name` message as the owner's), the two chats link to each other, and Chief's tool answer carries no word of
   the agent's. A second `route` in that turn is refused. The owner's next messages go to the agent alone.
7. **A hand-off agent cannot be relayed by anyone.** `ask_persona` answers `HAND_OFF` for it, whoever asks.
   Chief's wide `ask_persona` goes; in its own chat `route` is the way to an agent.
8. **`@name` at the start of a message in Chief's own chat is routed by the hub.** It names an agent by id or by
   name, declared purpose or not. The agent's mode decides the path. Chief's session is not prompted; it is told of
   the exchange before its next turn. An unknown name is refused and nothing is sent.
9. **Every route leaves a row cooee writes, naming the agent**: the ask card (delegate) or the link row
   (hand-off). After `@name` the answer itself is an item whose `speaker` is the agent. A hand-off reply is in the
   agent's own chat. The model can neither omit nor reword the label.
10. **A plugin's blocks never pass through a model.** They travel from the plugin's screen through cooee's
    pass-through to the client, and taps go back the same way. Routing code may not call the plugin client at all
    (an `arch.toml` ban). The cards show in the chats of the plugin's agent. Open question 1: also in Chief's chat.
11. **An owner agent may stand on a plugin's profile: `profile = "<plugin>/<name>"`.** At each start it takes the
    profile's instructions, agent, settings, variables, MCP picks and policy; the folder keeps its name, avatar,
    character, notes, skills, pin, model and machine. Its plugin is attached, its mode is the profile's, and its
    purpose is the folder's if set, else the profile's. A profile that is not served, or is hidden, stops the start
    with the reason. `agent.toml` may not set the profile's keys beside `profile`.
12. **The Orchestrator moves by that key.** Chief's `access` card gains `profile` (Face ID, the file's diff), or the
    owner writes the line. `cooee doctor` names an orchestrator without one and an unused `INSTRUCTIONS.md`. The
    sample loses its copied instructions.
13. **Every start of an agent that a plugin backs is reported to the plugin**, from the one place all starts pass,
    not only from `turns::start`.
14. **Drover is called, never calls.** Cooee reads `GET /cooee/profiles` and the cards screen and posts
    `thread-started` and `/action`. No route is added for a plugin to call.

## Consequences

- No other persona's tools change, except that `ask_persona` now refuses a hand-off agent.
- Chief cannot answer for, quote or sum up a hand-off agent. It can still read that agent's threads with `history`.
- To start a new topic with a hand-off agent, the owner closes the last handed chat or starts one from the agent.
- A delegate agent's blocks (#776) stay in the exchange thread, one tap from the ask card. Chief gets its text.
- An existing Chief folder keeps instructions that name `ask_persona`. `cooee setup` rewrites the file only while it
  still equals a text cooee shipped; otherwise the tool's own description is what guides the model.
- The Orchestrator on a profile cannot start while drover is off or serves no such profile. Today it would start
  unguarded on its own text.
- An older cooee would drop drover's profiles if they carried the new keys, so drover serves them only after cooee's
  first slice is released, with `min_cooee` raised. Until then the folder's own `[purpose]` makes the Orchestrator
  listable.
- When Chief's model picks a hand-off, the phone app opens the chat by itself (slice F). The server-drawn chat
  shows a row to tap: blocks has no "open" on its stream yet. After `@name` both open at once.
- `role = "orchestrator"` keeps attaching drover until the last issue removes it. The role then means nothing.
- Size: about +400 Rust lines and +60 Swift lines, inside the headroom.
- Not in this issue: editing purpose and mode on the phone's persona screen (Chief's `persona` tool and the file
  set them), `@name` outside Chief's chat, more than one target in a message, anything in drover but the two
  profile keys.

## Hot files

| File (merged PRs in 14 days) | What this design does there |
|---|---|
| `crates/cooee-core/src/agents.rs` (15; 797 of 800 lines) | Three keys on `Agent` and three lines in `list`: about +12. To make room, slice A first moves `Consolidate`, `Heartbeat` and their constants (88 lines, a pure move) to `agents/upkeep.rs`. Everything else is new files: `agents/route.rs`, and `onto` in `profiles.rs`. |
| `scripts/gen.sh` (17) | Nothing: `hub/route.rs` and `chief/tools/route.rs` land in folders it already collects. |
| `crates/cooee-server/src/routes/threads.rs` (15) | Nothing: `@name` is inside `Hub::send` and `Hub::queue`. |
| `crates/cooee-server/src/hub/tests.rs` (14) | Nothing: the tests drive the binary (`crates/cooee/tests/`). |
| `crates/cooee-server/src/mcp/own.rs` (frozen) | Nothing: `route` comes through Chief's generated tool table. |
| The app's frozen files (`ChatFeature.swift`, `ChatScreen.swift`, `AppFeature.swift`) | Nothing: slice F edits `CooeeCore/Item.swift` and adds one file under `CooeeFeatures`. |

No central list is started. `agents.rs` stays hand-written here; turning it into a generated folder is the week's
hotspots issue (#808), not this one.

## The issues to file, in order

| | Repo | Issue | Builds | AC line |
|---|---|---|---|---|
| A | cooee | Agents declare purpose and mode | The `agents/upkeep.rs` move; `purpose` and `mode` on `Agent` and `Profile`; `agents/route.rs` (the table); a save that leaves the keys out keeps them; Chief's `persona` tool may set them; purposes for the samples; `docs/personas.md` | the manifest fields |
| B | cooee | Chief's `route` tool: delegate | `hub/route.rs`, `chief/tools/route.rs`; `ask_persona` without Chief's bypass and with `HAND_OFF`; Chief's instructions; a hand-off agent answers `HAND_OFF` until C | unknown id refused; delegate answer labelled |
| C | cooee | Hand-off | The agent's own chat, `handed_by`, the two link rows, the turn's end; `thread-started` from every start | hand-off opens its own chat |
| D | cooee | `@name` in Chief's chat | The mention parser moved to core and shared with rooms; `Hub::mentioned`; the answer item with `speaker`; the `send` action's `open` | `@name` with no model call |
| E | cooee | An agent on a plugin's profile | `profile`, `profiles::onto`, the attach, Chief's `access` card, the doctor rows, the sample; Chief's deck if question 1 is yes | the original block; the Orchestrator on drover's profile |
| F | cooee | The app follows a route | An agent item with a `speaker` drawn under that agent's name in a 1:1 chat; a live outgoing link in Chief's chat opens its thread | (the phone half of C and D) |
| G | drover | Served profiles declare `purpose` and `mode` | Two keys in `GET /cooee/profiles`; `min_cooee` raised. After A is released | the Orchestrator listed without the folder's `[purpose]` |
| H | cooee | Retire `role = "orchestrator"`'s attach | `agents::list` attaches by `profile` only. After the owner's Orchestrator is on its profile | (clean-up) |

B, C and D each need the one before. E needs C. F needs C and D. A to E are tested through the binary with the fake
agent and a fake plugin backend, and need no phone.

## `.drover/arch.toml`

`.drover/arch.toml` is written by `scripts/gen.sh` from `.drover/arch/`. Slice B edits `.drover/arch/chief.toml`
and runs `just gen`. It changes one reason and adds one ban; no `frozen`, `generated` or `[[collect]]` line changes,
because `chief/tools/route.rs`, `hub/route.rs` and `agents/route.rs` fall under folders already collected.

```toml
[[ban]]
pattern = ".send"
paths = ["crates/cooee-server/src/chief.rs", "crates/cooee-server/src/chief/**"]
except = []
reason = "#664, #802: Chief never sends as another persona: route is the only way to an agent, and the hub sends"

[[ban]]
pattern = "plugin::"
paths = ["crates/cooee-server/src/chief.rs", "crates/cooee-server/src/chief/**", "crates/cooee-server/src/hub/route.rs", "crates/cooee-server/src/hub/ask.rs"]
except = []
reason = "#802: routing never fetches or forwards a plugin's blocks: they go from the plugin's screen through cooee's pass-through to the client"
```
