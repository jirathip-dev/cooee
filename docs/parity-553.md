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
| 15 | Thread list per repo: unread dot, state, last line, badge | partial | `web/src/lib.rs` `row` and `threads` (folder cards, ● unread, state, last line) | unread badge; live updates |
| 16 | Start by typing; auto title; retitle | partial | new-thread form (`web/src/lib.rs` `threads`), title from the first text (`cooee-server/src/hub.rs:370`) | retitle |
| 17 | Swipe to archive, 5 s undo, restore | missing | only close (`cooee-server/src/lib.rs:419`) | archive, undo, restore, archived list |
| 18 | Message blocks: colours, markdown, system lines, folded tool runs | partial | `web/ts/chat.ts` `fill()` (you/agent/tool/plan/system rows) | markdown (text is plain); folding runs of tools |
| 19 | Fenced code blocks with copy | missing | — | code blocks and [copy] |
| 20 | Several attachments, thumbnails, image viewer | missing | — | files on send, storage, `GET` by name, viewer |
| 21 | Instant send: sending…/✓, red retry | done | `web/ts/chat.ts` `deliver()` (sending → sent, failed · tap to retry) | — |
| 22 | Growing composer, pinned send, follow-scroll | partial | follow-scroll (`chat.ts` `stuck` + ResizeObserver) | the box growing to 6 lines then scrolling |
| 23 | iOS keyboard: composer on the keyboard, page never jumps | partial | `interactive-widget=resizes-content` (`web/src/lib.rs:190`), anchor kept on resize (`chat.ts`) | unverified in the Home Screen app; visualViewport fallback |
| 24 | Back button, edge swipe, side-by-side ≥1000 px | partial | ‹ back link (`web/src/lib.rs` `chat`); the sidebar beside the chat or composer from 1024 px, as #48's approved design draws it (`web/src/compose.rs` `side`) | edge swipe |
| 25 | Working pill with timer and [stop]; reconnecting/offline | partial | `#pill` with timer and stop (`web/src/lib.rs` `chat`, `chat.ts` `setState`) | "stopping…"; reconnecting/offline states |
| 26 | Queue or steer during a turn; edit/remove queued | missing | — | queue, steer, unqueue |
| 27 | Per-thread drafts, picked files included | partial | IndexedDB draft per thread (`chat.ts` `draft`) | picked files (after row 20) |
| 28 | Long-press menu: copy, copy as markdown, quote | missing | — | the menu |
| 29 | Sticky thread header; older messages on scroll | done | `.th` above the scrolling list; slices by id with preload (`chat.ts` `load`, `preload`) | — |
| 30 | Choice rows, "other" box, answered ✓ line | partial | approval rows with options and ✓ (`chat.ts` `fill` approval); plugin cards (`cooee-server/src/lib.rs:619`) | "other" free-text box; who and when on the ✓ line |
| 31 | Conductor-answered asks keep [change] 30 min | missing | — | [change] on an answered ask |
| 32 | Config-proposal card | missing | — | diff, apply/discard, stale, waiting bar |
| 33 | Secret request card, masked input | missing | — | the card and its write |
| 34 | Harness pick at start; model/effort chip | partial | agent picker at start (`web/src/lib.rs` `agent`) | model/effort chip and its op |
| 35 | Handoff chips, flow-back lines, pick-a-thread row | missing | — | chips, lines, the choice row |

Totals at `51f92e8`: 4 done (9, 11, 21, 29), 18 partial, 13 missing. The rows 1–14 PR builds 1–8, 10 and 12–14
(9 and 11 were done); rows 15–29 and 30–35 follow as cooee#43 and cooee#44.
