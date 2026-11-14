# Native parity audit: herdsman#553 against the iOS app (#366)

Audited at `cdc9794b` (staging), with the drover plugin at the herdsman commit `plugins/index.toml` pins (`3ebaf6c`).
The web client does not count (it goes away under #189); the owner's phone pass runs on the TestFlight build (q1628).

**done**: the app does it; **partial**: some of it; **missing**: none of it; **n/a-web**: only a web client needs it,
with the native equivalent named; **n/a-shed**: a reliability row with no cooee part; **dropped**: #553's `drop?` rows.
**gate**: ● the row is in the retirement gate (#553's amendment q873). Paths are under `apple/Sources/` unless they
start with `crates/`; herdsman's are `screens.rs` (`crates/server/src/screens.rs` in herdsman).
Every phone check runs in the native app from TestFlight, paired to the host.

## The one blocker across the drover rows

herdsman builds its asks, needs-human, design and run cards at `GET /screens/cards` for a `chat-cards` place, but
nothing in cooee reads a `chat-cards` place and herdsman never posts `/plugin/card`: no drover card reaches the app
(rows 63–71 and run cards → #377). The app's block renderer also ignores `tap.thread` and `tap.open` and sends every
action with an empty block id (→ #378).

## cooee core (rows 1–35)

| # | feature | gate | state | where | gap | issue | phone check |
|---|---|---|---|---|---|---|---|
| 1 | Home Screen app, offline paint | ● | n/a-web | native: the app itself, `AppIconPicker.swift`; kept copy only for plugin sheets (`KeptScreens.swift`) | native offline copy of Home and chat missing | #369 | Airplane mode, open the app: the last Home and thread show with "as of Xm ago" |
| 2 | Live stream, sync dot, gapless reconnect, no scroll jump | ● | partial | `CooeeAPI/Events.swift` `Client.events` (SSE, Last-Event-ID); `CooeeUI/Layout.swift` `Scroller` | no sync dot; Home polls every 5 s | #369 | Toggle airplane mode in a thread: dot amber then green, no message missing |
| 3 | Themed loading strip, "as of Xm ago" | | partial | `CooeeUI/PluginSheets.swift` (plugin sheets only) | no strip or age on Home and chat | #369 | Slow network, open a thread: a thin strip, content stays tappable |
| 4 | Short self-clearing toasts | | missing | `ChatFeature`/`HomeFeature` set `state.error`, no view draws it | errors are silent; no toasts | #369 | Tap any action: a short toast, then it clears |
| 5 | Pull to refresh, ↻, reload on resume | | partial | `HomeFeature` `poll`; `ChatFeature` reconnect loop | no `.refreshable`, no ↻, no reload on foreground | #369 | Pull down on Home: it reloads |
| 6 | Pair by code approved on host; list and remove devices | ● | partial | `PairingFeature`; `CooeeAPI/Endpoints.swift` `askToPair`/`claim`/`cancelPair` | Settings › Devices is an empty page (`GET /devices`, `POST /devices/remove` unused) | #239 | Pair, run `cooee pair <code>`: paired; Settings › Devices lists and removes this phone |
| 7 | Unpaired devices read-only | | done | `AppFeature`: no server → onboarding/pairing only | — (an unpaired app has nothing to read) | — | Fresh install: only pairing shows, no composer |
| 8 | Web push: enable, status, test push, themed titles | ● | n/a-web | native: APNs, `CooeeApp/Push.swift` `PushDelegate`; `MenuFeature` `toggle` → `POST /push/apns` | status is the local token, not the server's; no test push | #373 | Settings › Notifications › send test push: it arrives |
| 9 | Push tap opens its thread; no push for open thread | | partial | `crates/cooee-server/src/push.rs` skips a watched thread | no notification-tap routing (no `UNUserNotificationCenterDelegate`) | #373 | Tap a reply push: its thread opens; an open thread's reply shows no banner |
| 10 | Themes as data: farm, kitchen, garden, terminal, moments | | missing | `CooeeUI/Root.swift` always `Palette.farm` | no theme pick, no packs | #252 | Look › Theme › garden: sprites and words change everywhere |
| 11 | Colour schemes: presets plus user files | | missing | Look offers `LookScheme` system/light/dark only | no presets, no user scheme files | #375 | Look › Colours › nord: colours change and survive a relaunch |
| 12 | Appearance per phone: theme, colours, pixel or ascii | | partial | `MenuFeature` `.scheme`/`.style` (modern/terminal/scene) in `SettingsStore` | no theme pick (Terminal is the ascii style) | #252 | Look › Terminal: rows turn to text figures |
| 13 | Settings sheet: section index, deep links, alert dot | | partial | `CooeeUI/Menu.swift` `MenuSheet`/`AvatarMenu` | Look and Notifications only; no deep links, no alert dot | #312, #239 | Avatar menu: every section opens; alerts off shows a dot |
| 14 | Report mode | | missing | server `POST /report` only | no report mode in the app | #376 | Report, tap an element, Send: `cooee reports` lists it |
| 15 | Thread list: unread dot, state, last line, badge | ● | done | `CooeeUI/HomeRows.swift` `ChatRow`; `ChatFeature` `read` | — | — | New reply: unread badge shows; opening the thread clears it |
| 16 | Start by typing; auto title; retitle | ● | partial | `ChatFeature` `.new` → `newThread`; server titles from the first line | no retitle (`POST /threads/{id}/title` unused) | #370 | + chat, send "hi": the row reads "hi"; rename it from the header |
| 17 | Swipe to archive, 5 s undo, restore | | missing | server `POST /threads/{id}/archive` only | no swipe, undo or restore | #370 | Swipe left, archive, undo: the thread is back |
| 18 | Message blocks: colours, markdown, system lines, folded tool runs | ● | done | `CooeeCore/Item.swift` `ChatItems`; `CooeeUI/Items.swift` `BlockView`, `ToolFold` | — | — | Tap "▸ 3 commands": rows open and stay open on update |
| 19 | Fenced code blocks with copy | ● | done | `CooeeUI/Items.swift` `CodeBlock` | — | — | Tap Copy on a code block, paste it |
| 20 | Several attachments, thumbnails, image viewer | ● | partial | `CooeeUI/Composer.swift` `fileImporter`; `Endpoints` `send` files | no thumbnails or viewer (`GET /threads/{id}/files/{name}` unused) | #371 | Send 2 photos, tap one: full image, swipe down closes |
| 21 | Instant send: sending…/✓, red retry | ● | partial | `CooeeFeatures/ChatOutbox.swift`; `Items.swift` `Delivery` | no ✓; a retry sends a new id (could post twice) | #73 | Send offline: "Not delivered · Retry"; Retry resends once |
| 22 | Growing composer, pinned send, follow-scroll | | done | `Composer.swift` `TextField` `lineLimit(1...5)`; `Scroller` bottom anchor | — (grows to 5 lines) | — | Type 8 lines: the box grows, then scrolls |
| 23 | iOS keyboard: composer on the keyboard | | n/a-web | native: SwiftUI keyboard avoidance, composer at the bottom of `ChatView`, `scrollDismissesKeyboard` | — (native test → #385) | #385 | Tap the composer: it sits flush on the keyboard |
| 24 | Back, edge swipe, side-by-side ≥1000 px | | done | `CooeeUI/SwipeBack.swift`; `ChatHeader` back; `Root` `WideView` on regular width | — | — | Swipe right from the left edge: back at Home |
| 25 | Working pill with timer and [stop]; reconnecting/offline | ● | partial | `CooeeUI/Chat.swift` `WorkingPill` → `/threads/{id}/cancel` | no "stopping…", no reconnecting/offline state | #369 | While working, tap Stop: "stopping…", then idle |
| 26 | Queue or steer; edit/remove queued | | partial | `Composer.swift` `queuedRow` (send now, take back) | no edit of a queued message | #371 | Send while working: queued chip; Send now steers it in |
| 27 | Per-thread drafts, picked files included | | missing | `ChatFeature.State.draft`/`files` in memory | lost on back or relaunch | #371 | Type, force-quit, reopen: the draft is there |
| 28 | Long-press: copy, copy as markdown, quote | | partial | `CooeeUI/Copyable.swift` (Copy, Select text) | no copy as markdown, no quote | #371 | Long-press a reply, Quote: "> " lines in the box |
| 29 | Sticky header; older messages on scroll | | partial | `ChatHeader` above the scroll; `Endpoints` `messages(before: 50)` | older messages never load | #372 | Scroll to the top: older messages load |
| 30 | Choice rows, "other" box, answered ✓ line | ● | done | `crates/cooee-core/src/blocks.rs` choice; `CooeeUI/BlockCards.swift` `ChoiceBlock` | — | — | Tap the recommended option: ✓ with who |
| 31 | Conductor-answered asks keep [change] 30 min | | done | `blocks.rs` keeps the choice open until `until`; `ChoiceBlock` re-answers via `/threads/{id}/approve` | — | — | Re-tap another option on an answered ask: the new pick goes |
| 32 | Config-proposal card | ● | partial | `blocks.rs` config card; `CardBlock` | no applying state, diff not coloured | #374 | Tap apply: "applying…", then "applied … doctor ✓" |
| 33 | Secret request card, masked input | ● | done | `CooeeUI/BlockFields.swift` `SecretBlock` | Face ID icon is dead (#240) | #240 | Save a value: "secret X set ✓" |
| 34 | Harness pick; model/effort chip | | partial | `CooeeUI/Components.swift` `AgentChip` (no action); persona sets the agent | no per-thread model/effort (`POST /threads/{id}/model` unused) | #240, #230 | Tap the agent chip, pick effort low: "from the next turn" |
| 35 | Handoff chips, flow-back lines, pick-a-thread | ● | partial | `/plugin/handoff`, `/plugin/line` → link items; `ChatItems` default branch | link items are plain notes, not tappable | #374 | Tap "continued in t6": t6 opens |

## drover queue (rows 36–48)

Served by herdsman as the conductor's Queue header-strip screen (`screens.rs` `queue_screen`), drawn by
`CooeeFeatures/PluginScreenFeature.swift` and `CooeeUI/Block*.swift`.

| # | feature | gate | state | where | gap | issue | phone check |
|---|---|---|---|---|---|---|---|
| 36 | One repo at a time: chips, swipe, needs dot, [all] | | partial | repo chips + `filter` action (`ChipsBlock`) | no swipe, no needs dot; the stream drops the filter | #378, #379 | Queue: swipe left: the next repo shows |
| 37 | Fleet cards in colour; paused folds; needs-you first | ● | partial | `fleet_card` | one accent for all; paused doesn't fold; order | #379 | Pause a fleet: its card folds to one line |
| 38 | Blocked chains as a tree, collapsed | | partial | `indent: 1` rows, "held until #b" | no tree, no collapse | #379 | Tap a chain row: its steps open |
| 39 | Groups, place numbers, "+N more", stage times, done today | | partial | sections with counts, `#k` badges, `since`, done today | no "+N more" | #379 | Tap "+3 more": rows open in place |
| 40 | Issue and PR titles; issue, PR, checks links | ● | partial | row title; `tap.url` to the PR (non-live rows) | no issue/checks links; live rows' `tap.thread` ignored | #378, #379 | Tap [checks]: the PR's checks page opens |
| 41 | Reorder ▲/▼ | | partial | long-press Top/Drop (`RowView` context menu) | no ▲/▼ | #379 | Tap ▲, pull to refresh: the row stays moved |
| 42 | Pause and resume a fleet | ● | done | card Pause/Resume → `pause` action | — | — | Tap Pause: the card reads paused |
| 43 | Drive now, per-fleet drive line | ● | done | Drive now button, `drive.line` subtitle, toast; "Drive all now" | — | — | Tap Drive now: "driving…", then "started N" |
| 44 | Pipeline stage chips | | partial | `rounds()` subtitle text | no lit chips | #379 | A row in impl shows the impl chip lit |
| 45 | Issue card lanes; tap opens the lane | | partial | live row `tap: {thread}` | `tap.thread` ignored by `ActButton.fire` | #378 | Tap a lane on a row: its Workflow chat opens |
| 46 | "waiting: decision" with options and asking thread | | partial | row badge "waiting:" | no options, no link | #379 | Tap "asked in <thread>": that thread opens |
| 47 | Backlog: search, filters, tick, [queue N] | | missing | — | no backlog | #379 | Tick 2, [queue 2]: they appear queued |
| 48 | Legend [?], serial 🔒 tags | | missing | — | no legend, no tags | #379 | Tap [?]: the legend explains every state |

## Lanes, now cooee core (rows 49–62; #553 re-mapping to the closed cooee#33 Workflow sessions and cooee#34 Scene)

| # | feature | gate | state | where | gap | issue | phone check |
|---|---|---|---|---|---|---|---|
| 49 | Lanes grouped by state, repo, newest first | | partial | Workflow threads in Home Chats (`CooeeCore/Listing.swift` `chats`) | no grouping; `workflow` labels not decoded | #229 | Home: lanes sit under Conductor › Workflow, blocked first |
| 50 | Lane row: name, age, waiting reason, "now:" | ● | partial | `ChatRow`: title, last line, age, state dot | no role/round, no waiting reason | #229 | A working lane's row reads "now: cargo test" |
| 51 | Parked, dead, host-offline with reasons | | missing | `LaneState` maps working/needs/done/fail/idle | no such states | #381 | A parked lane reads "parked · <reason>" |
| 52 | Conductor sessions as lanes; tap opens thread | | done | conductor persona's chats on Home | — | — | Tap a conductor row: its thread opens |
| 53 | Foreign agents get only [close] | | missing | hub refuses `close` on Workflow threads | no close for a dead/foreign session | #381 | Close a dead session, confirm: it leaves the list |
| 54 | Bottom sheet: drag, 3 snap heights | | partial | SwiftUI `.sheet` (`MemberSheet` medium/large) | `PluginScreenSheet` has no detents | #378 | Drag a plugin sheet's handle: it snaps to 3 heights |
| 55 | Transcript as role blocks; load more | | partial | `ChatItems` you/agent/tool folds | newest 50 only | #372 | Scroll to a lane's top: older turns load |
| 56 | Summary view, full-conversation toggle | | missing | — | no toggle | #242 | Switch the persona to Chat style: final replies only, a tap shows every turn |
| 57 | Reply box to a lane | | done | composer → `/threads/{id}`; hub routes to the Workflow terminal | — | — | Send a reply: it shows in the lane's transcript |
| 58 | "/" command menu | | missing | `Composer.swift` | no menu | #254 | Type "/": the menu lists commands |
| 59 | Stop, Kill with dry-run confirm, Clean with reasons | ● | partial | `WorkingPill` Stop; Queue live row long-press Stop | no Kill, no Clean | #380 | Long-press a lane, Kill: the confirm shows the unpushed/dirty line |
| 60 | Lane figures animated by state | | partial | figure: scene sprite / ascii glyph / tone dot | not animated; herdsman sends no `coat` | #246 | A working lane's figure animates |
| 61 | Herd scene: ranch, state bands, gate | | partial | `PinnedRow` `scene-strip` | no per-repo ranch, bands or gate | #181 | Scene: one figure per lane in its band |
| 62 | Weather, day/night, repo switching | | missing | — | none of it | #382 | Tap [☾]: night; tap ›: next repo |

## drover asks (rows 63–71) and run cards

Built by herdsman at `GET /screens/cards` (`screens.rs` `chat_cards`), never delivered; the native card blocks
(`BlockCards.swift`, `BlockFields.swift` `RunBlock`) are ready.

| # | feature | gate | state | where | gap | issue | phone check |
|---|---|---|---|---|---|---|---|
| 63 | Needs-you cards: options, free text, count badge | ● | missing | `ask_card` at `/screens/cards` only | not delivered; no count | #377 | Tap an option on an ask card: it goes, the count drops |
| 64 | Routine vs owner tag, default countdown | | missing | `ask_card` meta | not delivered | #377 | A card reads "routine · defaults to 2 in 24m" |
| 65 | Answer a blocked lane's menu by tap | | done | ACP permission → approval card → `/threads/{id}/approve` → shed | — | — | Tap "1. Yes" on a lane's prompt: sent, the card settles |
| 66 | Permission card: reason, command, auto-deny countdown | | partial | same approval card (title, options) | no reason/command detail, no countdown | #72 | The card shows the command and "auto-denies in 0:14" |
| 67 | Needs-human: merge anyway, one more round, park | ● | missing | `needs_card` at `/screens/cards` only | not delivered | #377 | Tap [one more round]: the pending line shows at once |
| 68 | [later] / [later all] hide cards on this phone | | missing | — | none | #377 | Tap [later all]: the cards hide on this phone only |
| 69 | Design review: PNGs inline, approve/revise/reject | ● | missing | `ask_card` `images` at `/screens/cards` only | not delivered | #377 | A design card shows its PNGs; approve: the impl lane starts |
| 70 | Verify screenshots on cards | | missing | — | herdsman sends none; nothing delivered | #377 | A verify card shows its real screenshots |
| 71 | Alert when an answer doesn't reach its lane | | missing | — | no push for `answer-undelivered` | #377 | Answer a gone lane: an "undelivered" push arrives |
| run cards | Run card: command and sha, approve/deny, exit code and tail (the closed cooee#83, herdsman#538) | ● | partial | `blocks.rs` run block; `BlockFields.swift` `RunBlock`; hub sends the sha once, 409 → stale | herdsman never sends it; tail shown raw | #377 | Approve a run card: exit code and tail show; a second approve is refused |

## drover system (rows 72–85; #553 re-mapped machines to the closed cooee#32, providers to cooee#30)

| # | feature | gate | state | where | gap | issue | phone check |
|---|---|---|---|---|---|---|---|
| 72 | Host cards: gauges, dots, top memory | ● | missing | `crates/cooee-machines` web tab only; Menu `.machines` empty | no native Machines screen | #239 | Settings › Machines: the main host's card is first |
| 73 | 24 h sparklines, 1h/6h/24h chart | | missing | cooee-machines `html.rs` only | no native charts | #239 | Tap the memory sparkline: the chart opens |
| 74 | Host health and services | | missing | — | no services, logins, version, drift | #239 | Log gh out on a host: its row turns red |
| 75 | Turn a host on or off | ● | missing | — | nowhere in the app | #380 | Drover › Hosts, [off], confirm: it reads off |
| 76 | Host tags; cloud spend and budget | | missing | — | none | #239, #383 | A cloud host's card shows this month's spend |
| 77 | Fleet disk, orphans, leftovers, [reap] | | missing | — | none | #380 | Tap [reap]: "reaped N leftover process(es)" |
| 78 | Provider accounts: 5h/7d meters, caps, balance | | missing | Menu `.signIns` empty | no native Sign-ins screen | #239 | Settings › Sign-ins: each account's 5h % |
| 79 | Provider sign-in, add key, [use] | | missing | Menu `.signIns` empty | none | #239 | Tap sign in: the login streams to the phone |
| 80 | Fleets: per-repo config, pipeline graph | | partial | `/screens/fleets` form (Queue › More › Fleets…) | no pipeline graph; the "drover" menu row isn't drawn | #213 | Open a fleet: grouped settings and its stage graph |
| 81 | Role model/effort pickers | | partial | Fleets impl/rev pickers from herdsman's models | no effort, two roles only | #380 | Pick review effort high: the next review lane uses it |
| 82 | Conductor role and new-thread chat defaults | | partial | `PersonaSheet` Agent/Model/Effort as text fields | no live lists; no chat defaults | #230 | Conductor persona: pick a model from the list: it saves |
| 83 | Activity: time groups, glyphs, [important], older | | partial | `/screens/events`: kind chips, figures, 24 h, stream | `tap.open` ignored; no [important], one group | #378, #380 | Events, tap [important]: noise rows hide |
| 84 | Notifiers and Telegram status | | missing | Notifications page shows cooee push only | none | #380 | Drover › Notifications: "telegram on · polled 1m ago" |
| 85 | Real config load errors shown plainly | ● | done | Queue `text` block `id: error` | — | — | Break fleet.toml: Queue shows the real error |

## drop? (rows 86–90)

| # | feature | state | note |
|---|---|---|---|
| 86 | Host and Events as tabs; [◐] look button | dropped | superseded by the settings sheet (herdsman#440) |
| 87 | Lanes mode button; fixed Corral art | dropped | superseded by styles and Scene |
| 88 | Queue as a table with per-row buttons | dropped | superseded by Queue cards |
| 89 | Server HTML tabs and `/frag` | dropped | superseded; the app draws blocks |
| 90 | Public demo as a static export | dropped | herdsman docs only |

## Reliability (R1–R27, cooee's part)

Server tests are in `crates/cooee/tests/reliability.rs` unless named otherwise. Phone checks are the native app on TestFlight.

| R | state | where | gap | issue | phone check |
|---|---|---|---|---|---|
| R1 | partial | `r1_a_hung_keychain_falls_through_to_a_new_key_and_blocks_nothing` | the app's `CooeeAPI/Keychain.swift` read has no deadline | #384 | — (a test row) |
| R2 | done | `crates/cooee-core/tests/upgrade_compat.rs` `every_earlier_release_config_loads` | — | — | — |
| R3 | partial | `r3_a_store_error_is_a_503_not_unpaired_and_a_401_does_not_latch` | the app forgets its pairing on one 401 (`AppFeature` `.home(.delegate(.unauthorized))`) | #384 | Restart the host: the app stays on Home |
| R4 | partial | `Layout.swift` `defaultScrollAnchor`, `scrollDismissesKeyboard` | no native keyboard test | #385 | Tap the composer: flush on the keyboard, the thread keeps its place |
| R5 | partial | `PluginSheets.swift` inline `ProgressView` | no 300 ms strip test | #385 | Slow network: a strip only, content tappable |
| R6 | done | `crates/cooee-server/src/hub.rs` `r6_requests_over_a_long_log_are_within_budget`; `cooee-core` `chat.rs` `r6_…` | native 1,000-message chat | #337 | A 1,000-message thread opens without a stall |
| R7 | done | `r7_page_loads_run_no_subprocess` | — | — | — |
| R8 | done | `crates/cooee-core/tests/group.rs` | — | — | — |
| R9 | partial | `chat_first.rs` (sends reach the new terminal) | no ask-after-replace-then-answer test | #386 | — |
| R10 | done | `r10_an_ask_made_during_the_start_is_listed_and_answerable` | — | — | — |
| R11 | done | `r11_a_slow_start_is_adopted_once` | — | — | — |
| R12 | n/a-shed | — | — | — | — |
| R13 | done | `r13_malformed_input_or_config_is_denied_with_a_reason` | — | — | — |
| R14 | done | `r14_a_crash_between_start_and_send_is_recovered_once` | — | — | — |
| R15 | done | `hub.rs` `r15_a_turn_idle_30_minutes_is_surfaced_as_needs_you` | — | — | — |
| R16 | partial | `r16_a_repeated_id_is_one_row_however_often_it_is_refreshed` | the app's upsert has no repeated-id test | #385 | — |
| R17 | done | `r17_paths_in_text_are_never_files_served` | — | — | — |
| R18 | done | `cooee-core` `devices.rs` `parallel_writes_never_share_a_temp_file` | — | — | — |
| R19 | done | `cooee-core` `accounts.rs` `a_dead_logins_leftover_folder_never_blocks_a_new_account` | — | — | — |
| R20 | done | `r20_a_stuck_delivery_never_holds_the_next_one` | — | — | — |
| R21 | done | `r21_an_answer_to_a_gone_session_says_not_delivered` | — | — | — |
| R22 | n/a-shed | — | — | — | — |
| R23 | n/a-shed | — | — | — | — |
| R24 | n/a-shed | — | — | — | — |
| R25 | partial | `apple/Tests/CooeeUITests/SnapshotTests.swift` (static frames) | no sheet-close flash test | #385 | Open and close a plugin sheet: no flash |
| R26 | n/a-shed | — | — | — | — |
| R27 | partial | `hub.rs` reported state from shed | no scripted ACP working → idle test | #386 | A long tool run: the pill never flickers |

## Totals and filed issues

Rows 1–85 and run cards: 15 done, 39 partial, 29 missing, 3 n/a-web; 86–90 dropped. Reliability: 14 done, 8 partial,
5 n/a-shed. Of the 28 gate rows, 8 are done (15, 18, 19, 30, 33, 42, 43, 85); the biggest blocker is #377.
Filed by this audit: #369–#386.
