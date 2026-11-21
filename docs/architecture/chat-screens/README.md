# Chat, sessions and pairing as blocks screens (#714)

## Summary

Decision: cooee-server builds three new screens from core blocks kinds (chat, sessions, pair) and sends every change on one live stream per device, the protocol blocks' own clients already follow. A reply streams because the server rebuilds the chat screen up to 20 times a second and blocks sends only the text that was added. A desktop pairs by typing the six-digit code or by asking and checking a word, and signs back in with a key kept in the OS key store: no camera. No existing route changes.
Slices: five, about 1,400 lines, one issue each: (1) the blocks pin; (2) the device stream and the sessions screen; (3) the chat screen; (4) the chat's actions and uploads; (5) the pair screen and the device key.
Open questions: (1) Raise the line cap by about 600 (942 lines are free)? (2) File a small blocks issue so the composer sends an id with each send, which lets a retried send be stored once? (egui also draws no composer yet; the desktop app will need it.) (3) May a device that is not paired read the pair screen and, while its request waits, hold one stream?

## Read in this order

1. [`adr.md`](adr.md): the question, what the code says, the options, the thirteen decisions, the consequences, the
   hot files, the slices, the `.drover/arch.toml` lines.
2. [`callers.md`](callers.md): a desktop client from "pair" to "a reply streams", a screen module on the server,
   the frames of one turn, the tests. Written first.
3. [`shapes.md`](shapes.md): the routes and keys, the types, how each chat item becomes a block, the actions, the
   pair screen's states, the device key's contract.
4. [`boundaries.md`](boundaries.md): which module may call which, what holds each rule, the statecharts of a key,
   of a reply's text and of pairing.

## Where the issue's points are answered

| The issue asks | ADR decision |
|---|---|
| The routes and screen ids | 1, 2; `shapes.md` § Routes |
| How a streaming reply maps onto screen ops: the stream's shape | 2, 3, 4 |
| Ordering | 7 |
| Resume after reconnect | 7; `boundaries.md` § A key on the server |
| Tool rows and attachments | 4; `shapes.md` § A chat item as a block |
| Pairing and passkey on a desktop | 10, 11 |
| What the existing JSON routes keep doing | 12 |

## What was checked, and what was not

- Read in cooee at this branch's base (b13498747): `screens/`, `routes/threads.rs`, `routes/pairing.rs`,
  `pairing.rs`, `gate.rs`, the hub's `slice`, `subscribe`, `home` and `read`, `cooee-core`'s chat items and
  `Devices`, `apple/Sources/CooeeAPI/Passkey.swift`, `scripts/gen.sh`, `.drover/arch.toml`, `drover cap cooee`.
- Read in blocks at staging 3fe5208 (blocks#296 merged there on 2026-11-21 as PR #305): `docs/architecture/message/`,
  `wire.rs`, `shelf.rs`, `mirror.rs`, the `composer`, `island`, `message`, `tool` and `prose` kinds, the web
  composer. Read in drover: `docs/architecture/screen-hub/shapes.md`, for the stream's routes.
- Not run: cooee was not built against the new pin (it pins blocks d088209, 112 commits and one schema version
  behind); the cost of a put is blocks' own measurement (`message/measured.md`), not one on cooee's threads; the
  egui client does not exist yet, so the client sketch in `callers.md` is a sketch.
