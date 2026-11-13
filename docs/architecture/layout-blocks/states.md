# Statecharts: the layout, a screen, reset (#300)

## The layout on the server (`cooee-server::layout`)

```text
 start ─► read layout.json
            │ missing            ─► Default            (served; etag "default")
            │ parses + validates ─► Good(etag)         (served)
            │ fails              ─► Broken(Default)    (Default served; `broken` fanned out on first subscribe)
                                       │
 poll (1 s, fingerprint as themes.rs) │ file changed
                                       ▼
            parse + validate ─► ok   ─► Good(new etag)  ─► SSE `changed {etag}`
                            ─► fail ─► Broken(last)     ─► SSE `broken {line, col, why}`; the served layout stays `last`
 POST /layout/reset ─► write default (keep .bak) ─► Good(default etag) ─► `changed`
```

`last` is whatever was served before the bad write: the previous Good or Default. The server never serves a
layout it couldn't validate; the app never sees a broken one. Validation is the same code the app runs
(`Layout` rules) ported to Rust, so a file the server accepts, the app accepts.

## The layout in the app (`LayoutFeature`)

```text
            launch
              │  LayoutStore.load() ─► lastGood (on disk) else Layout.bundled
              ▼
        Drawing(lastGood)  ────────────────────────────────────────────────────┐
              │ task: GET /layout (If-None-Match: lastGood.etag)               │
              ├─ 304                 ─► no change                               │
              ├─ 200 {etag, layout}  ─► Layout.init ok   ─► Drawing(new); LayoutStore.save(new)
              │                      ─► Layout.init fail ─► Drawing(lastGood) + banner("app can't read this
              │                                               layout: <why> · last good layout kept")
              ├─ error (no server)   ─► Drawing(lastGood) (the offline line is per screen, below)
              │
              │ subscribe /layout/events
              ├─ `changed {etag}`    ─► GET /layout again (above)
              ├─ `broken {why}`      ─► Drawing(lastGood) + banner("layout.json line 12: <why> · last good
              │                                                     layout kept")   [banner clears on `changed`]
              │
              │ reset-layout screen's Reset ─► POST /layout/reset
              │     ok            ─► Drawing(answer); LayoutStore.save; banner cleared
              │     unreachable   ─► Drawing(Layout.bundled); LayoutStore.save(bundled); banner("reset here;
              │                       the Mac's layout.json is unchanged until it's back")
              ▼
   Drawing(x) re-resolves on size-class change (iPad split / rotation): resolved(for:) is pure; the
   ScreenFeatures keep their state, only places change (a sheet on compact becomes a panel on wide).
```

Invariant: `state.layout` is always a `Layout` that decoded, and `LayoutStore` holds the last one that did.
"The app keeps the last good layout when an edit is broken" is the server's `Broken(last)` plus the app's
`Drawing(lastGood)`; both stand alone (an app on a stale cache, a server with a fresh file).

## A screen (`ScreenFeature`, today's `PluginScreenFeature` over `Source`)

```text
   placed (visible) ─► Loading ─► GET source:path
                         │ 2xx ─► Live(screen)  ─► keep(KeptScreens[source|path]); if `stream`: follow
                         │          │ event {ops} ─► Live(applied); keep
                         │          │ act ─► Acting ─► answer ops / {open} ─► Live / navigate
                         │          │ stream ends or errors ─► wait 5 s ─► GET again
                         │ fail ─► Offline(kept?)   kept: shows it with `as of 5m ago`, actions greyed
                         │                          none: the bundled default screen for core paths
                         │                                (CooeeUI/Default/screens/<path>.json), else "Nothing kept yet"
                         ▼
   unplaced (layout changed, node hidden) ─► cancel follow; state dropped; KeptScreens stays
```

Core screens differ from a plugin's only in `Source`: the URL prefix, the poster, and that the bundled default
answers when nothing is kept. That is why "the first draw is instant and screens work offline": home, menu and
settings have a kept or bundled answer before any request.

## Pairing and reset: always reachable

```text
 server == nil ─► AppFeature shows PairingFeature (root `main` = core:/screens/pairing, locked), the layout
                  is irrelevant; after `paired` ─► Drawing(lastGood) as above.
 server != nil ─► the menu-host's last row is reset-layout (locked); it opens even when the layout fetch fails,
                  because the row comes from the Resolved tree the app already holds.
```
