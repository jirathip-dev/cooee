# Shapes (#714)

Rust sketches are the source for impl; bodies are not final. Every block type is blocks' own (staging 3fe5208).

## Routes

| Call | Who may | Answer |
|---|---|---|
| `GET /screens/sessions` | a paired device | screen `sessions`; `?archived=1` lists the archived chats |
| `GET /screens/chat/{id}` | a paired device | screen `chat` of thread `id`, its newest 50 items; 404 for no such thread |
| `GET /screens/chat?persona=<id>` | a paired device | screen `chat` with a composer only: a chat not started yet |
| `GET /screens/pair` | anyone | screen `pair`, drawn for who asks (§ The pair screen); `?code=` fills the code field |
| `POST /screens/pair/action` | anyone | blocks' `Action` in, `Answer` out; sets or clears the two cookies |
| `POST /screens/action` | a paired device | exists; a `screen` under `/screens/chat` or `/screens/sessions` answers blocks' `Answer` |
| `POST /screens/files` | a paired device | the composer's file handler: the body is the file, `X-Name` its name; answers `{id, name, src}` |
| `GET /stream` | a paired device, or a request that waits | SSE: `hello`, frames, `bye`; `Accept` names the blocks version |
| `POST /stream/{sid}` | the stream's own caller | blocks' `Sync` in, `Synced` out; 404 when `sid` is not its live stream |

A stream's key is a screen's path with its query, as `GET` takes it: `/screens/chat/k3`. Only the three screens
above are keys in this issue; any other key answers `gone: 404`, and a key a caller may not read `gone: 401`.
Limits, as drover's ADR sets them: 4 streams a device (the oldest gets `bye evicted`), 16 keys a stream, and a
stream with over 1 MB unsent gets `bye evicted` and syncs again.

## The publisher: `live.rs`

```rust
pub(crate) enum Who { Device(Device), Asking(String) }          // set by the gate: a cookie's device, or a claim's hash
pub(crate) enum Wake { Thread(String), Home, Every(Duration) }  // hub.subscribe, hub.home, a tick
pub(crate) struct Source {
    pub wake: Wake,
    pub build: Box<dyn Fn(&Hub, usize) -> Result<Block, HubError> + Send + Sync>,   // usize: pages asked for by `older`
    pub seen: Option<Box<dyn Fn(&Hub) + Send + Sync>>,          // after a frame went to a device: a chat marks itself read
}

pub(crate) struct Live { .. }                                   // per key: its Shelf, its holders, dirty, page, last cost
impl Live {
    pub(crate) fn attach(&self, who: Who, accept: &str) -> Result<(Hello, Frames), Outdated>;   // GET /stream
    pub(crate) fn sync(&self, sid: &str, who: &Who, s: Sync) -> Option<Synced>;                  // POST /stream/<sid>
    pub(crate) fn more(&self, path: &str) -> blocks::Answer;                                     // one page wider, at most 10
}
```

- A key is `(path, scope)`: every paired device shares one copy of a chat or of sessions; the pair screen has one
  copy per claim. Each key has its own `Shelf`, so two replies never wait on each other.
- `Hello.epoch` is the hub's boot id, so a restart is a new epoch and every key is sent whole.
- `screens::source(path, who)` (generated, as `screens::routes` is) asks each screen module for its `Source`.

## A chat item as a block

The chat screen: an `older` row (while older items exist), a `section` `log` of messages, a `state` row (only when
the turn needs words), the `composer` `say`.

