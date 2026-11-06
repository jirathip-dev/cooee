# Parity audit: herdsman#553 rows 1–35 against cooee core (#21)

Audited at `51f92e8` (staging). **done**: cooee does it today; **partial**: some of it; **missing**: none of it.
Row numbers are #553's.

| # | feature | cooee today | where | gap to build |
|---|---|---|---|---|
| 1 | Home Screen app, offline paint | partial | `web/src/manifest.webmanifest` (standalone, icons); `web/src/lib.rs:190` viewport; `web/src/sw.js:1` caches nothing | no zoom lock; no offline copy of the last page with its time |
| 2 | Live stream, sync dot, gapless reconnect, no scroll jump | partial | `web/src/chat.js` `listen()` (SSE from `since=` cursor, `reset`), `keep()` (scroll held on a message); `web/src/app.css:41` `.dt` | the dot shows turn state, not the connection (amber/green); the thread list is not live |
| 3 | Themed loading strip, "as of Xm ago" | missing | `web/src/chat.js` `open()` has a 300 ms skeleton only | the strip and its age |
| 4 | Short self-clearing toasts | missing | errors are `alert()` (`chat.js` 🔔) or inline lines | a toast for every action's reply |
| 5 | Pull to refresh, ↻, reload on resume | missing | SSE reopens only on error (`chat.js` `listen()`) | all three |
| 6 | Pair by code approved on host; list and remove devices | partial | `cooee pair` prints a code typed on the phone (`crates/cooee/src/main.rs:136`, `web/src/lib.rs` `pair`); `cooee devices` / `devices rm` (`main.rs:154,161`) | phone-side request + `serve approve <code>`; device list and remove on the phone |
| 7 | Unpaired devices read-only | partial | unpaired devices are redirected to `/pair` (`crates/cooee-server/src/lib.rs:147`) | a read-only view with the composer off and a pair hint |
| 8 | Web push: enable, truthful status, test push, themed titles | partial | 🔔 enable (`chat.js`), `/push/key`, `/push/subscribe` (`cooee-server/src/lib.rs:630,647`), themed body (`web/src/look.rs` `push_body`) | status line ("alerts are on"), test push, unsubscribe |
| 9 | Push tap opens its thread; no push for open thread | done | `web/src/sw.js` `push` (skips a visible client on that thread) and `notificationclick` | — |
| 10 | Themes as data: farm, kitchen, garden, terminal, moments | partial | `web/src/look.rs` (data-only themes, own themes from config), `web/themes/*` | `moments` is parsed but never played |
| 11 | Colour schemes: presets plus user files | done | `web/src/look.rs` `Looks::load` (12 built-in + `<config>/schemes/*.json`, contrast gate); `look.js` COLOURS | — |
| 12 | Appearance per phone: theme, colours, pixel or ascii | partial | `web/src/look.js` (theme, mode, style modern/terminal, sprites on/off in localStorage) | ascii figures (sprites off shows a dot) |
| 13 | Settings sheet: section index, deep links, alert dot | partial | `web/src/look.js` `settings()` (sign-ins, look) | section index, `#system/<section>` deep links, alert dot |
| 14 | Report mode | missing | — | tap an element, note, send; host-side list |
| 15 | Thread list per repo: unread dot, state, last line, badge | partial | `web/src/lib.rs` `row` and `threads` (folder cards, ● unread, state, last line) | unread badge; live updates |
| 16 | Start by typing; auto title; retitle | partial | new-thread form (`web/src/lib.rs` `threads`), title from the first text (`cooee-server/src/hub.rs:370`) | retitle |
| 17 | Swipe to archive, 5 s undo, restore | missing | only close (`cooee-server/src/lib.rs:419`) | archive, undo, restore, archived list |
| 18 | Message blocks: colours, markdown, system lines, folded tool runs | partial | `web/src/chat.js` `fill()` (you/agent/tool/plan/system rows) | markdown (text is plain); folding runs of tools |
| 19 | Fenced code blocks with copy | missing | — | code blocks and [copy] |
| 20 | Several attachments, thumbnails, image viewer | missing | — | files on send, storage, `GET` by name, viewer |
| 21 | Instant send: sending…/✓, red retry | done | `web/src/chat.js` `deliver()` (sending → sent, failed · tap to retry) | — |
| 22 | Growing composer, pinned send, follow-scroll | partial | follow-scroll (`chat.js` `stuck` + ResizeObserver) | the box growing to 6 lines then scrolling |
| 23 | iOS keyboard: composer on the keyboard, page never jumps | partial | `interactive-widget=resizes-content` (`web/src/lib.rs:190`), anchor kept on resize (`chat.js`) | unverified in the Home Screen app; visualViewport fallback |
| 24 | Back button, edge swipe, side-by-side ≥1000 px | partial | ‹ back link (`web/src/lib.rs` `chat`) | edge swipe; list + chat side by side |
| 25 | Working pill with timer and [stop]; reconnecting/offline | partial | `#pill` with timer and stop (`web/src/lib.rs` `chat`, `chat.js` `setState`) | "stopping…"; reconnecting/offline states |
| 26 | Queue or steer during a turn; edit/remove queued | missing | — | queue, steer, unqueue |
| 27 | Per-thread drafts, picked files included | partial | IndexedDB draft per thread (`chat.js` `draft`) | picked files (after row 20) |
| 28 | Long-press menu: copy, copy as markdown, quote | missing | — | the menu |
| 29 | Sticky thread header; older messages on scroll | done | `.th` above the scrolling list; slices by id with preload (`chat.js` `load`, `preload`) | — |
| 30 | Choice rows, "other" box, answered ✓ line | partial | approval rows with options and ✓ (`chat.js` `fill` approval); plugin cards (`cooee-server/src/lib.rs:619`) | "other" free-text box; who and when on the ✓ line |
| 31 | Conductor-answered asks keep [change] 30 min | missing | — | [change] on an answered ask |
| 32 | Config-proposal card | missing | — | diff, apply/discard, stale, waiting bar |
| 33 | Secret request card, masked input | missing | — | the card and its write |
| 34 | Harness pick at start; model/effort chip | partial | agent picker at start (`web/src/lib.rs` `agent`) | model/effort chip and its op |
| 35 | Handoff chips, flow-back lines, pick-a-thread row | missing | — | chips, lines, the choice row |

Totals: 4 done (9, 11, 21, 29), 18 partial, 13 missing.
