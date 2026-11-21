# The device stream (#714, #718)

One live stream per device carries every core screen it shows, in blocks' own protocol (the frames blocks'
`Mirror`, `Replica` and `stream.ts` follow). The design is [the chat-screens ADR](architecture/chat-screens/adr.md).

| Call | Answer |
|---|---|
| `GET /stream` (`?v=` or `Accept: application/vnd.blocks+json;v=`) | SSE: `hello` (`v`, `sid`, `epoch`, `keys`), then a `message` per frame, `bye` when the server ends it |
| `POST /stream/{sid}` | blocks' `Sync` (`open` with the rev held, `close`) in, `Synced` out: per key nothing, the ring's ops, the whole screen or `gone` |

- A key is a screen's path with its query, as `GET` takes it. Today: `/screens/sessions`,
  `/screens/sessions?archived=1` and `/screens/chat/{id}` ([the chat screen](chat.md#the-chat-screen-714-719)). Any
  other key is `gone: 404`.
- `live.rs` keeps a key while a stream holds it: a change of the thread list (a chat's key: of its thread) wakes
  it, it is built by the same function as its `GET`, and blocks' `Shelf` diffs it into one frame for every holder. A key is built at most 20
  times a second, and after a build that took `t` it waits `10 t`. A key nobody holds is not built; it is
  forgotten 5 minutes after its last holder left.
- `epoch` is new each time the server starts, so a reconnect after a restart gets every key whole. A device holds
  at most 4 streams (the oldest gets `bye evicted`), a stream 16 keys, and a stream over 1 MB behind gets
  `bye evicted` and syncs again.
- The sessions screen: one `section` per persona with chats, in `GET /personas` order, then `Other chats`. A row
  `t-<thread>` per chat of `GET /threads` (its title, its last line toned by its state, its unseen count), and a
  `New chat` row per persona.
