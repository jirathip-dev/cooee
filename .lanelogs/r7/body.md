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

## Evidence (head 3bbf80490a7d92836067231888a2f84f67f04f3e)
Fix round: CI's SwiftLint failed `type_body_length` on `ClientTests`, `AppFeatureTests` and `ChatFeatureTests` (157–167 lines, limit 150). This PR's three new setup-chat tests now sit in a same-file `extension` of each suite, so each struct body is back to staging's size. The branch is also merged with current origin/staging (clean).

src/ net +188 (budget: none stated in the issue). It covers the setup flag, the `update_persona` endpoint and its approval card.

- `scripts/apple-check.sh` → 0 (swift format, anti-slop, SwiftLint: no violations, strict build, tests with coverage)
- `cargo fmt --all --check` → 0
- `sh ci/gates/clippy.sh` → 0
- `sh ci/gates/{comments,doc,neutral,hygiene,bans,lessons,themes,ci-scripts,deps}.sh` → 0
- `sh ci/gates/test.sh` → 101: one failure, `reliability::a_background_start_answers_before_the_agent_runs_and_sends_once` (runs 2 vs 1 under the full suite's load). It then passed 3 of 3 alone (`cargo test -p cooee --test reliability a_background_start` → 0). This PR only carries the setup flag on that path. `cargo test -p cooee --test mcp --test threads` → 0.
- `scripts/line-cap.sh` on a clean `git archive HEAD` → 0 (non-test 20625 / 27500, apple 13497 / 14316)
- **Not run locally:** `topcoat fmt` (not installed here). CI runs it.

