Closes #84

## For humans

Every chat's header now shows its id in small text next to the title.
A thread id written in a message, a handoff line or a question opens that thread in place, and the header's back arrow (or the browser's back) returns to the thread you came from.
Ids inside code quotes, ids glued to other text, and ids of threads that don't exist stay plain text.

## What changed

- `web/src/lib.rs`: the thread bar shows the id (`.tn small`) beside the title; the page carries every thread's id (`data-ids`) for the refs. The back arrow's label is now just "back".
- `crates/cooee-server/src/lib.rs`: the `/t/{id}` page passes `hub.list()`'s ids.
- `web/ts/chat.ts`: `refs()` links a whole-word `t…` id that cooee knows in messages (you/agent), ask questions, ask option labels (a link after the option buttons), the ✓ answer line and handoff/line rows; code spans (`` `…` `` and ``` fences) are left as they are. The back arrow goes to the thread the page was opened from (same-origin `/t/` referrer), else the list.
- `web/src/app.css`: one line for the title row.
- `web/e2e/cards.spec.mjs`: new test at 390 and 1440 (header id; a ref in a handoff line and a card question and in a card option is linked, and the ref opens the thread; a code-span, a partial-word (`x<id>`) and an unknown id (`t0ffff`) stay plain; back returns to the first thread). Shots: `ui-390/refs-header.png`, `ui-390/refs-tapped.png`.

## Decisions

- **Not in this PR (waits for herdsman#618, per the issue's scope item 5):** refs to herdsman conductor ids (t27) and the archived thread opening read-only with [restore]. cooee has no archive state of its own; both need #618's thread id + archived fields. Until then a t27 that cooee doesn't know stays plain text (the "unknown" rule).
- A ref in an open option's label is linked after the option buttons, not inside its button (a link inside a button is a nested control axe rejects); once answered, the ✓ line links it too.
- Merged origin/staging (agent profiles, #40): `chat_page` now takes both the profile label and the ids.

## Evidence (head a24dd97b979d31de289fc8104009d70e9f7b7a0d)

- At a24dd97: `just fmt` 0, `just cap` 0. Earlier at 9f3743d (code unchanged since but for formatting): `just clippy` 0, `just test` 0, `just doc` 0, `just deny` 0, `just cap` 0 (non-test lines 12955 / 15500), `just deps` 0, `just hygiene` 0, `just ts` 0 (cargo with `CARGO_TARGET_DIR=$HERDSMAN_SCRATCH_TARGET/target`)
- `just fmt` 0 (`cargo fmt --all --check` and `topcoat fmt --check`, topcoat-cli 0.11.0 as CI pins, installed under the lane's scratch dir): fix round 1 reformatted `chat_page`'s `view!` (whitespace only)
- `npx playwright test` with a local-only config swapping the `pair` project to Chromium and dropping the `webkit` project (WebKit isn't installed here and lanes may not install browsers): 98 passed, 7 skipped, exit 0. CI runs the real config.

src/ net +43 (budget: none stated): the id in the header, the ref linker, the back target and the server passing ids.

Not verified: AC4 (live on the owner's phone).

