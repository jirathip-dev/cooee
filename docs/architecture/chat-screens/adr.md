# ADR: chat, sessions and pairing as blocks screens (#714)

## Summary

Decision: cooee-server builds three new screens from core blocks kinds (chat, sessions, pair) and sends every change on one live stream per device, the protocol blocks' own clients already follow. A reply streams because the server rebuilds the chat screen up to 20 times a second and blocks sends only the text that was added. A desktop pairs by typing the six-digit code or by asking and checking a word, and signs back in with a key kept in the OS key store: no camera. No existing route changes.
Slices: five, about 1,400 lines, one issue each: (1) the blocks pin; (2) the device stream and the sessions screen; (3) the chat screen; (4) the chat's actions and uploads; (5) the pair screen and the device key.
Open questions: (1) Raise the line cap by about 600 (942 lines are free)? (2) File a small blocks issue so the composer sends an id with each send, which lets a retried send be stored once? (egui also draws no composer yet; the desktop app will need it.) (3) May a device that is not paired read the pair screen and, while its request waits, hold one stream?

Files: this ADR, [`callers.md`](callers.md) (how the callers read, written first), [`shapes.md`](shapes.md) (the
routes, the types, the block for each chat item, the device key), [`boundaries.md`](boundaries.md) (who calls
whom, what holds each rule, the statecharts).

## Question

How does cooee-server serve the chat, the list of chats and pairing as screens any blocks renderer draws with no
cooee code, so that a reply still streams, a dropped connection loses nothing, a desktop with no camera can pair
and sign in, and the apps that call today's routes notice nothing?

## What the code says

1. **cooee's "blocks screens" are partly cooee's own kinds.** `home.json` is two app kinds (`pinned`, `chats`)
   over `threads` and `personas` data; `pairing.json` is `step`, `scanner`, `nearby`, `word` and others. Only the
   Swift app draws them. An egui client would draw their fallback rows.
2. **`/screens/pairing` is behind the gate** (`gate.rs`, `OPEN`): a device that is not paired gets 401, and the
   app draws its bundled copy. No server screen reaches an unpaired client today.
3. **Core screens have no stream.** Plugin screens have the old per-screen `{seq, ops}` stream, proxied
   (`plugin/feed.rs`). blocks' migration plan (C5, cooee#631) puts cooee's own screens on blocks' device stream:
   `GET /stream`, `Shelf` on the server, `Mirror` or `Replica` on the client. drover's screen-hub ADR sets its
   two routes.
4. **A chat is state the hub holds.** `hub.slice` gives a window of items and the turn, text so far included, under
   the thread's lock. `hub.subscribe` gives a wake for every change. A thread counts as open, with no push, while
   anything listens (`receiver_count`). `hub.rs` is 790 lines of its 800.
5. **A turn is stored as its items in order, and its text once, last.** Text chunks grow `Turn.text`; tool calls
   are items as they come; `Reply` stores the whole text as one `Agent` item. Item ids count up by one per thread.
6. **blocks' `message` is who speaks around ordinary blocks** (staging 3fe5208): a `prose` for text, a `tool` per
   call, attachments as a field. Text grows by the `append` op, which only `diff` writes: the app puts the longer
   screen. Measured there: a put costs about 2.5 ms on a 40-message screen; `append` sends 0.15 MB for an
   8,000-character reply where a merge sends 4.2 MB.
7. **The app's passkey is a device key.** The app keeps a P-256 key in the keychain (the Secure Enclave where
   there is one) and `Passkey.swift` builds the WebAuthn-shaped bytes itself; the server checks the challenge, the
   host and the signature. No browser and no Apple passkey service is involved, so any client with a key store
   can do the same.
8. **The composer sends no id.** Its action carries `text`, the chips and, on the web, `files`. cooee's send
   needs a `random_id` so a retry is stored once. blocks draws the composer in HTML, React and SwiftUI; egui draws
   its fallback row (`draw/composer.rs` is still a `TODO(#45)`).
9. **cooee pins blocks d088209**, 112 commits and one schema version before `message`, in `Cargo.toml` and
   `apple/Package.swift`.

## Options

### 1. How a reply reaches a renderer

| | A. Rebuild the screen, let `Shelf` diff | B. Turn each hub event into ops by hand | C. Keep `/threads/{id}/events`, build blocks in each client | D. The old per-screen `{seq, ops}` stream |
|---|---|---|---|---|
| Who knows what a chat looks like | one builder | the builder and an op writer, which must agree for ever | every client, again | one builder |
| Resume | blocks' revs: nothing, the ring, or the screen | cooee's own, to write and prove | today's cursor, per client | none: `append` becomes a merge of all the text |
| Server cost | a put per tick, about 2.5 ms | less | none | a put per tick |
| New client code | none | none | all of it | the format blocks is retiring |

### 2. Pairing and sign-in with no camera

