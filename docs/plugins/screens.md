# Screens

A plugin can instead (or as well) answer its screens as JSON made of cooee's blocks, which the native app draws: no
frontend code (the spec: [design/screens-as-data](../design/screens-as-data/README.md)). Its manifest says where they go
and what it may do:

```toml
claims = ["drover"]               # the workflow sources it owns (#229): those sessions show in its place, not as chats

[[places]]
type = "header-strip"             # a chip under a persona's chat header; or "menu-row" (the avatar menu), "chat-cards"
id = "queue"
persona = "conductor"             # the persona it attaches to by default
label = "Queue"
screen = "/screens/queue"         # a path on its backend, under /screens/

[permissions]                     # shown when it's attached to a persona; cooee refuses the rest
places = ["header-strip"]
actions = ["drive", "drop"]       # what its screens may POST
destructive = ["drop"]            # always confirmed first by the app
push = false
network = []
```

cooee answers `GET /p/<plugin>/screens/...` from the backend's `GET /screens/...` (a screen, or its event stream of
`{seq, ops}`) and passes `POST /p/<plugin>/action` to its `POST /action` with `{screen, block, action, values, by}`.
An action not in `actions` gets a 403 and never reaches the backend. A plugin with screens only has no tab.
A `chat-cards` place's screen (default `/screens/cards`) answers a list of `card` blocks: the app follows its
`/stream` and draws them at the end of that persona's chats, keyed by card id; a tap posts `screen: "cards"` with the
tapped choice or run block's `id` as `block`. A 2xx settles the card to `✓ <toast>` until the plugin changes it, a 409
to its line; the persona's first chip of that plugin counts the open cards (#377).
These cards, a plugin row that waits in a chat and cooee's own unanswered approvals sit in the chat's **Asks** sheet
(#852), never in the thread: its `Asks N` chip and one `N need you` line above the composer count them and hide at 0.
A row names a `group`; the manifest lists them in order (`[[groups]]` with `id`, `label` and `urgent = true` for the
ones that sort first and turn the chip red). cooee's own come after the urgent ones: `Approvals`, then `Check-in`.
`docs/plugins/hello-py/` is a whole one: `hello.py`, a live screen in about 30 lines of Python, no dependencies.
