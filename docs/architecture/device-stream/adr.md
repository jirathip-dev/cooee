# ADR: the app follows a plugin's screens on one device stream, with blocks' `Replica` (#631, blocks C5)

## Summary

Decision: the app keeps one live connection per plugin, not one per screen. blocks' `Replica` does the bookkeeping,
cooee's server passes the connection straight through, and after a drop the app gets only what it missed.
Slices: 1 the server passes the stream through; 2 the app's follower and stream client; 3 screens, chips and cards
move to it, and the old client and `LiveScreen`'s op code go; 4 (own issue, later) the server's old streams go.
Open questions: 1. drover counts every cooee device as one device, so 4 can be live at once: accept (recommended)?
2. A third-party plugin must serve the new stream to stay live, with no bridge for the old one: OK (recommended)?

## Question

blocks' ADR left "cooee's own stream routes" open, and its migration plan asks for this ADR before C5 is queued.
Four things to settle: the app's stream client, reconnect and resume, what `LiveScreen` keeps, and how the old
per-screen streams are retired.

## What the code does today

1. **One socket per screen.** `ScreenFeature` GETs a screen, reads its `stream` field and follows that URL
   (`ScreenOpening.swift`). Each header chip and each `chat-cards` place follows `<screen>/stream` too
   (`ChatDeck.swift`). Every one of them has its own loop: on any end, wait 5 s, GET the whole screen, subscribe again.
2. **cooee has its own op code.** `LiveScreen` (`CooeeCore/Screen.swift`) applies `{seq, ops}` events to untyped
   JSON: `replace`, `merge`, `remove`, `insert`, `screen`, `badge`, `dot`. It has no `move`, drops an unknown op
   silently and cannot tell that it missed an event. An action's answer goes through the same code.
3. **The server passes screens through.** `GET /p/<plugin>/screens/<path>` answers the backend's JSON, or its SSE
   stream. Since #615 one backend stream per screen is shared among devices (`plugin/feed.rs`).
4. **blocks already has the client rules.** `Replica` at cooee's pin holds, per key, the rev and the typed screen,
   and answers each frame with patched, replaced, old or gap. It is pure: it opens no socket.
5. **The protocol** (blocks `docs/schema/stream.md`, drover `screen-hub/shapes.md`): `GET /stream` starts with
   `hello` (version, stream id, epoch, key limit). The client answers with `POST /stream/<sid>` naming the keys it
   opens and the rev it holds of each. Per key the reply is nothing, the missed ops or the whole screen. A frame
   applies only on the rev it names. The server keeps nothing about a stream once its socket closes.
6. **The count is a field.** On the device stream the place's count is the screen's `badge` field; the diff never
   writes `badge` or `dot` ops. Nothing in the app reads `LiveScreen.dot` or `.seq`.

## Options

| | Choice | For | Against |
|---|---|---|---|
| Server A | **Pass through: `/p/<plugin>/stream` and `/p/<plugin>/stream/<sid>`** | About 70 lines; no state in cooee-server, so its restart costs nothing; resume is the backend's | Every cooee device is one of the plugin's streams (question 1) |
| Server B | cooee-server follows each plugin once and republishes from its own `Shelf` | One upstream stream however many devices; what #615's title meant | A second hub, a mirror, key ref-counts and a second diff, for a fan-out drover's hub already does |
| Scope A | **One stream per server and source** | The protocol's own unit; the same client serves core at `/stream` the day cooee's own screens go live | A persona with three plugins holds three sockets |
| Scope B | One stream per server for everything | One socket | Needs Server B |
| Replica A | **Inside the client; features get whole screens** | Features keep their shape; a burst is one state change | The held screens sit outside reducers, as a socket's bookkeeping |
| Replica B | In a root reducer; each frame an action | All state in TCA | Every screen, chip and deck would read shared state; every frame crosses the root store |
| `LiveScreen` A | **Keep it, without op code** | Three callers keep their type | A thin type |
| `LiveScreen` B | Delete it, hold a bare `Block` | One type fewer | The count and the key rule would be written three times |
| Old plugins A | **The plugin contract's live channel becomes the device stream** | One way; a whole-screen plugin needs about 10 more lines | Third-party plugins must change (question 2) |
| Old plugins B | cooee-server turns `{seq, ops}` into frames | No plugin changes | A translator and two contracts to keep, forever |

## Decision

1. **The route.** A source's stream is at `<prefix>/stream`, its sync at `<prefix>/stream/<sid>`, where the prefix
   is `/p/<plugin>` for a plugin and empty for core. cooee-server passes a plugin's two routes to its backend's
   `GET /stream` and `POST /stream/<sid>`, one backend connection per app connection, never shared. It checks one
   thing at the edge: a key in `open` that is not under one of the plugin's declared screens is not sent on and is
   answered `gone: 403`. Core has no stream yet; no core screen names one, so the app never asks.
2. **The client, in two halves.** `Follower` (CooeeCore) is pure: it holds the `Replica`, the mounted keys and the
   statechart, takes what was heard and returns what to do. `ScreenStream` (CooeeAPI) is an actor per server and
   source: the socket, the POSTs, the clock. Only `Follower` touches blocks' wire types; `ScreenStream` moves bytes.
3. **A key.** A live screen's key is its `stream` field less a trailing `/stream` path segment, query kept. A
   screen with no `stream` is not live, as today. Followers of one key share one mount: the key closes when the
   last one leaves, and a second follower gets the held screen at once (a chip and its opened screen share a key).
