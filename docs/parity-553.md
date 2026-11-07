# Parity audit: herdsman#553 rows 1–35 against cooee core (#21)

Audited at `51f92e8` (staging). **done**: cooee does it today; **partial**: some of it; **missing**: none of it.
Row numbers are #553's.

| # | feature | cooee today | where | gap to build |
|---|---|---|---|---|
| 1 | Home Screen app, offline paint | built (#21, rows 1–14 PR) | sw.ts network-first copy; chat.ts keeps the newest slice; `touch-action:manipulation` | — |
| 2 | Live stream, sync dot, gapless reconnect, no scroll jump | built (#21, rows 1–14 PR) | chat.ts `sync()` on the stream's open/error/offline | — |
| 3 | Themed loading strip, "as of Xm ago" | built (#21, rows 1–14 PR) | `#ld` (lib.rs `document`), chat.ts `strip()` | — |
| 4 | Short self-clearing toasts | built (#21, rows 1–14 PR) | chat.ts `toast()` on every action | — |
| 5 | Pull to refresh, ↻, reload on resume | built (#21, rows 1–14 PR) | chat.ts `refresh()`: ↻ `#rf`, pull, resume after 30 s, `online` | — |
| 6 | Pair by code approved on host; list and remove devices | built (#21, rows 1–14 PR) | devices.rs `request`/`approve`/`collect`; `/pair/request`, `/claim`, `/cancel`; `cooee pair <code>`; Settings → devices | — |
| 7 | Unpaired devices read-only | built (#21, rows 1–14 PR) | `/pair` as the shell: look sheet, pair hint, composer off; reads stay 401 (q517) | — |
| 8 | Web push: enable, truthful status, test push, themed titles | built (#21, rows 1–14 PR) | chat.ts `alertState()` checks `/push/endpoints`; `/push/test`, `/push/unsubscribe` | — |
| 9 | Push tap opens its thread; no push for open thread | done | `web/ts/sw.ts` `push` (skips a visible client on that thread) and `notificationclick` | — |
| 10 | Themes as data: farm, kitchen, garden, terminal, moments | built (#21, rows 1–14 PR) | chat.ts `moment()` on the thread bar, `html[data-moments]` | — |
| 11 | Colour schemes: presets plus user files | done | `web/src/look.rs` `Looks::load` (12 built-in + `<config>/schemes/*.json`, contrast gate); `look.ts` COLOURS | — |
| 12 | Appearance per phone: theme, colours, pixel or ascii | built (#21, rows 1–14 PR) | look.ts sprites on, ascii or off | — |
| 13 | Settings sheet: section index, deep links, alert dot | built (#21, rows 1–14 PR) | look.ts Settings index, `#settings/<section>`, ⚙ dot | — |
| 14 | Report mode | built (#21, rows 1–14 PR) | chat.ts `reportMode()`, `POST /report`, `cooee reports` | — |
| 15 | Thread list per repo: unread dot, state, last line, badge | built (#43) | `web/src/lib.rs` `row` (● and the unseen count's badge), `chat.ts` `cardsNow` every 5 s, the app badge | — |
| 16 | Start by typing; auto title; retitle | built (#43) | title from the first line (`hub.rs` `start`); the thread bar's title opens rename (`POST /threads/{id}/title`) | — |
| 17 | Swipe to archive, 5 s undo, restore | built (#43) | `chat.ts` swipe on a row, the toast's [undo]; the archived list with [restore] (`POST /threads/{id}/archive`) | — |
| 18 | Message blocks: colours, markdown, system lines, folded tool runs | built (#43) | `web/ts/md.ts` (agent text only; settled blocks while streaming); `chat.ts` `fold()`, `▸ N commands` | — |
| 19 | Fenced code blocks with copy | built (#43) | `md.ts` `code` (language, [copy] on chat.ts's data-copy) | — |
| 20 | Several attachments, thumbnails, image viewer | built (#43) | 📎 in the composer, `files` on send (`<data>/files/<thread>/`), `GET /threads/{id}/files/{name}`, the viewer popover (back closes it) | — |
| 21 | Instant send: sending…/✓, red retry | built (#43) | `chat.ts` `deliver()` (sending… → ✓, not sent · [retry]; kept through a reload of the slice) | — |
| 22 | Growing composer, pinned send, follow-scroll | built (#43) | `chat.ts` `grow()` to 6 lines (`#box` max-height), then it scrolls | — |
| 23 | iOS keyboard: composer on the keyboard, page never jumps | built (#43) | `chat.ts` sets `--vvh` from `visualViewport` (`.chat` height), the page kept at the top | owner's check in the Home Screen app |
| 24 | Back button, edge swipe, side-by-side ≥1000 px | moved to #48 | ‹ back link (`web/src/lib.rs` `chat`) | #48 |
| 25 | Working pill with timer and [stop]; reconnecting/offline | built (#43) | [stop] says stopping… until the turn stops; `#net` says reconnecting… or offline | — |
| 26 | Queue or steer during a turn; edit/remove queued | built (#43) | `POST /threads/{id}/queue`, `/steer`, `/unqueue`; the server sends the next as a turn ends (`hub.rs` `drain`) | — |
| 27 | Per-thread drafts, picked files included | built (#43) | the IndexedDB draft keeps the picked files | — |
| 28 | Long-press menu: copy, copy as markdown, quote | built (#43) | `#menu` popover on a long press or right click | — |
| 29 | Sticky thread header; older messages on scroll | done | `.th` above the scrolling list; slices by id with preload (`chat.ts` `load`, `preload`) | — |
| 30 | Choice rows, "other" box, answered ✓ line | built (#44) | chat.ts `choices()`: recommended first, an "other" box on plugin cards; the ✓ line has `by` (the device) and when (hub `approve`) | — |
| 31 | Conductor-answered asks keep [change] 30 min | built (#44) | `/plugin/card` `answered`/`by`: [change] until `until` (core `Chat::tap`) | — |
| 32 | Config-proposal card | built (#44) | `form: config`: the diff, [apply] [discard], the backend's line or its 409 (stale); `#cw` bar and the list's `waiting` | — |
| 33 | Secret request card, masked input | built (#44) | `form: secret`: a password field; the value goes to the plugin only | — |
| 34 | Harness pick at start; model/effort chip | built (#44) | agent picker plus model/effort at start; composer chips, `POST /threads/{id}/model` (q662: from the next start until shed can switch a live session) | live switch: a shed input for ACP `session/set_model` |
| 35 | Handoff chips, flow-back lines, pick-a-thread row | built (#44) | `/plugin/handoff`, `/plugin/line` (`link` items with chips); `form: thread` | — |

Totals at `51f92e8`: 4 done (9, 11, 21, 29), 18 partial, 13 missing. The rows 1–14 PR builds 1–8, 10 and 12–14
(9 and 11 were done); rows 15–29 and 30–35 follow as cooee#43 and cooee#44.

Rows 15–29 (but 24, moved to #48) are built in cooee#43; its tests are `web/e2e/parity-threads.spec.mjs` (a 390 px shot per row
in `docs/screens/43`), `web/e2e/md.spec.mjs` and `crates/cooee/tests/threads.rs`.
