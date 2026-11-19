# Chief (#664)

## Summary

Decision: Chief is a normal persona folder with a fixed id and a role only it can have; setup makes it after asking for its agent and machine. In Chief's own chat with you, and nowhere else, it gets nine extra cooee tools.
Persona and routine changes save at once and leave a line in the chat. Anything touching secrets, MCP servers, machines or the config file becomes a card that shows the exact change and applies only after Face ID. Chief can ask any persona, read their history and open a room; it has no way to send as them or answer their cards.
Slices: A Chief exists (setup, rename yes, delete no); B its reach (the tool gate, look, ask anyone, read history, rooms); C free changes (personas, routines); D carded changes (MCP, secrets, a persona's access, machines, config); E Face ID approvals in the phone app.
Open questions: (1) The phone app cannot approve any Face ID card today: it sends no proof, so the server refuses. That already breaks the MCP, GitHub-routine and secret cards. Slice E fixes it with no server change: inside #664, or as its own issue first (my pick)?
(2) Adding a new machine stays a command typed on the Mac; Chief can only move a persona to a machine and set one up. Enough for now?

## Read in this order

1. [`adr.md`](adr.md): the question, what the code says, the options, the twelve decisions, the consequences, the
   hot files, the slices, the `.drover/arch.toml` lines.
2. [`callers.md`](callers.md): `cooee setup`'s transcript, the owner's API, Chief's tool calls for the issue's two
   phone checks, the server and app code that handles them, the tests by AC line. Written first.
3. [`shapes.md`](shapes.md): Chief's folder, the core types (`PersonaAsk`, `Change`, `Carded`, `Maker`), the tool
   table, the wire, the phone's proof.
4. [`boundaries.md`](boundaries.md): which module may call which, what holds each rule, four statecharts.

## Where the issue's points are answered

| The issue asks | ADR decision |
|---|---|
| Setup creates it, always there, rename but no delete (q2337) | 1, 2 |
| Setup asks for its agent and machine, no default (q2342) | 2 |
| Quiet on first start (q2339) | 2 |
| Free: personas, instructions, plugins, routines (q2338) | 5 |
| Carded with Face ID: secrets, MCP servers, machines, config; the exact change (q2338) | 6, 7, 8, 11, 12 |
| Message any persona, read history, start a room; never act as them (q2340) | 9 |
| Tools beside `remember`, `history`, `ask_persona`, for Chief only | 3, 4, 10 |
| The phone check | callers.md §3; its second half needs slice E (open question 1) |

## Decided without asking

Each is a routine call; say so if you want another.

- **Chief's tools work only in its own chat with you.** Not when another persona asks it, not in a room, not in a
  routine. One persona cannot borrow Chief, and nothing happens while you are away.
- **Archive is new** (cooee only had delete): the persona's folder moves aside, it leaves every list, and Chief can
  bring it back. Settings has no list of archived personas yet.
- **A persona Chief makes runs on this machine with Chief's agent** unless you name another. Moving it to another
  machine is a card.
- **Chief cannot delete** a persona or a routine (the issue names archive and pause), and edits itself only through
  the diff card every persona already gets.
- **`cooee setup --yes`** fails on a machine with no Chief unless both picks are passed as flags: "no default"
  leaves nothing else honest to do.
- **A pre-existing persona whose id is `chief`** is not promoted: setup asks you to rename its folder first.
- **Config changes apply at cooee's next start**; Chief cannot restart cooee.

## What was checked, and what was not

- Read at staging bae473e02: cooee's own MCP server and its routes, the hub's cards, proposals and approval route,
  `agents.rs`, the routines' core and runner, rooms, `ask_persona`, `history`, the secrets and config writers,
  `machine.rs`, `setup.rs`, the passkey check and its test authenticator, the app's card answer path and persona
  screen, `scripts/gen.sh`, `.drover/arch.toml`, `drover cap cooee`.
- Not run: any code. The Secure Enclave key (slice E) is specified from the server's check and the test
  authenticator, which sign the same bytes; the simulator may need a keychain key instead.