| Item (`cooee-core`) | Block | Id |
|---|---|---|
| `You` | `message`, role `owner`, name `You` (a routine's name when a routine sent it), time; a `prose` of its text; its `files` as attachments | `m<id>`, prose `i<id>` |
| `Agent` | the `prose` that ends its group's message | `<message id>-text` |
| `Tool` | `tool`: `name` its action word, `summary` its subject (80 characters; the whole subject and the rule's line in `body`), `state` `running` (pending, in progress), `done`, `failed` (failed, cancelled) | `i<id>` |
| `Approval` | the `card` `Item::block()` builds today, unchanged; a proposal's approve button is the `device-key` island | as today |
| `Plan` | `steps` | `i<id>` |
| `Ask`, `Handoff` | a `row` that opens the other chat; a `card` whose button sends `handback` with the owner's note | `i<id>` |
| `Frame` | `images`, from `/threads/<id>/frames/<name>` | `i<id>` |
| `Link`, `Routine`, `Note`, `Ended` | `message`, role `system`, name `cooee`, one `text`; a link opens the other chat | `m<id>` |

- **A group** is one agent's run of items. It ends at an owner or system item, at another speaker (a room) and
  after an `Agent` item. Its message is `m<n>`, `n` the id of its first item.
- **The text still coming** (`Turn.text`) is a `prose` with `streaming`, last in the turn's group. A turn with no
  item yet has the group `m<last id + 1>`: item ids count up by one, so that is the id its first item will take.
- **An attachment** is `{name, tap: {url: "/threads/<id>/files/<name>"}}`, with the same URL as `src` for an image.
- **The composer**: `action` `send`; `acts` `queue`, `steer`, `unqueue`, `stop`; `chips` the model and the effort;
  `queued` each waiting message (`q`: the first 48 bits of its `random_id`'s hash); `working.since` while a turn runs.
- **The state row**: `unconnected` (the note, a `retry` button), `error` and `limited` (the note), `ended` (why).
  `working`, `blocked` and `done` need no row: the composer and the open card show them.

## Actions

`POST /screens/action`, `screen` a chat's key. Each calls what its JSON route calls today, through `turns.rs`.

| Action | Sent by | Values | As |
|---|---|---|---|
| `send` | the composer | `text`, `files` (ids from `/screens/files`), `nonce`, `model`, `effort` | `POST /threads/{id}/send`; with a changed pick, `/model` first; on a chat not started, `POST /threads` in the background, answering `open` |
| `queue`, `steer`, `unqueue` | the composer | as `send`; `q` | `/queue`, `/steer`, `/unqueue` |
| `stop` | the composer | | `/cancel` |
| `answer` | a card | `item`, `choice`, `value`; a proposal's also `id`, `client`, `auth`, `sig` | `/approve` |
| `handback`, `retry` | a card, the state row | `text`, `nonce`; none | `/handback`, `/retry` |
| `older` | the `older` row | | the key grows by 50 items, to 500 at most |
| `archive`, `close` | the screen's menu | | `/archive`, `/close` |

The answer is blocks' `Answer`: empty when done (the change arrives on the stream), a `bad` toast with the hub's
words when refused, `open` when a chat was started. On `/screens/sessions`: `archive` with a row's `thread`.

## The sessions screen

One `section` per persona that has open chats, in `GET /personas` order, then one for chats with no persona. Each
chat is a `row` `t-<thread>`: its title, its last line toned by its state, its unseen count as the badge, a tap
that opens `/screens/chat/<id>`, a swipe that archives. Each section ends with a `New chat` row that opens
`/screens/chat?persona=<id>`. The rows are `GET /threads`'s, so both lists always agree.

## The pair screen

`/screens/pair` is drawn for the caller. Its actions go to `POST /screens/pair/action`.

| The caller | The screen shows | Actions |
|---|---|---|
| no cookie | the server's name; a `form` with a six-digit `code`; `Ask to pair`; the `device-key` island, mode `signin` | `code`, `ask`, `signin` |
| a claim cookie, waiting | the check word; a line that counts down the 5 minutes; how to approve (`a paired device`, or `cooee pair approve`) | `cancel` |
| a claim cookie, approved | who approved; `Continue` | `collect` |
| a claim cookie, declined or run out | who said no, or that it ran out; `Ask again` | `ask` |
| a device cookie | `Paired with <host>`; the island, mode `enroll`; `Open cooee`, which opens `/screens/sessions` | `enroll` |

```rust
// cooee-core: the one new read. `collect` stays the only call that makes a token.
pub enum Peek { Waiting { word: String, until: u64 }, Approved { by: String }, Declined { by: String }, Gone }
impl Devices { pub fn peek(&self, claim: &str, now: u64) -> io::Result<Peek>; }

// pairing.rs: moved out of the `POST /pair` handler, which calls it too, so both doors share one count of misses.
pub(crate) enum Tried { Paired(String), Wrong { left: usize }, Expired, Limited { wait: u64 } }
pub(crate) async fn try_code(cx: &Cx, code: &str) -> Result<Tried>;
```

- `ask` sets `cooee_claim=<claim>; Max-Age=300; HttpOnly; SameSite=Strict`. With it the caller may hold one stream
  whose only key is `/screens/pair`, rebuilt every second, so a yes from a phone or from `cooee pair approve`
  shows at once.
- `code`, `collect` and `signin` answer with the device cookie (as `POST /pair`, `/pair/claim` and
  `/passkey/signin` set it) and `open: "/screens/sessions"`; `collect` also clears the claim cookie and ends the
  claim's stream with `bye handover`.

## The device key: island `device-key`

The one piece of pairing a screen cannot do: sign with a key that never leaves the device. It is what
`apple/Sources/CooeeAPI/Passkey.swift` does today, named as a contract so a desktop client can do the same.

```json
{ "type": "island", "name": "device-key", "id": "key",
  "props": { "mode": "signin", "send": "signin", "values": {} },
  "events": ["signin"],
  "fallback": { "title": "This app keeps no device key", "subtitle": "Pair with the code instead" } }
```

The client's part, on a tap:

1. Its key is P-256, made once per server, the private half in the OS key store behind the OS's own check of the
   user (Touch ID, Windows Hello, the login keyring). Its id is the base64url SHA-256 of the public point.
2. `POST /passkey/challenge` gives `{challenge, ids}`.
3. Mode `enroll`, or a paired caller whose id is not in `ids`: `POST /passkeys` with the key and a
   `webauthn.create` ceremony, then a new challenge.
4. It signs `auth || SHA-256(client)` (ES256, DER): `client` is `{"type":"webauthn.get","challenge","origin"}`,
   `auth` is SHA-256 of the host, the flags byte `0x05` and four zero bytes.
5. It sends the event `send` names with `{id, client, auth, sig}` over `values`. Mode `enroll` sends nothing more.

The server's side is the code that exists: `/passkey/challenge`, `POST /passkeys`, `pairing::assert`,
`Devices::restore`. A client with no such island draws the fallback, so it can never approve a proposal or sign in
by a tap alone.

## What the switch issues may delete

Decided by those issues, not here. Candidates once the iPhone and Mac apps draw these screens: `/home/events`,
`/screens/home`'s `threads` and `personas`, `/screens/pairing` and its bundled `pairing.json`, `/pair/claim` and
`/pair/cancel`. Staying whatever the apps do: the routes `cooee send` calls (`/threads/{id}/messages`, `/send`,
`/events`), `/threads/{id}/files/{name}` and `/frames/{name}`, `POST /pair` (the QR link), the passkey routes.
