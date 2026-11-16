Closes #270

## For humans
A persona can now be set up by chat. The persona form has a "Set up by chat instead" link (a saved persona has "Refine by chat"); it saves the persona and opens a chat in which the persona interviews you and writes its own Character and Instructions.
In that setup chat each change is saved at once and shows an "Updated …" line with View. In any other chat, a persona that wants to change itself shows you a card with the diff, and nothing changes until you approve.
Changes reach the persona's next session, not the one in progress.

## What changed
- **Setup mode**: `POST /threads` takes `setup: true` (a 400 without a persona); the thread keeps it, `Row.setup` shows it, and its brief gets one interview line after the persona's Character, Instructions and notes (`personas::SETUP`). The brief is still built only at session start.
- **`update_persona {character?, instructions?}`** on cooee's own MCP server → `POST /mcp/agent/persona` (`crates/cooee-server/src/hub/setup.rs`). It acts only on the calling thread's own persona: the arguments have no field that names a persona (`deny_unknown_fields`), and the thread's token is checked. In a setup thread it saves at once and posts `Updated Character` / `Updated Instructions` notes. In any other thread it raises an approval card (`form: "persona"`, `detail` = the diff, `approve` / `deny`), and saves only on approve. Deny saves nothing, and so does a card left unanswered.
- **Apple**: `PersonaFeature.chat` saves, then delegates `.chat(persona)`. `AppFeature` opens a `ChatFeature` with the new target `.setup(persona)`, whose first message starts the thread with `setup: true`. The sheet's link reads "Set up by chat instead" (new) or "Refine by chat" (saved). An `Updated …` note has a View button that opens the persona sheet.
- Docs (staging split `docs/api.md` into one doc per area): `setup` on `POST /threads` and `Row` in `docs/chat.md`, the tool in `docs/personas.md`, `/mcp/agent/persona` in `docs/mcp.md`.

## Decisions
- `update_persona` takes no persona argument, so a thread can't name another persona. An unknown field is a 400 (the tool's error), not a silent ignore.
- The new sheet link is hidden in design stills (`look.still`), like "Pin to Home" and the Clear memory menu, so the `persona-*` design frame stays unchanged. This host has no Xcode and can't re-record snapshots.
- An unanswered card is held in memory, the same as `propose_server`'s; after a restart, approving it says it ran out.

## Evidence (head db07a247e6d4197d09bf52107d410e227a047d1d)
Update round 9: merged origin/staging (#435, a persona with no instructions takes its profile's). The one conflict was in `BlockPersonaEdit.swift`. It keeps staging's `instructions` view and this PR's "Set up by chat instead" / "Refine by chat" link below it. Round 8 moved this PR's `docs/api.md` edits into `docs/chat.md`, `docs/personas.md` and `docs/mcp.md` after staging's docs split (#444).

src/ net +188 (budget: none stated in the issue). It covers the setup flag, the `update_persona` endpoint and its approval card.

- `cargo fmt --all --check` → 0
- `sh ci/gates/clippy.sh` → 0
- `cargo test --workspace --locked` → 0 (incl. `a_setup_chat_interviews_and_its_update_persona_saves_at_once_for_the_next_session`, `outside_setup_update_persona_is_a_card_saved_only_on_approve_and_never_for_another_persona`)
- `cd apple && COOEE_STRICT=1 swift test` → 0
- `swift format lint --strict apple/Sources/CooeeUI/BlockPersonaEdit.swift` → 0
- `sh ci/gates/{doc,hygiene,comments,neutral}.sh` → 0
- `scripts/line-cap.sh` on a clean `git archive HEAD` → 0 (non-test 20283 / 27500, apple 13202 / 14316)
- **Not run locally:** `topcoat fmt`, SwiftLint and Periphery (not installed here), xcodebuild. CI runs them.

## Why (round 5)

Decided: merge origin/staging; the persona sheet keeps staging's instructions view and this PR's chat link below it
Because: apple/Sources/CooeeUI/BlockPersonaEdit.swift:117
Rejected: drop staging's instructions view for the old paragraph — that would undo #435's profile fallback
Check: the persona sheet shows Instructions, then Set up by chat instead / Refine by chat
Owner: Same feature; this round catches up with staging, where an empty persona's instructions come from its profile.

<sub>note on db07a24 · git log --notes=why</sub>

