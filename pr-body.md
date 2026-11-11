Closes #140

Native app build 4/6: plugin screens as data, live.

## What changed

**Server**
- `cooee-plugin.toml` gets `[[places]]` (`header-strip`, `chat-cards`, `menu-row`) and `[permissions]` (places, push, actions, destructive, network). A place that `[permissions]` doesn't grant is refused at parse, and so is a destructive action missing from `actions`. A plugin may now have screens and no tab; it gets no PWA tab.
- `GET /p/{plugin}/screens/{path}`: the backend's screen with the plugin's token (never the cookie). A stream comes back event by event, with its `{seq, ops}` data and id.
- `POST /p/{plugin}/action` `{screen, block, action, values}`: goes to the backend's `POST /action` with `by` set to the device. **An action not in `[permissions] actions` gets a 403 and is never sent.**
- `GET /personas/{id}/places` returns the attached plugins' `header-strip` chips, and `GET /plugins` rows carry `places` and `permissions` for the attach sheet.
- `hello.py` from #125 ships as the example (`docs/plugins/hello-py/`). Its screen is now live: a wave shows up on every open device.
- `docs/api.md` and `docs/plugins.md` (new "Screens" section) are updated.

**App**
- The header strip: a persona's chips come from `/personas/{id}/places`, and a tap opens the plugin's screen as a sheet.
- The plugin screen sheet: drawn by #138's block renderer, it follows the stream's ops in place (`LiveScreen`), and actions go through `POST /action`. The last answer is kept on the device.
- Offline: when the Mac or the plugin can't be reached, the sheet shows the kept answer under an amber line (`Mac unreachable` or `<plugin> isn't answering`, `as of 5m ago`, ticking, and Retry). Actions are off until it's back.
- The attach sheet: the chat's `⋯` lists the installed plugins with a header-strip place that aren't attached yet. The sheet shows the version, commit and source, what the plugin may do, its actions (destructive ones in red) and what its backend reaches. Attach adds it to the persona.

**e2e**
- `apple/e2e/serve.sh` installs a fixture `drover` (`apple/e2e/screens.py`) that serves #125's `queue.json` and `events.json`. Its queue stream plays `queue-stream.json`, and `pause` takes it down for 20 s for the offline case.
- In the simulator, `LiveUITests` attaches it to Conductor, then opens Events, then Queue (and waits for the live update), then goes offline.

## Decisions
- q1212 (owner): drover's real screens land in herdsman (herdsman#814), not here. This PR uses a fixture plugin serving #125's JSON. AC2's live check against a real fleet runs once herdsman#814 merges and `plugins/index.toml` points at herdsman.
- q1213 (owner): apple_cap raise, filed as #204 (merged: a5c9e55 raised apple_cap to 7500). The Swift here is +586 lines.
- Refusals cover the new surface (screens and `POST /action`). `/plugin/push` and `/plugin/card` keep today's checks (cards only into an attached persona's chats), so the PWA's drover keeps working until its manifest declares `[permissions]`.
- `hello.py` moved from `docs/design/screens-as-data/hello/` to `docs/plugins/hello-py/`. The design's `sd.js` and README point at the new path.

## Evidence (head f3fa2a6bc25e6df74a7d5e0d000e68cd4223ac84)
- `cargo test --workspace --locked`: exit 0, including `crates/cooee/tests/screens.rs`. That test runs the real hello.py behind `cooee plugin add` and checks the screen, the stream and a wave. It also checks a 403 for `drop` (an action outside `[permissions]`, with the backend's count unchanged) and 403s for a screen path off the manifest and for `..` / `%2E%2E` paths.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`: exit 0
- `scripts/apple-check.sh`: exit 0
- CI at f3fa2a6: all 12 checks green. The apple workflow's run 42923961787 (build iOS Simulator, build macOS, apple-quality, e2e) concluded `success`; the ci workflow's stable, msrv, ui, ts, no-node and actionlint all pass.
- Screenshots: the `simulator-shots` artifact of run 42923961787 (e2e, `LiveUITests/testPairsStreamsAndShowsHome`), https://github.com/jirathip-dev/cooee/actions/runs/42923961787/artifacts/13813828730
  - `attach-dark`: Attach drover. The sheet lists the Queue and Events header-strip chips, the actions with `stop` and `drop` in red as "always confirmed first", and that the backend reaches 127.0.0.1 (#125's `attach-*`). The dark pass attaches; the light pass finds drover already attached, so the attach sheet has no light shot.
  - `conductor-light` / `-dark`: Conductor's header with the Queue and Events chips (#125's `conductor-*`).
  - `queue-light` / `-dark` and `queue-live-*`: drover's Queue as a sheet, then the same sheet after the stream's ops update it in place (#125's `queue-*`).
  - `events-light` / `-dark`: the Events screen.
  - `queue-offline-light` / `-dark`: the amber "drover isn't answering · as of" line with Retry, over the kept Queue with its actions dimmed and off (#125's `queue-offline-*`).
  - `chat-cards-*` / `chat-cards-answered-*`: an ask card answered in the chat.

src/ net +269 non-test Rust/web lines (24120 / 27500) and +586 Swift lines (apple_cap 7500 after #204). No budget is stated in the issue; this is the server proxy and permissions plus the app's sheet, strip and attach.

## For humans

Plugins can now show their own screens in the iPhone and Mac app: chips under a persona's chat header open them as sheets that update live.
When the Mac or a plugin can't be reached, the sheet still shows the last answer with how old it is, and its buttons stay off until it's back.
Attaching a plugin to a persona first shows what it is allowed to do, and cooee refuses any action a plugin didn't declare.

