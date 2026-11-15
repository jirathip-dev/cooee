# Every listed file and what happens to it

The 80 files of #356's table, in its order. `S` is the slice ([adr.md](adr.md)); `189` is the web client's
removal (#189, ready), which this ADR builds on. pred. = conflict-predicted, upd. = update rounds, from the issue.

| file | PRs | pred. | upd. | S | after |
|---|---:|---:|---:|---|---|
| `crates/cooee-server/src/lib.rs` | 44 | 472 | 20 | 1 | **generated** module index; handlers to `routes/<area>.rs`, wiring to `serve.rs`, the gate to `gate.rs`, root names to `root.rs` (frozen) |
| `crates/cooee-server/src/hub.rs` | 39 | 552 | 17 | 2 | **kernel** (the types, `open`, load and unload) under the size cap; every operation to `hub/<area>.rs` |
| `docs/architecture.md` | 36 | 155 | 17 | 5 | **deleted, frozen**; its sections to `docs/<area>.md`, the overview to `docs/README.md` |
| `crates/cooee-core/src/lib.rs` | 30 | 41 | 12 | 0 | **generated** module index; root names to `root.rs` (frozen) |
| `web/src/app.css` | 30 | 279 | 20 | 189 | gone: #189 deletes it (moved to `design-system/app.css`, a design asset) |
| `docs/api.md` | 28 | 314 | 20 | 5 | **deleted, frozen**; its sections to `docs/<area>.md` under `## API`, the conventions to `docs/README.md` |
| `crates/cooee/tests/support/mod.rs` | 26 | 103 | 16 | 4 | **generated** flat index; the support by concern in `support/{shed,agent,cooee,events}.rs` |
| `web/src/lib.rs` | 26 | 497 | 21 | 189 | gone: #189 deletes it |
| `web/e2e/serve.sh` | 25 | 173 | 14 | 189 | gone: #189 deletes it |
| `crates/cooee/src/main.rs` | 24 | 45 | 10 | 3 | **kernel** (parse, look up, exit code) under the size cap; one file per command in `cmd/`, index generated |
| `web/ts/chat.ts` | 19 | 254 | 16 | 189 | gone: #189 deletes it |
| `crates/cooee-core/src/chat.rs` | 17 | 23 | 9 | - | as it is: one area's module, under the size cap |
| `crates/cooee-core/src/shed.rs` | 17 | 12 | 6 | - | as it is: one area's module, under the size cap |
| `crates/cooee-server/src/push.rs` | 17 | 21 | 8 | - | as it is: one area's module; over the size cap, so the baseline lets it only shrink |
| `crates/cooee-core/src/store.rs` | 16 | 20 | 10 | - | as it is: one area's module, under the size cap |
| `crates/cooee-server/src/login.rs` | 15 | 24 | 10 | - | as it is: one area's module, under the size cap |
| `docs/plugins.md` | 15 | 4 | 4 | 5 | per area: takes the plugins' architecture and API sections; `machines` and `terminal` leave to `docs/plugin-machines.md`, `docs/plugin-terminal.md` |
| `crates/cooee-core/src/config.rs` | 14 | 4 | 4 | - | as it is: one area's module, under the size cap |
| `Cargo.lock` | 13 | 24 | 5 | 0 | **generated** by `scripts/gen.sh` (`cargo update --workspace`) |
| `web/src/chat.js` | 12 | 6 | 1 | - | gone already: build output since #49, no longer in the tree |
| `ci/cap.toml` | 11 | 106 | 3 | - | as it is: two numbers in the format drover reads; lanes cannot edit it; raises are already batched (#668) |
| `crates/cooee-core/src/login.rs` | 11 | 19 | 4 | - | as it is: one area's module, under the size cap |
| `crates/cooee/tests/plugins.rs` | 11 | 80 | 8 | - | as it is: one area's tests, collected by cargo already |
| `web/e2e/look.spec.mjs` | 11 | 141 | 10 | 189 | gone: #189 deletes it |
| `web/src/agents.rs` | 11 | 116 | 3 | 189 | gone: #189 deletes it |
| `apple/UITests/LiveUITests.swift` | 10 | 335 | 4 | 8 | **deleted, frozen**; one file per screen in `UITests/Live/`, the base case in `LiveUITestCase.swift` |
| `crates/cooee-core/src/personas.rs` | 10 | 93 | 4 | - | as it is: one area's module, under the size cap |
| `Cargo.toml` | 9 | 0 | 0 | - | as it is: the format is cargo's; a new dependency is an owner ask already |
| `apple/Sources/CooeeUI/Root.swift` | 9 | 120 | 4 | 8 | keeps `AppRoot`; `ChatScreen.swift` and `PersonaScreen.swift` leave it |
| `crates/cooee/tests/chat.rs` | 9 | 47 | 11 | - | as it is: one area's tests, collected by cargo already |
| `crates/cooee/tests/personas.rs` | 9 | 63 | 4 | - | as it is: one area's tests, collected by cargo already |
| `.github/workflows/ci.yml` | 8 | 20 | 3 | 6 | the gate list leaves to `ci/gates/`; the file stays the CI area's own |
| `README.md` | 8 | 128 | 0 | 5 | **generated** `Today` list (from each `docs/<area>.md`); the roadmap list to `docs/roadmap.md` |
| `crates/cooee/Cargo.toml` | 8 | 0 | 3 | - | as it is: as `Cargo.toml` |
| `crates/cooee/tests/accounts.rs` | 8 | 36 | 4 | - | as it is: one area's tests, collected by cargo already |
| `crates/cooee/tests/login.rs` | 8 | 15 | 5 | - | as it is: one area's tests, collected by cargo already |
| `crates/cooee/tests/pairing.rs` | 8 | 15 | 6 | - | as it is: one area's tests, collected by cargo already |
| `web/e2e/parity.spec.mjs` | 8 | 72 | 6 | 189 | gone: #189 deletes it |
| `web/e2e/threads.spec.mjs` | 8 | 36 | 4 | 189 | gone: #189 deletes it |
| `.github/workflows/apple.yml` | 7 | 94 | 4 | 6 | the e2e steps leave to `apple/e2e/steps/`; the file stays the CI area's own |
| `apple/Cooee.xcodeproj/project.pbxproj` | 7 | 0 | 4 | - | as it is: sources are synchronized folders already; only build settings edit it |
| `apple/Sources/CooeeApp/Live.swift` | 7 | 48 | 4 | 7 | **deleted, frozen**; one file per client in `Live/`, `CooeeRoot.swift` |
| `apple/Sources/CooeeUI/Chat.swift` | 7 | 13 | 0 | - | as it is: one screen's or one type's file, under SwiftLint's 250 lines |
| `crates/cooee-core/src/devices.rs` | 7 | 8 | 3 | - | as it is: one area's module, under the size cap |
| `crates/cooee-server/src/mcp.rs` | 7 | 35 | 3 | - | as it is: one area's module, under the size cap |
| `crates/cooee/tests/setup.rs` | 7 | 2 | 6 | - | as it is: one area's tests, collected by cargo already |
| `docs/themes.md` | 7 | 33 | 6 | - | as it is: already one area's doc |
| `web/e2e/cards.spec.mjs` | 7 | 36 | 7 | 189 | gone: #189 deletes it |
| `web/src/sheet.rs` | 7 | 139 | 11 | 189 | gone: #189 deletes it |
| `web/src/shown.rs` | 7 | 29 | 9 | 189 | gone: #189 deletes it |
| `web/ts/look.ts` | 7 | 26 | 11 | 189 | gone: #189 deletes it |
| `apple/Package.swift` | 6 | 0 | 3 | - | as it is: the format is SwiftPM's; sources are globbed already, only a new target or resource edits it |
| `apple/Sources/CooeeFeatures/Clients.swift` | 6 | 13 | 4 | 7 | **deleted, frozen**; one file per client in `Clients/` |
| `crates/cooee-core/src/profiles.rs` | 6 | 7 | 6 | - | as it is: one area's module, under the size cap |
| `crates/cooee-server/src/hub/rooms.rs` | 6 | 0 | 1 | - | as it is: one area's module, under the size cap |
| `crates/cooee/tests/mcp.rs` | 6 | 46 | 3 | - | as it is: one area's tests, collected by cargo already |
| `crates/cooee/tests/profiles.rs` | 6 | 71 | 9 | - | as it is: one area's tests, collected by cargo already |
| `crates/cooee/tests/reliability.rs` | 6 | 15 | 4 | - | as it is: one area's tests, collected by cargo already |
| `justfile` | 6 | 32 | 3 | 6 | **frozen** at `check`, `gen`, `apple-check`; a gate is a file in `ci/gates/`, a task a script in `scripts/` |
| `web/e2e/first-run.spec.mjs` | 6 | 63 | 6 | 189 | gone: #189 deletes it |
| `web/src/compose.rs` | 6 | 15 | 4 | 189 | gone: #189 deletes it |
| `web/src/look.js` | 6 | 5 | 1 | - | gone already: build output since #49, no longer in the tree |
| `web/src/look.rs` | 6 | 31 | 5 | 189 | gone: #189 deletes it (moved to `crates/cooee-server/src/look.rs`, one area's module) |
| `apple/Sources/CooeeCore/Item.swift` | 5 | 41 | 0 | - | as it is: one screen's or one type's file, under SwiftLint's 250 lines |
| `apple/Sources/CooeeCore/Models.swift` | 5 | 55 | 3 | 8 | **deleted, frozen**; one file per type in `Models/` |
| `apple/Sources/CooeeFeatures/ChatFeature.swift` | 5 | 356 | 0 | 9 | **kernel**: one `Action` case per area; the area's actions and reducer in its own file |
| `apple/Sources/CooeeUI/Fixtures.swift` | 5 | 0 | 1 | - | as it is: one screen's or one type's file, under SwiftLint's 250 lines |
| `apple/Sources/CooeeUI/Home.swift` | 5 | 36 | 3 | - | as it is: one screen's or one type's file, under SwiftLint's 250 lines |
| `apple/Sources/CooeeUI/Items.swift` | 5 | 40 | 0 | - | as it is: one screen's or one type's file, under SwiftLint's 250 lines |
| `apple/Sources/CooeeUI/Sheets.swift` | 5 | 16 | 3 | - | as it is: one screen's or one type's file, under SwiftLint's 250 lines |
| `apple/Tests/CooeeCoreTests/ItemsTests.swift` | 5 | 13 | 0 | - | as it is: one type's tests, collected by SwiftPM already |
| `crates/cooee-core/src/accounts.rs` | 5 | 0 | 3 | - | as it is: one area's module, under the size cap |
| `crates/cooee-core/src/plugins.rs` | 5 | 0 | 5 | - | as it is: one area's module, under the size cap |
| `crates/cooee-server/Cargo.toml` | 5 | 0 | 2 | - | as it is: as `Cargo.toml` |
| `crates/cooee-server/src/manage.rs` | 5 | 74 | 2 | - | as it is: one area's module; over the size cap, so the baseline lets it only shrink |
| `crates/cooee-server/src/plugin.rs` | 5 | 12 | 2 | - | as it is: one area's module, under the size cap |
| `crates/cooee/tests/first_run.rs` | 5 | 36 | 4 | - | as it is: one area's tests, collected by cargo already |
| `web/e2e/chat.spec.mjs` | 5 | 36 | 5 | 189 | gone: #189 deletes it |
| `web/e2e/plugins.spec.mjs` | 5 | 36 | 5 | 189 | gone: #189 deletes it |
| `web/e2e/scene.spec.mjs` | 5 | 40 | 9 | 189 | gone: #189 deletes it |