4. **What a feature gets.** `PluginsClient.follow(server, source, key)` replaces `events`. It yields whole
   screens, already patched, and one `lost` when the link is down. It stays open across reconnects. A feature never
   sees an op, a rev or a socket, and never reconnects: `ScreenFeature` retries only its first GET, as today.
5. **Reconnect.** The socket falls: wait 1 s, 2 s, then 5 s each time; at once when a key is mounted or the app
   becomes active. A feature hears `lost` only when a reconnect fails, so a blip shows nothing. `bye evicted` parks
   the stream until the app is next active. A 4xx for the stream ends its followings: the screens stay as loaded.
6. **Resume.** The `Replica` outlives the socket. After `hello` the client POSTs every mounted key, with its rev
   when the epoch is the one held. A missed frame (a gap) or an op the app does not know reopens that one key. A
   restart of cooee-server changes nothing here: the epoch is the backend's. A restart of the app starts with no
   rev: the first paint is the GET or the kept copy, as today, and the sync brings the screen.
7. **`LiveScreen` keeps** the screen as last heard (`root`), the place's count read from its `badge` field, the
   key rule, and `answered`: an action's answer applied at once through blocks' `apply`. It loses `seq`, `dot`,
   both `apply`s, `patch`, `edit` and the three JSON helpers. The stream is the truth: the key's next frame
   replaces what `answered` showed.
8. **Retiring the old streams, in order.** (a) #631: the app's per-screen client and op code go in the PR that adopts
   the stream; the server still serves the old streams, so an older app build keeps working. (b) A follow-up cooee
   issue: `GET /p/<plugin>/screens/<path>` answers JSON only, `plugin/feed.rs` goes, `docs/plugins.md` names the
   device stream as the live channel, the examples drop their old stream. (c) drover's D5 deletes its route and
   `wire::old`.

## Consequences

- A fallen socket or a restart of cooee-server costs a 1 s wait and the missed ops, not 5 s and a whole screen.
- A missed update is seen (a gap) and repaired. Rows that change place arrive as a `move`.
- drover gives a device 4 streams and sees every cooee device as one device, the plugin. A fifth live cooee device
  evicts the oldest, which parks until it is next active. If that bites, the small fix is on both sides: cooee
  sends its device id on the stream GET and drover counts by it. Server B is the large fix.
- A plugin with a live screen or a `chat-cards` place serves the device stream. Whole-screen frames are enough
  (`callers.md`). Without it its screens still load and its actions still work; they do not update by themselves,
  and its cards do not show.
- A screen opened for the first time is sent twice: the GET, then the sync's whole screen. `Replica` cannot be
  seeded from a GET today. drover's ETag join is a later saving, in blocks.
- `Replica` holds typed blocks: a field of a core kind that blocks' schema does not know may not survive it (an
  app's own kind is kept whole). drover's screens pass blocks' check from screen-hub slice 4.
- A header chip mounts its screen's key for the count alone, and so takes one of the stream's 16 keys.
- Between (a) and (b) the server serves both streams. After (b) an app build older than #631 shows its offline
  banner on live screens until it is updated.
- No design frame changes. The banner is the one there is. Frames that build a screen from a stream fixture apply
  its ops through `answered`; their PNGs must not change.
- Lines, estimated: #631 `src/` net about +300 (the server about +70, Swift about +230); the follow-up gives back
  about 230.

## Slices

Slices 1 to 3 are #631's PR, in this order; slice 4 is its own issue.

1. **The server passes the stream through.** The two routes in `routes/plugins.rs`, the key check, the two rows in
   `docs/plugins.md`; `docs/plugins/hello` and `hello-py` serve the device stream beside their old one. Tests:
   `crates/cooee/tests/plugins.rs` against `hello`.
2. **The follower and the stream client.** `Followed`, `Follower`, `ScreenStream`, `ScreenStreams`. Tests:
   `FollowerTests` plays the statechart; `ScreenStreamTests` runs canned SSE and POSTs.
3. **The callers move, the old way goes** (needs drover's slice 5 deployed). `PluginsClient.follow` replaces
   `events`; `act` answers a typed `Answer`; `ScreenFeature`, `ChatDeck` and `ChatPlugins` move; `LiveScreen`'s op
   code, `Client.screenEvents`, the chips' and cards' 5 s loop and the cards' fallback GET go; the bans below are added.
4. **The server's old streams go** (after #631's app build is on the owner's devices). Decision 8 (b). Then D5.

## `.drover/arch.toml`

No hot file is converted. New files go in folders that are already collected (`CooeeCore/Models/`,
`CooeeApp/Live/`). Slice 3 adds these bans:

```toml
[[ban]]
pattern = "Replica"
paths = ["apple/Sources/**"]
except = ["apple/Sources/CooeeCore/Follower.swift"]
reason = "#631: only the follower holds blocks' Replica; a feature gets whole screens from PluginsClient.follow"

[[ban]]
pattern = "/stream"
paths = ["apple/Sources/**"]
except = ["apple/Sources/CooeeAPI/ScreenStream.swift", "apple/Sources/CooeeCore/Screen.swift"]
reason = "#631: one device stream per source: only ScreenStream builds its URL, only LiveScreen reads a key from one"

[[ban]]
pattern = "[\"op\"]"
paths = ["apple/Sources/**"]
except = []
reason = "#631: the app has no op code of its own: blocks' apply runs an op, through Replica or LiveScreen.answered"
```
