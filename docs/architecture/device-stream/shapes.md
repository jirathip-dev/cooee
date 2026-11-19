# device-stream: the shapes (#631)

The wire types are blocks' (`Hello`, `Frame`, `Bye`, `Sync`, `Have`, `Synced`, `KeyState`, `Op`, `Block`, `Answer`,
`Outdated`), generated into `Blocks` for Swift and in the crate for Rust. cooee defines none of them again.

## The routes, on cooee-server (slice 1)

```
GET /p/{plugin}/stream                    the backend's GET /stream, as sent: SSE with its event names and comments
  event: hello
  data: {"v":1,"sid":"<id>","epoch":"<id>","keys":16}
  data: {"k":"/screens/queue","from":7,"rev":8,"ops":[…]}      a delta: applies only on rev 7
  data: {"k":"/screens/cards","rev":3,"screen":{…}}            no `from`: the whole screen
  event: bye
  data: {"why":"evicted"}                                       evicted | unpaired | handover

POST /p/{plugin}/stream/{sid}             body: blocks' Sync; the backend's POST /stream/{sid}
  {"open":[{"k":"/screens/queue","rev":7},{"k":"/screens/cards"}],"close":["/screens/lanes"]}
  200 {"keys":[{"k":"/screens/queue","rev":8,"ops":[…]},       the missed ops
               {"k":"/screens/cards","rev":3,"screen":{…}},    the whole screen
               {"k":"…","rev":7},                              current
               {"k":"…","gone":404}]}                          404 · 401 · 429, from the backend
  a key not under one of the plugin's places' screens: not sent on, answered {"k":"…","gone":403} by cooee
  404 as the backend sent it: no such stream, reconnect

both: 404 no installed plugin with places has this name · 502 the backend can't be reached
```

- A key is checked as `GET /p/{plugin}/screens/{path}` checks a path today: under a declared place's `screen`, no
  `..`, no `%2e`. The query is not part of the check.
- One backend connection per app connection. A `sid` is one device's stream, so `plugin::share` is not used.
- The stream holds no place among the backend's calls and ends when the backend is silent for 60 s
  (`plugin::events`, as now). The app then reconnects and resumes by rev.
- Core's stream, when cooee's own screens go live, is the same pair with no prefix: `GET /stream`,
  `POST /stream/{sid}`, published through blocks' `Shelf`. Not built here.

## What a feature hears (CooeeCore, `Models/Followed.swift`)

```swift
/// What following a key says: its screen as of the last frame, or why none is coming for now.
public enum Followed: Sendable, Equatable {
    case screen(Block)        // whole, already patched; also said again when the link is back
    case lost(Lost)           // nothing more until the next `.screen`
}

public enum Lost: Sendable, Equatable {
    case unreachable          // no answer from the server; the client keeps trying
    case refused(Int)         // the server answered 5xx for the stream (502: the plugin is down); it keeps trying
    case parked               // `bye evicted`; back when the app is next active
    case outdated(Outdated)   // the wire is over one version away: `.app` or `.server` needs an update
    case gone(Int)            // this key (404, 401, 403, 429) or the stream itself (4xx); the following ends
}
```

## The pure half (CooeeCore, `Follower.swift`)

```swift
/// The device stream's client rules around blocks' `Replica`: what to do for each thing heard.
public struct Follower: Sendable, Equatable {
    public enum Heard: Sendable {
        case mount(String)                    // a key's first follower
        case unmount(String)                  // its last follower left
        case event(name: String, data: Data)  // an SSE event: `hello`, `message` (a frame), `bye`
        case synced(Data)                     // the 200 body of the POST in flight
        case refused(Int)                     // the GET or the POST answered this status
        case fell                             // the socket ended, or failed with no status
        case tick                             // the wait asked for is over
        case wake                             // the app is active again
    }

    public enum Step: Sendable, Equatable {
        case connect                          // GET <prefix>/stream
        case hangUp                           // drop the socket
        case post(sid: String, body: Data)    // POST <prefix>/stream/<sid>, an encoded `Sync`
        case wait(Duration)                   // then `tick`
        case tell(String, Followed)           // to every follower of the key
        case end(String)                      // finish the key's followings
    }

    public init()
    public mutating func heard(_ heard: Heard) -> [Step]
}
```

