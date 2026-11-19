# device-stream: how the callers read (#631)

Sketches, not final code. Each is the shape the impl lane writes to.

## A screen (`ScreenOpening.swift`)

The GET is retried every 5 s until it answers, as today. Once a key is followed the stream reconnects itself:
no second GET, no sleep.

```swift
internal func watch(_ state: State) -> Effect<Action> {
    let (plugins, cache, clock, server, source) = (plugins, cache, clock, state.server, state.source)
    let (path, key) = (state.current, state.key)
    return .run { send in
        while true {
            do {
                let screen = LiveScreen(try await plugins.screen(server, source, path))
                await send(.loaded(screen))
                guard let live = screen.key else { return }
                for await heard in plugins.follow(server, source, live) { await send(.heard(heard)) }
                return
            } catch {
                await send(.failed(Self.who(error, source: source), cache.load(key)))
            }
            try await clock.sleep(for: .seconds(5))
        }
    }
    .cancellable(id: CancelID.follow, cancelInFlight: true)
}
```

```swift
case .heard(.screen(let block)):
    state.screen = LiveScreen(block)
    return keep(&state)                       // clears `unreachable`, saves the kept copy after 2 s, as today
case .heard(.lost(let lost)):
    state.unreachable = Self.line(lost, source: state.source)
```

| `Lost` | The banner's `who` (the banner and its Retry are the ones there are) |
|---|---|
| `.unreachable` | `Mac unreachable` |
| `.refused` | `<source> isn't answering` |
| `.outdated(.app)` | `cooee needs an update` |
| `.outdated(.server)` | `<source> needs an update` |
| `.parked`, `.gone` | none: the screen stays as it is |

Retry sends `.retry` as today: the GET again, then `follow`, whose mount makes a waiting stream connect at once.

## An action's answer (`ScreenFeature.acted`)

```swift
let was = state.screen?.key
state.said = error ?? answer?.toast
if let ops = answer?.ops { state.screen?.answered(ops) }
if let open = answer?.open { return .merge(next, navigate(&state, to: open)) }
let moved = state.screen?.key != was          // a filter: the answer's screen names another key
return .merge(next, keep(&state), moved ? refollow(state) : .none)
```

`refollow` is `follow` on the new key under the same cancel id, so the old key unmounts and the new one mounts in
one POST.

## Header chips and chat cards (`ChatDeck.swift`, `ChatPlugins.swift`)

```swift
internal static func deal(_ places: [Place], _ plugins: PluginsClient, _ server: Server, _ send: Send<Action>) async {
    await withTaskGroup(of: Void.self) { group in
        for place in places {
            group.addTask {
                for await heard in plugins.follow(server, .plugin(place.plugin), place.screen) {
                    guard case .screen(let block) = heard else { continue }
                    await send(place.cards ? .cards(.heard(place.plugin, block)) : .plugins(.counted(place.id, block)))
                }
            }
        }
    }
}
```

```swift
case .counted(let place, let block):
    state.strips[place] = LiveScreen(block)   // `chipCounts` reads `.count`, as today

internal mutating func heard(_ plugin: String, _ block: Block) {
    screens[plugin] = LiveScreen(block)       // then the card keys, as today
}

internal mutating func played(_ item: Int, _ answer: Answer?, line: String?) {
    screens[keys[index].plugin]?.answered(answer?.ops ?? [])   // then the settled line, as today
}
```

The clock argument, the 5 s loop and the cards' fallback GET go. `PluginDeck` reads the cards from the screen's
`blocks`; its bare-list branch goes.

## The live client (`CooeeApp/Live/PluginsClient+Live.swift`)

```swift
private let streams = ScreenStreams(clients: client, wakes: { becameActive() })

follow: { server, source, key in streams(server, source).follow(key) },
act: { server, source, screen, action in try await client(server).act(source, screen: screen, action) },
```

