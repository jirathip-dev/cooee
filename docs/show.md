# Personas answer with blocks (#776)

## Today

- **Blocks in chat:** a persona with a catalog may answer with blocks (a form, a choice, a chart) by calling cooee's
  `show` tool; the owner's answer is its next turn. The ADR is
  [architecture/persona-blocks](architecture/persona-blocks/adr.md).

## How it works

- **The catalog.** A persona's `blocks` key (`agent.toml`, `/personas`) names one of cooee's two catalogs:
  `show` (prose, lists, rows, stats, tables, steps, charts; sends nothing) or `ask` (those, and cards, buttons
  (`pick`), choices (`answer`) and forms (`submit`) with text, toggle and picker inputs). Links may start with
  `https://`; no file is loaded and no secret is asked. None: text only, no `show` tool, the brief unchanged. A name
  that is no catalog is a 400. The catalogs are built into cooee; the owner picks one, never writes one.
- **The prompt.** A session's brief ends with three lines about `show`, then blocks' `model::prompt` for the
  catalog, whole. A changed key reaches the next session.
- **The tool.** `show`'s arguments are the reply, `{"blocks": […]}`, read with blocks' `model::read` under the
  catalog before anything is stored. It answers `Shown.`; a refusal says what to fix, once a turn; the next refused
  call stores what the reply said as plain text (`Shown as plain text…`); after that nothing in the turn is read
  (`NO_RETRY`). A turn that ends after one refusal also stores its plain text. In a room it answers `NOT_HERE`, with
  no catalog `NO_CATALOG`.
- **In the chat** the reply is an item `{"kind": "blocks", "blocks": […], "sent"?}`, its blocks' ids `b<item>.<n>`;
  the chat screen draws them in the agent's message, then `✓ <label> · <device>` once answered. The apps draw them
  once they draw the chat from the server's screen (#716).
- **An answer** is an action on a `b<item>.<n>` block, sent to `POST /screens/action` by a paired device. It never
  reaches the chat's own actions. It passes blocks' `own` and `catalog.sends`, then cooee's check: the block names the
  action; a button sends its own values, a choice one of its options (or `other` text it offers), a form only its
  fields, each of its type (text at most 2,000 characters); 8,000 bytes at most. A refused one is a 400 and nothing is
  queued. A reply takes one answer; the same one again does nothing. The persona gets
  `[The owner answered your blocks]: <action>` and the values as one line of JSON as its next turn, through the
  queue; unqueueing it opens the reply again.
