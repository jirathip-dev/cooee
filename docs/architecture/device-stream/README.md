# device-stream: the app follows a plugin's screens on one stream, with blocks' `Replica` (#631, blocks C5)

## Summary

Decision: the app keeps one live connection per plugin, not one per screen. blocks' `Replica` does the bookkeeping,
cooee's server passes the connection straight through, and after a drop the app gets only what it missed.
Slices: 1 the server passes the stream through; 2 the app's follower and stream client; 3 screens, chips and cards
move to it, and the old client and `LiveScreen`'s op code go; 4 (own issue, later) the server's old streams go.
Open questions: 1. drover counts every cooee device as one device, so 4 can be live at once: accept (recommended)?
2. A third-party plugin must serve the new stream to stay live, with no bridge for the old one: OK (recommended)?

## Read in this order

| File | What |
|---|---|
| `callers.md` | how a screen, a chip, the cards, the live client, the server routes and a small plugin read afterwards |
| `adr.md` | the question, what the code does today, the options, the decision, consequences, slices, `arch.toml` lines |
| `shapes.md` | the routes, the Swift types, what `LiveScreen` keeps, the numbers |
| `boundaries.md` | which module may call which, and the stream's statechart |

## The design on one screen

```
a plugin's backend (drover)        cooee-server                      the app, per server and source
GET  /stream            <────────  GET  /p/<plugin>/stream   <─────  ScreenStream (CooeeAPI): the socket, the POSTs,
POST /stream/<sid>      <────────  POST /p/<plugin>/stream/<sid>       the retry clock
                                   passes both through; checks       Follower (CooeeCore): blocks' Replica and the
                                   each opened key is a declared       statechart; whole screens out, per key
                                   screen; keeps no state            PluginsClient.follow (CooeeFeatures)
                                                                       ScreenFeature · header chips · chat cards
```

## Found while reading, for the owner

- **#615 merged in the old framing.** It shares one upstream per-screen `{seq, ops}` stream among devices
  (`plugin/feed.rs`); cooee-server holds no `Shelf`. This ADR does not need one: the server only passes the device
  stream through, and slice 4 deletes `feed.rs` with the streams it shares.
- **The real blocker is drover's device stream.** The issue names #1318 slice 4 (the pin). The app needs
  `GET /stream` and `POST /stream/<sid>`, which is screen-hub slice 5; drover staging (`4e2d5a2fa`) has neither it nor
  the hub. Slices 1 and 2 build and test without drover (blocks' wire types, the `hello` example plugin); slice 3
  switches the app and must wait for it.
- **blocks is ready at cooee's pin.** `d0882093` already carries `Replica` on the B2 protocol (`hello`, `have`,
  `frame`, `synced`); no pin move is needed for this issue.
- **Order after C4 (#620).** `Replica` holds typed `Block`s. Until #620 the views read JSON, so `LiveScreen` would
  convert each screen back. Recommended: `drover queue cooee block --on 620 631`.

## What this needs from drover's slice 5

1. A reply and a frame name a key exactly as the client opened it (the app opens what the screen's `stream` field
   names, less its `/stream` segment).
2. The cards key's screen is a `Block` whose `blocks` are the cards: `Frame.screen` is typed, a bare list does not
   fit it.
3. The place's count stays the screen's own `badge` field: the diff never writes a `badge` op.