| | A. A typed code, a check word, and a device key in the OS key store | B. The OS's own passkeys | C. The system browser, then hand the session to the app | D. Code only |
|---|---|---|---|---|
| Needs | what the server has today | a web domain tied to the app, which a tailnet name or localhost cannot be | a cooee web page; the owner said none for now | nothing |
| Sign back in, approve a proposal | yes | yes | yes | no: pair again each time |

### 3. Long chats

| | A. One key per chat, 50 items, `older` widens it for its holders | B. Older pages as separate screens | C. A window in the key, one per device |
|---|---|---|---|
| Reading back | in place | a second screen to move to | in place, if the client swaps keys in place: new client code |
| Server cost | grows with the window, capped | none | one shelf per window in use |

## Decision

**1A, 2A, 3A.**

1. **Three screens of core kinds, beside the others.** `/screens/sessions`, `/screens/chat/{id}`, `/screens/pair`,
   ids `sessions`, `chat`, `pair`, in `screens/`. No cooee kind is in them, so every renderer draws them. The one
   exception is the `device-key` island (decision 11), which has a fallback. `shapes.md` § Routes.
2. **One stream per device, blocks' protocol.** `GET /stream` and `POST /stream/{sid}`, the routes drover's
   screen-hub ADR sets and the frames `Mirror`, `Replica` and `stream.ts` follow. A key is a screen's path. cooee
   writes no stream format.
3. **A screen is published by building it again.** `live.rs` holds a `Shelf` per key somebody has open. A change
   in the hub wakes the key; the key's screen is built from `hub.slice`; `Shelf::put` diffs it; the frame goes to
   every stream that holds the key. A screen module returns a `Block` and never writes an op.
4. **An agent's turn is one `message`.** Its run of items is a message: each tool call a `tool`, each ask its
   `card` (the one `Item::block()` builds today), the text a `prose` that is last and has `streaming` until the
   reply is stored. The owner's message carries its files as attachments. The table is `shapes.md` § A chat item
   as a block. The message's id comes from its first item, and from the id that item will get when text arrives
   first, so the block that streams is the block that is stored and nothing is sent twice.
5. **The pace.** A key is put at most 20 times a second, and after a put that took `t` the next waits `10 t`, so
   one reply never takes more than a tenth of a core however long the chat. Only held keys are built.
6. **Fifty items, wider on request.** The key holds the newest 50 items. `older` adds 50 for everyone who holds
   it, to 500; the count is forgotten with the key, 5 minutes after its last holder left.
7. **Order and resume are blocks'.** Per key, revs count up by one and a frame applies only on the rev it names.
   A reconnect, a restart and a handover are one case: `hello`, one sync with the revs held, and each key answers
   nothing, the ring's ops or the whole screen. A screen is state, so text can be neither lost nor doubled; there
   is no `reset` to handle.
8. **A held chat is an open chat.** While a paired device holds a chat's key its thread sends no push (the hub's
   rule today, since the key listens) and is marked read up to the newest item each time a frame goes out. A
   client needs no read call.
9. **Actions go through the door that exists.** `POST /screens/action` takes blocks' `Action` and answers its
   `Answer`; the change itself comes on the stream. Each chat action calls the function its JSON route calls,
   moved to `turns.rs`, so both doors share the restart of a gone agent, the passkey rule for proposals and the
   store-once rule. A send's id is the composer's `nonce` (open question 2). Files go first to
   `POST /screens/files`, and the action names their ids.