`Client.screenEvents` goes. Its one other caller, `Client.watch` for the terminal, keeps the same few lines as a
private reader in `Terminal.swift`: it reads the terminal plugin's own events, not screens.

## Inside `ScreenStream`

```swift
private func run(_ heard: Follower.Heard) {
    for step in follower.heard(heard) {
        switch step {
        case .connect: socket = Task { await self.listen() }          // GET; each SSE event back in as `.event`
        case .hangUp: socket?.cancel()
        case .post(let sid, let body): Task { await self.sync(sid, body) }   // back in as `.synced` or `.refused`
        case .wait(let time): timer = Task { try await client.clock.sleep(for: time); await self.run(.tick) }
        case .tell(let key, let said): followers[key]?.values.forEach { $0.yield(said) }
        case .end(let key): followers.removeValue(forKey: key)?.values.forEach { $0.finish() }
        }
    }
}
```

## The server (`routes/plugins.rs`)

```rust
#[route(GET "/p/{name}/stream")]
async fn stream(cx: &Cx) -> Result<Response> {
    let Some((p, m)) = with_screens(cx) else { return plain(404, NO_PLUGIN) };
    let head = [("Accept", "text/event-stream")];
    match blocking(move || plugin::call(&p, "GET", "/stream", &head, &[], Duration::from_secs(5))).await? {
        Ok(r) => pass_on(cx, r, ops, || true).await,          // `ops` keeps `event: hello` and `event: bye`
        Err(e) => plain(502, &format!("{}: {e}", m.name)),
    }
}

#[route(POST "/p/{name}/stream/{sid}")]
async fn stream_sync(cx: &Cx, Json(sync): Json<blocks::Sync>) -> Result<Response> {
    let Some((p, m)) = with_screens(cx) else { return plain(404, NO_PLUGIN) };
    let (open, refused): (Vec<Have>, Vec<Have>) = sync.open.into_iter().partition(|h| declared(&m, &h.k));
    // POST /stream/{sid} with {open, close}; a 200's `keys` gain {k, gone: 403} for each refused; else as sent
}
```

`declared` is the check `screen` has inline today, lifted into a function both use.

## A small plugin (`docs/plugins/hello-py`)

Whole-screen frames are a complete server: no diff, no ring, no per-stream state.

```python
def frame(): return {"k": "/screens/hello", "rev": rev, "screen": screen()}

# GET /stream           event: hello  data: {"v": 1, "sid": "s", "epoch": EPOCH, "keys": 16}
#                       then, on each change:   data: <frame()>
# POST /stream/s        {"keys": [frame()]}                    whatever was asked
```

`EPOCH` is made once per start, so the app drops its revs when the plugin restarts.

## Tests, by slice

| Slice | Test | Holds |
|---|---|---|
| 1 | `plugins.rs`: a device follows `hello` through cooee | `hello`, a frame and `bye` arrive with their event names; the sync's reply comes back |
| 1 | `plugins.rs`: a key outside the plugin's places | answered `gone: 403`, never sent to the backend |
| 2 | `FollowerTests`: reconnect on the same epoch | the sync carries each key's rev; a `current` reply still tells each key its screen |
| 2 | `FollowerTests`: a new epoch; a gap; an unknown op | no rev sent; one key reopened; reopened with no rev |
| 2 | `FollowerTests`: a blip against a failed reconnect | `lost` told only on the failed one, once |
| 2 | `FollowerTests`: evicted, then active again; outdated | parked then connecting; no sync while outdated |
| 2 | `ScreenStreamTests`: two followers of one key | one mount, the second told at once, closed after the last leaves |
| 3 | `ScreenFeatureTests` | a screen follows its key; a `lost` shows the banner and the next screen clears it; a filter moves the key |
| 3 | `ChatPluginsTests`, `CardAnswerTests` | a chip's count follows the screen's `badge`; an answered card settles |
| 3 | `CooeeUITests` Frames and Packs | unchanged PNGs |

blocks runs `fixtures/protocol` against `Replica` itself; cooee does not run them again.
