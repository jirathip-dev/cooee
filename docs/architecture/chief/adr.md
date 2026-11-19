# ADR: Chief, cooee's own persona that manages personas, routines and config (#664)

## Summary

Decision: Chief is a normal persona folder with a fixed id and a role only it can have; setup makes it after asking for its agent and machine. In Chief's own chat with you, and nowhere else, it gets nine extra cooee tools.
Persona and routine changes save at once and leave a line in the chat. Anything touching secrets, MCP servers, machines or the config file becomes a card that shows the exact change and applies only after Face ID. Chief can ask any persona, read their history and open a room; it has no way to send as them or answer their cards.
Slices: A Chief exists (setup, rename yes, delete no); B its reach (the tool gate, look, ask anyone, read history, rooms); C free changes (personas, routines); D carded changes (MCP, secrets, a persona's access, machines, config); E Face ID approvals in the phone app.
Open questions: (1) The phone app cannot approve any Face ID card today: it sends no proof, so the server refuses. That already breaks the MCP, GitHub-routine and secret cards. Slice E fixes it with no server change: inside #664, or as its own issue first (my pick)?
(2) Adding a new machine stays a command typed on the Mac; Chief can only move a persona to a machine and set one up. Enough for now?

Files: this ADR, [`callers.md`](callers.md) (how the callers read, written first), [`shapes.md`](shapes.md) (the
types, the tools, the wire), [`boundaries.md`](boundaries.md) (who calls whom, what holds each rule, the statecharts).

## Question

How does one built-in persona get to set up and manage cooee (personas, routines, MCP servers, secrets, machines,
the config file) and to talk to every persona, so that no other persona gains any of it, the risky changes cannot
apply without the owner's Face ID, and the card the owner approves is exactly what is applied?

## Constraints found in the code (staging bae473e02)

1. cooee's own MCP server is one process per thread (`cooee mcp serve --thread <id>`, `mcp/own.rs`) with a fixed
   list of nine tools; each call goes back to cooee with the thread's token (`POST /mcp/agent/*`). The process runs
   in the agent's hands, so only cooee's check of the token counts.
2. `agents::save` already keeps a persona's launch keys (`env`, `mcp`, `sites`, `policy`, `account`, `settings`)
   from the file whatever the body says. So "edit a persona" and "touch its secrets or MCP servers" are already two
   different writes.
3. A card of cooee's own is an approval item plus a pending value in memory (`Hub::raise`, `Proposed`); forms
   `mcp`, `routine` and `secret` need a passkey on `POST /threads/<id>/approve` (`is_proposal`), checked before the
   hub hears of the tap. A restart drops pending values ("ask the agent again").
4. **The native app sends no passkey** (`Endpoints.swift` `answer`), and has no code that makes one. Approving any
   of those three forms from the phone gets 403 today. The app also draws only an item's block, and the block of an
   `mcp` or `routine` card leaves out its exact command or prompt.
5. A routine is made from the thread that asks (`Routine::make(…, from: &Thread, …)`): its place is that chat, or
   new threads started as that thread was. There is no way to make one for another persona.
   `Runner::as_persona` already builds the same "who" from a persona.
6. A persona can only be deleted (its folder removed); there is no archive. Machines are shed's: cooee has no
   "add a machine", only `cooee machine check <m> --yes` (clone the folders, copy the rules there).
7. `cooee setup --yes` answers every question with its default; Chief's two have none (q2342).
8. Hot or full files on the path: `hub.rs` (66 PRs in 14 days, 788 of 800 lines), `setup.rs` (18), `hub/mcp.rs`
   (17, 712 lines), server `routines.rs` (13, 740), `agents.rs` (9), core `routines.rs` (8, exactly 800),
   `routes/mcp.rs` (7), `tests/mcp.rs`, `tests/personas.rs`, `docs/mcp.md`.
9. Headroom at staging (`drover cap cooee`): 1,141 Rust lines, 219 Swift lines.

## Options

| | Choice | For | Against |
|---|---|---|---|
| Powers A | A plugin attached to Chief, as drover is to an orchestrator | No core change | A second process and token for what cooee already owns; its cards would be a plugin's, which need no Face ID |
| Powers B | Chief runs `cooee …` commands in a shell | Nothing to build for reads | Any persona with a shell has the same powers; no card before a write |
| Powers C | A `can_manage` key any persona may carry | Flexible | The issue says one built-in; a key in a file an agent can edit |
| Powers **D** | **cooee's own MCP tools, listed and answered only for Chief's threads** | What the issue asks; one gate, on the server | A per-thread tool list (one extra call at session start) |
| Identity A | The id `chief` alone | Simplest | An owner's older persona with that id would gain the tools on an update |
| Identity B | A role alone | No reserved id | Nothing stops two |
| Identity **C** | **The id and the role, each implying the other** | One by construction; no silent promotion | Setup must refuse when the id is taken by a plain persona |
| Card A | A closure run on approval (`Proposed::Later`, as routines do) | No new type | The card's text and what runs are written apart and can drift |
| Card **B** | **A typed `Change`, made with its card text by one checked constructor** | "The card shows the exact change" holds by construction; testable | One enum, one `apply` |
| Archive A | An `archived` key in `agent.toml` | Visible in the file | Every reader of the list must remember to skip it |
| Archive **B** | **Move the folder to `agents/.archive/<id>`** | No reader changes; nothing can start it; one rename undoes it | Not listed in Settings (Chief lists and restores) |
| Phone proof A | A system passkey in the app | Same as the browser | Apple ties one to a web domain fixed in the app; a tailnet name is per owner |
| Phone proof B | Face ID checked by the app only | 10 lines | The server cannot tell, so the rule would live in the app alone and the 403 would have to go |
| Phone proof **C** | **A key in the phone's Secure Enclave, Face ID on each use, signing the same message a passkey signs** | The server's check and tests stay as they are | One key per server, registered on first use |

## Decisions

1. **Chief is a persona folder, `agents/chief/`, with `role = "chief"`.** Core holds "id `chief` ⇔ role chief" in
   `save`, `remove` and `load`. Renaming changes `name` only. Delete and archive are refused in core, so the API,
   the CLI and Chief itself all get the same no. The app hides Delete by the role it already receives.
2. **`cooee setup` gets a `chief` step** after `agents`: it lists shed's agents and machines and takes a pick for
   each, with no default. An existing Chief is skipped. With `--yes` and no Chief it needs `--chief-agent` and
   `--chief-machine`, checked before any step runs. A folder `chief` that is the owner's own persona stops the
   step with "rename it". Setup writes the folder and nothing else: no thread, no routine (q2339). It is pinned to
   Home. Installs from before this issue get it on their next `cooee setup`; `cooee doctor` says so.
3. **One gate.** `Hub::chief_of(thread, token)` returns `Me` only for Chief's own chat with the owner; every tool
   takes `&Me`. `tools/list` for any other thread is today's nine.
4. **Only in Chief's own chat.** In a thread another persona asked, a room member's session, a routine's run or a
   Workflow session, Chief is an ordinary persona: no extra tool, no wider `history` or `ask_persona`. So a
   persona that may message Chief can neither change anything through it nor read another persona's threads
   through it, and nothing Chief reads can act while the owner is away.
5. **Free changes** (q2338): `persona` create, edit, archive, restore, limited to `PersonaAsk` (name, the two
   texts, avatar, agent, model, effort, plugins, `can_message`); `routine` create for a persona, edit, pause,
   resume. Each saves through the phone's own checks, runs #350's text scan, and leaves cooee's line in the chat.
   A new persona runs on this machine with Chief's agent unless told otherwise. Chief edits itself only with
   `update_persona` (a diff card, as for everyone). A GitHub or webhook routine keeps its card (#261).
6. **Carded changes** (q2338): `mcp_server` (remove, on, off; adding stays `propose_server`), `stored_secret`
   (set, remove), `access` (a persona's variables, MCP picks and machine), `machine` (setup), `config` (set, unset).
   One change a card, form `change`, drawn as a proposal card with the exact change (a line diff for the config
   and for `agent.toml`). Approving needs Face ID; a file changed since the card was made fails the approval and
   writes nothing. Chief's chat is told the outcome.
7. **A persona's machine is carded**, though the phone sets it freely: where code runs is a machine change. So
   Chief makes a persona here, then asks to move it.
8. **Chief never holds a secret.** `stored_secret set` is the existing masked card; `look` shows names only and the
   config with tokens redacted; `config` cannot touch `plugins` (tokens) or `mcp` (its own tool).
9. **Other personas** (q2340): `ask_persona` skips `can_message` for Chief (depth and timeout stay); `history`
   takes a `persona` for Chief and reads any persona's thread; `room` opens a room of Chief and up to four others
   with a cooee note saying why, and the owner speaks first, as in any room. There is no tool that sends into a
   session or answers a card, and `arch.toml` bans the calls that could.
10. **Chief's tools are a folder**, one file a tool, with a generated table that drives both the list and the
    call. `mcp/own.rs` learns two paths once and never a tool's name.
11. **The phone proves it is the owner with a device key** (slice E): made in the Secure Enclave on first use,
    saved with `POST /passkeys`, signing each approval's challenge; iOS asks for Face ID per signature. The app
    does it for cooee's own proposal and secret cards. The server also draws `mcp` and `routine` cards as proposal
    cards so their exact command shows.
12. **Config changes apply at cooee's next start**, and the card's line says so. Chief cannot restart cooee.

## Consequences

- No other persona's tools, prompts or cards change. A persona can still ask Chief if its `can_message` says so,
  and gets talk, not changes.
- Chief cannot use its tools in a room or in a routine of its own (a weekly "which personas were idle" run would
  see only its own history). If the owner wants that later, it is a wider `chief_of`, in one place.
- An archived persona's routines fail their runs with the hub's sentence, as a deleted persona's do today;
  Chief's instructions tell it to pause them first. Settings shows no archive list; Chief lists and restores.
- Several steps mean several Face ID prompts (add a server, turn it on, give it to a persona: three). One change
  a card keeps each card exact; a batch card can come later.
- A pending change is lost when cooee restarts, like every proposal today.
- `cooee setup --yes` without the two flags now fails on a machine with no Chief. `tests/setup.rs` passes them.
- Not in this issue: adding a machine to shed; a persona's folder, sites and policy through Chief; deleting a
  routine or a persona through Chief (the issue names archive and pause).
- Size: about +900 Rust lines against 1,141 headroom, and about +170 Swift against 219. Both fit; neither leaves
  much.
- Not checked: no code was run. Whether the simulator can hold an enclave key (if not, the live client keeps a
  keychain key there). Whether Home refreshes by itself after a persona is made on the server; it lists it on its
  next read either way.

## Hot files

None of them is a list this issue converts, so nothing is frozen. The design keeps new code out of them:

| File (PRs in 14 days) | What this issue does there |
|---|---|
| `hub.rs` (66), a kernel by #356 | `mod chief;` and one `use` |
| `setup.rs` (18) | `mod chief;` and one call; the step is `setup/chief.rs` |
| `hub/mcp.rs` (17) | one `Proposed` variant, one match arm, `change` in `is_proposal`, the `history` reach: edits in place |
| server `routines.rs` (13) | `create` split into "who asks" and `create_by`; `as_persona` shrinks by using `Agent::who`: net about zero |
| `agents.rs` (9) | the role, three guards, `may_message`: about +25; archive and create live in `chief.rs` |
| core `routines.rs` (8, at its cap) | `make` takes `&Maker`; the type is `routines/maker.rs`: net negative |
| `routes/mcp.rs` (7) | nothing: Chief's two routes are `routes/chief.rs` |
| `mcp/own.rs` | about +12 lines, once |
| `tests/mcp.rs`, `tests/personas.rs`, `docs/mcp.md` | nothing: `tests/chief.rs`, `docs/chief.md` |

The one central list this issue would have started is Chief's tool list. It is a folder from the first PR:
`crates/cooee-server/src/chief/tools/`, one file a tool exporting `TOOL`, collected by `scripts/gen.sh` into
`chief/tools/mod.rs` (`ALL`). `gen.sh` gains a `tools` function (as `cmds`), the line
`tools crates/cooee-server/src/chief/tools`, and `chief` in the server crate's private list.

## Slices, in order

| | Builds | AC lines |
|---|---|---|
| A | `Role::Chief` and its guards, `chief::create`, the setup step and flags, the doctor row, the app's hidden Delete | setup; delete and rename |
| B | `chief_of` and `Me`, the two routes, `own.rs`'s merge, the generated table, `look`, `room`, `ask_persona` and `history` for Chief | reach every persona; history and room; no other persona |
| C | `persona`, `routine`, archive and restore, `Maker` | changes at once; phone check, first half |
| D | `Change`, `Carded`, `propose_change`, `apply_change`, the `change` form, the five carded tools | cards and Face ID, per tool |
| E | the app's device key and `prove`, `passkey` on answer, proposal blocks for `mcp` and `routine` | phone check, second half |

A to D are tested through the binary with the fake authenticator and need no phone. E is the only Swift beyond
A's one condition.

## `.drover/arch.toml`

Slice B, in the PR that adds the folder: add `"crates/cooee-server/src/chief/tools/mod.rs"` to `generated`
(written by `generate = "scripts/gen.sh"`), and:

```toml
[[collect]]
dir = "crates/cooee-server/src/chief/tools"
glob = "*.rs"

[[ban]]
pattern = ".approve("
paths = ["crates/cooee-server/src/chief.rs", "crates/cooee-server/src/chief/**"]
except = []
reason = "#664: Chief never answers a card, its own or another persona's"

[[ban]]
pattern = ".send"
paths = ["crates/cooee-server/src/chief.rs", "crates/cooee-server/src/chief/**"]
except = []
reason = "#664: Chief never sends as another persona: ask_persona is the only way to one, and it sends as Chief"

[[ban]]
pattern = ".queue("
paths = ["crates/cooee-server/src/chief.rs", "crates/cooee-server/src/chief/**"]
except = []
reason = "#664: a Chief tool never queues a message into a thread; the hub tells Chief its own card's outcome"
```