10. **A desktop pairs on a server screen, after finding the server itself.** `/screens/pair` and its action route
    answer anyone. It offers the six-digit code (`cooee pair`, or a paired device's Settings), `Ask to pair` with
    the check word, and sign-in with the device key. While a request waits, its claim sits in a cookie, and with it
    the device may hold one stream that carries only the pair screen, so the yes shows at once. The device cookie
    is set by the same three checks as today: a right code, a collected claim, a good signature.
11. **The passkey on a desktop is a device key.** The client keeps a P-256 key in the OS key store and signs the
    bytes `Passkey.swift` signs today. The screen places it as the island `device-key`; the server's passkey
    routes do not change. It signs in, is enrolled after pairing, and approves a proposal card.
12. **Today's routes do what they do.** Nothing is removed or changed. The native apps and `cooee send` keep
    calling them; both doors read one hub and one `devices.json`, so a message sent through one shows through the
    other. What each app's switch may delete is listed in `shapes.md`, for those issues to decide.
13. **The pin moves first,** in Cargo and SwiftPM together, to a blocks commit with `message`, as its own slice
    that changes nothing else and passes every test and frame unchanged.

## Consequences

- **A new client is a renderer and a cookie jar.** Chat, the list and pairing need no client code but the
  device key and the first "where is my server" window.
- **The server pays for each watched reply:** up to a tenth of a core, and a 50-item screen kept as JSON per held
  key. A chat nobody watches costs nothing.
- **Text sits after the tool rows of its turn,** as today's chat shows it. blocks could show text between two
  calls in the order it was said, but cooee stores a turn's text once. Storing it in pieces is another issue.
- **Items older than the newest 500 cannot be reached from the screen.** `GET /threads/{id}/messages` still
  reaches them. A history screen is not in this issue.
- **A window left open keeps its chat read and its pushes quiet,** as a page left open does today.
- **Approving another device stays where it is** (a paired phone, or `cooee pair approve`). A card for it on the
  sessions screen belongs to the apps' switch.
- **New open surface, if question 3 is a yes:** the pair screen, which shows the server's name and nothing else,
  its action route, and a stream for a request that waits. Requests are 4 at once and 5 minutes each, as now; a
  wrong code counts against the same 5 a minute as `POST /pair`. If it is a no, the pair screen ships inside each
  client, as the app's does today, and a desktop pairs by code or signs in by key through the open routes that
  exist; slice 5 shrinks to the device key.
- **The egui app needs blocks to draw the composer** (a `TODO` in `blocks-desktop` today). That is the desktop
  issue's ground, named here so it is not found late.
- **Slice 4 waits for blocks** (question 2). If the answer is no, a send is taken as a retry when the same device
  sends the same text and files as the chat's last owner message within 30 seconds and the agent has said nothing
  since. That rule can drop a message meant twice, which is why the id is the better fix.
- **The pin is the risky slice.** Schema 1 to 2 may change what the app decodes. If the Swift side does not build
  or a frame moves at the new commit, slice 1 stops and asks: that ground is cooee#620's.
- **About 1,400 lines** against 942 free once PR #706 lands (`drover cap cooee`). Nothing here deletes code: the
  screens they replace go with the apps' switch.

## Hot files

- **`hub.rs`** (790 of 800 lines): untouched. The publisher uses `subscribe`, `slice`, `home` and `read`.
- **`screens/persona.rs`** owns `POST /screens/action` and its `match` on the screen: a central list. It becomes
  generated: `scripts/gen.sh` writes `screens::act` into `screens/mod.rs`, trying each screen module's `act` in
  name order, as it writes `routes`. `screens::source` is written the same way. A new screen is a new file.
- **`routes/threads.rs`** (551 lines) gets smaller: start, send and answer move to `turns.rs`.
- **`lib.rs`, `screens/mod.rs`, `routes/mod.rs`** are generated already: add the file, run `just gen`
  (`live` and `turns` join the private names on `gen.sh`'s `crate` line).
- **Docs** go to their areas: the stream to a new `docs/stream.md`, the chat screen to `docs/chat.md`, the pair
  screen to `docs/pairing.md`. `docs/api.md` (frozen) and `docs/layout.md`'s table get no line.
- **Tests**: the stream's in a new `crates/cooee/tests/stream.rs`; the chat's and pairing's in their areas' files.

## The slices, in order

| # | Builds | Done when | Lines |
|---|---|---|---|
| 1 | the blocks pin, Cargo and SwiftPM, at 3fe5208 or later | every test and frame passes unchanged | about 0 |
| 2 | `live.rs`, `routes/stream.rs`, `screens/sessions.rs`, `gen.sh`'s `source` | a `Mirror` in the test follows the list through drops, doubles and a restart | 480 |
| 3 | `screens/chat.rs`: the builder, its `GET`, its source, the read mark | a reply streams by `append`; tool rows and attachments are there | 320 |
| 4 | `turns.rs`, the chat's `act`, `POST /screens/files`, `gen.sh`'s `act` | send once through both doors; queue, steer, stop, cards, `older` | 300 |
| 5 | `screens/pair.rs`, `pairing::try_code`, `Devices::peek`, the gate, the island on proposal cards | a code pairs; a word pairs; a key signs in; an unpaired stream opens one key | 300 |

Slices 2 to 5 each ship alone and change nothing an app calls. Slice 4 is blocked on the blocks issue of question 2.
#714 closes with slice 5; the egui app needs 2 to 5, the iPhone and Mac switch the same.

## The `.drover/arch.toml` lines

Slice 2, in the PR that adds `live.rs`:

```toml
[[ban]]
pattern = "Shelf"
paths = ["crates/cooee-server/src/**"]
except = ["crates/cooee-server/src/live.rs"]
reason = "#714: only live.rs publishes a screen: a screen module builds a Block and never diffs, puts or frames"
```

Slice 5, in the PR that adds `screens/pair.rs`:

```toml
[[ban]]
pattern = "cookie(cx"
paths = ["crates/cooee-server/src/**"]
except = ["crates/cooee-server/src/pairing.rs", "crates/cooee-server/src/routes/pairing.rs", "crates/cooee-server/src/screens/pair.rs"]
reason = "#714: a device cookie is set only where a code, a claim or a device key was checked"
```

No `frozen`, `generated` or `[[collect]]` line changes: `screens/mod.rs` is in `generated` already
(`generate = "scripts/gen.sh"`), the new screen and route files fall under the `crates/cooee-server/src` `*/*.rs`
collect, and every new file is under the 800-line rule.
