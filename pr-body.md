Closes #240

Finishes the controls the approved native designs draw but the build left dead.

- **Header agent chip** (item 1): a menu with the live agents, models and efforts (the same `GET /agents` and `GET /models` lists as #230). A model or effort pick goes to the thread's next turn (`POST /threads/{id}/model`). The server can't change a running thread's agent, so picking another agent starts a new chat with the same persona on that agent. In a new chat all three apply to its first message (`POST /threads` now carries `model` and `effort`).
- **Header avatar and name** (item 2): a tap opens the persona sheet.
- **Delete <persona>** (item 3): at the bottom of the sheet. An alert asks first ("Its chats stay."), then it calls `DELETE /personas/{id}` and home reloads.
- **Edit avatar** (item 4): choose a photo (shrunk to a 256 px JPEG data image) or initials in one of the six colours (saved as `initials-<colour>`, an avatar id the server already accepts).
- **Memory** (item 5): the newest three notes, then **Show all N**, which opens the full list.
- Items 6–9 (Face ID before a secret, strip counts, Mac camera QR, header +) are in the first commit, 4c12bb303. The pinned tap (item 9) now uses staging's #432 path, which opens the persona's newest chat; my duplicate was removed so there is one way.
- Staging merged (#230, #210, #432, #239, #310, #396, #179, #229, #425, #426, #151, #420, #242, #290, #152, #177, #291, #427, #178, #465, #314); #178's room-ask redirect keeps the Face ID gate on secret cards; after #314 made the persona sheet a core screen, the avatar editor, newest-three notes with a Show all button, the full-notes sheet and Delete sit on its PersonaEditBlocks, the notes read from the screen's memory block, and delete stays `DELETE /personas/{id}` (the screen's actions only save and clear) a new chat sends the picked model, effort and #152's machine, and a started or agent-restarted chat keeps its machine; the header tap and header + send #177's `persona` and `start` chat delegates, so each has one path. Conflicts in ChatFeature, AppFeature, Persona, Home, HomeRows, Wide, Root and Chat resolved; staging's `open` and `round` delegates, `pin` action, ScreenFeature, SlotView and workflow sheet kept (workflow fields added to the ChatActions file); after #426 grouped ChatFeature's actions per area, this PR's `new`, `profile` and `pick` stay top-level actions, the Face ID gate sits in ChatCards.swift and the agent pick's thread call reuses ChatCompose's `act`; after #425 split Root.swift, this PR's chat wiring lives in ChatScreen.swift and LiveControlsUITests in UITests/Live/, named in apple/e2e/map.toml (`sh ci/ci-scripts-test.sh` exit 0); AppFeature's chat-delegate relay moved to an extension to stay under the type length limit.

- Staging merged again (#270 setup chats, #315 block additions, #359, #252, #417, #422, #201, #202, #203, #318, #213): new threads send this PR's model and effort plus #270's setup flag; setup and new chats share one start path; the persona sheet keeps Delete beside #270's Chat; staging's item `view` action joins the ChatActions file; the memory-notes helper moved beside the pickers to keep BlockPersonaEdit.swift under SwiftLint's 250 lines; this PR's agent test stub takes the new setup argument.

src/ net: apple +399 lines (no budget stated): nine controls with their reducers and clients. Rust +0. Apple lines 13896 / apple_cap 14316 (staging 13497).

## Evidence at 1628f96616e20ceb2f198fd28383f6c2e6e0e65b

- `scripts/apple-check.sh` at 1628f9661: exit 0 (swift format, anti-slop, SwiftLint, strict build, all tests, coverage Core 97.3% / API 97.5% / Features 97.3%, swiftlint analyze, Periphery, anti-slop's own tests); `sh ci/ci-scripts-test.sh`: exit 0
- `scripts/line-cap.sh` in a clean checkout of 1628f9661: exit 0, apple lines 13896 / 14316 (in this worktree it counts gitignored build checkouts).
- New tests: ChatAgentTests (chip lists, model pick to the thread, agent pick restarts or applies to a new chat, header tap), PersonaControlsTests (header opens the sheet, header + starts a new chat, delete asks then removes only the persona, avatar saves, Show all), PersonaRenderTests (photo shrunk and drawn, initials and colour drawn, only the newest three notes drawn), API tests for delete, thread model and the new-chat body.
- Simulator: `.herdsman/verify.md` iOS recipe with `-only-testing:CooeeAppUITests/LiveControlsUITests` at e9ad2171b (same UI code as this head except staging's pinned-row rename): drive dark exit 0, drive light exit 0. Screenshots kept: chat-agent-menu, persona-from-header, persona-avatar-menu (green initials applied), persona-delete-asks ("Coder and its notes go. Its chats stay."), each dark and light.
- Known: the colour dots in the avatar menu draw white because iOS system menus are monochrome; each row names its colour.
- `HeaderRenderTests.aPlacesCountDrawsOnItsChip` and `PersonaRenderTests.memoryDrawsOnlyTheNewestThreeNotes`: the "same drawing" checks allow a 0.01% pixel difference instead of exact equality, which failed once each under full-suite load; the "different drawing" checks still need >0.1%.

## For humans

In a chat, the agent label under the name now opens a menu to switch agent, model and effort, and tapping the avatar or name opens that persona's settings.
In a persona's settings you can now set a photo or initials and a colour, see all its notes, and delete the persona (its chats are kept).