- It holds the `Replica`, the link (`boundaries.md`'s statechart), the `sid`, the `Have`s of the POST in flight and
  the keys to open or close next. It reads no clock and opens no socket.
- One POST is in flight at a time. A mount, an unmount or a gap during it goes in the next one.
- It decodes at the edge: an event or a reply that does not decode is `fell`.
- CooeeCore has its own `Sync` (the sync dot's); the wire's is written `Blocks.Sync` here.

## The I/O half (CooeeAPI, `ScreenStream.swift`)

```swift
/// One source's device stream: the socket, the sync POSTs and the retry clock around a `Follower`.
public actor ScreenStream {
    public nonisolated func follow(_ key: String) -> AsyncStream<Followed>
}

/// One `ScreenStream` per server and source, as `Clients` keeps one `Client` per server.
public final class ScreenStreams: Sendable {
    public init(clients: Clients, wakes: @escaping @Sendable () -> AsyncStream<Void>)
    public func callAsFunction(_ server: Server, _ source: LayoutSource) -> ScreenStream
}
```

- `follow` mounts the key while its stream is consumed and unmounts it when the consumer ends. It buffers the
  newest value only, so a burst of frames reaches a busy reducer as one screen.
- A second follower of a mounted key is told the held screen at once.
- It imports no `Blocks`: frames go to the `Follower` as bytes, steps come back.

## What `LiveScreen` keeps (CooeeCore, `Screen.swift`)

```swift
/// A screen as last heard, from its GET or its stream.
public struct LiveScreen: Sendable, Equatable {
    public private(set) var root: Block
    public init(_ root: Block)
    /// The place's count: the screen's own `badge`.
    public var count: Int? { get }
    /// Its key on its source's stream: `stream` less a trailing `/stream` segment; nil when it is not live.
    public var key: String? { get }
    /// Shows an action's answer at once; ops that do not fit leave the screen as it was.
    public mutating func answered(_ ops: [Op])
}
```

| Goes | Why |
|---|---|
| `seq`, `dot` | nothing reads them; the rev is `Replica`'s |
| `apply(event:)`, `apply(_:)`, `patch`, `edit` | blocks' `apply`, through `Replica` and `answered` |
| `JSONValue.editing`, `.merging`, `.inserting` | the same |
| `count` as a stored field fed by a `badge` op | the screen's `badge` field |

If #620 has not landed, `root` stays the JSON the views read and `init(_: Block)` converts; #620 then makes it `Block`.

## The Features client (`PluginClients.swift`)

```swift
public struct PluginsClient: Sendable {
    public var follow:                        // replaces `events`
        @Sendable (_ server: Server, _ source: LayoutSource, _ key: String) -> AsyncStream<Followed>
    public var act:                           // a typed `Answer` (ops, toast, open), decoded in CooeeAPI
        @Sendable (_ server: Server, _ source: LayoutSource, _ screen: String, _ action: BlockAction) async throws
            -> Answer
    // places, installed, screen, attach: as they are
}
```

## The numbers

| What | Value | From |
|---|---|---|
| waits between reconnects | 1 s, 2 s, then 5 s; back to 1 s after a sync | drover's page |
| a stream with no key mounted | closed after 30 s | drover's page (hidden for 30 s) |
| a follower's buffer | the newest value | |
| keys per stream | `hello.keys` (drover: 16); one more is `gone: 429` | the backend |
| streams per device | drover: 4, and every cooee device is the plugin | the backend |
| a silent backend stream | closed by cooee-server after 60 s | `plugin::IDLE`, unchanged |
