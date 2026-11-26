# Layout (#300)

## API

The ADR: [architecture/layout-blocks](architecture/layout-blocks/adr.md.

The owner's `layout.json` beside `config.toml`, read every second. The server serves only a layout that passes the
app's rules (`cooee layout check <file>` runs the same ones): with no file, or no good one yet, the app's bundled
default (`apple/Sources/CooeeUI/Default/layout.json`). A bad write changes nothing served. Beside its `place`, a
node may name a `transition`: `push` (the default), `sheet`, `fade` or `none`, how the app brings it on screen.
Two screens are locked (#316): the app always adds `core:/screens/pairing` and `core:/screens/reset-layout`, so a
file that names either path, or hides the root nav, is refused like any bad write and the last good one stays.

| Call | Body | Answer |
|---|---|---|
| `GET /layout` | | `{etag, layout, skipped}` and an `ETag` header; 304 with no body when `If-None-Match` names it. Each enabled plugin's manifest places are merged in (`header-strip` in the chat's `strip`, `chat-cards` in its `cards`, `menu-row` in the nav's `menu`) unless the file places that plugin; a plugin screen its `[permissions] places` doesn't allow there (`cards` needs `chat-cards`, `menu` `menu-row`, any other shown place `header-strip`) is dropped, and `skipped` lists `<json path>: <why>` for each |
| `GET /layout/events` | | SSE: `changed {etag}` after a good write; `broken {line, col, why}` after a bad one, and at once to a new subscriber while the file is broken |
| `POST /layout/reset` | | writes the default to `layout.json`, the old file kept as `layout.json.bak`; answers as `GET /layout`; 409 with no config folder |
| `GET /screens/home` | | core's home (#311): `{type: "screen", source: "core", id, title, blocks, threads, personas}`, the app's bundled `Default/screens/home.json` with `threads` as `GET /threads` and `personas` as `GET /personas`; `source` is always the server's |
| `GET /screens/menu` | | core's avatar menu (#312): the app's bundled `Default/screens/menu.json` with `source: "core"`; the rows that open a server screen carry a `value`: `signins` the signed-in accounts joined `, `, `devices` the paired count, `machines` the hosts joined `, `, `plugins` the installed count, `connectors` the connectors' count, or `1 needs sign-in` (`N need sign-in`) with `warn: true` while an account needs it (#524) |
| `GET /screens/settings/{devices,signins,machines,plugins,connectors}` | | core's settings screens (#312): `{type: "screen", source: "core", id, title, blocks}`, one `list` of `row`s from the data `GET /devices`, the sign-ins sheet, `GET /machines`, `GET /plugins` and `GET /connectors` read (a connector's subtitle: `no sign-in`, `N signed in` or the warned `N needs sign-in`) |
| `GET /screens/plugins/{name}/attach?persona=<id>` | | the attach sheet (#312): screen `attach`, one `card` of kind `permissions` (only core sends it): what installed plugin `name` may do for the persona, its actions (destructive ones always confirmed first) and the hosts its backend reaches; 404 for a plugin or persona that doesn't exist |
| `GET /screens/persona/{id}` | | the persona panel and its edit sheet (#314): the app's bundled `Default/screens/persona.json` with `source: "core"`, `title` the persona's name; blocks `screen`, `terminal`, `workflow` (left empty by core), `routines` (`rows` `{id, name, when, on}`: the routines that post to its chats or start threads as it), `places` (`rows` `{id, plugin, title, open}`: its attached plugins' places), `memory` (`count`, `notes` as `GET /personas/{id}/memory`) and `edit`; 404 for a persona that doesn't exist |
| `GET /screens/pairing` | | core's pairing steps (#316): the app's bundled `Default/screens/pairing.json` with `source: "core"`, one `step` block per step (`welcome`, `scan`, `nearby`, `address`, `waiting`, `paired`), its blocks around the `scanner` component; `{name}` in a text is the pairing's value (`host`, `word`, `count`), a text naming one the app lacks is not drawn |
| `GET /screens/reset-layout` | | the locked reset screen (#316): a text and a `reset` button whose action is `POST /layout/reset`; the app draws it as the avatar menu's last row |
| `POST /screens/action` | `{screen, block, action, values}` | an action on a core screen (#314): on `/screens/persona/<id>`, `save` puts `values` as the persona (as `PUT /personas/{id}`) and answers it, and values naming `mcp` set its connector picks too (#524: attach and detach; a server not in the registry is refused before anything is saved), `clear` forgets its memory and answers the screen again; 400 for another screen or action |
