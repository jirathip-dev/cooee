Closes #500

## For humans

When you take control of a persona's browser and hand it back, you can now ask it to save what you did as a skill.
The persona gets the list of your steps (which page, what you clicked, which field you typed into) but never what you typed.
On a site marked "no frames" or your real account, only the site's address is listed. Nothing is saved until you approve the skill card; declining saves nothing.

## What changed

- `crates/cooee-core/src/browser.rs`: `Did` and `step()`: one lease log line = origin + title + `click "<label>"` / `type into "<field>"`; a `no_frames` or `account` site gives its origin only. `framed()` reuses the same site-mark helper.
- `crates/cooee-server/src/hub/screen.rs`: each owner input under the lease reads the label of its target over CDP (`Runtime.evaluate`: label, aria-label, placeholder, name, text of non-editable elements, id; never `.value`) and appends a step; `take_control` starts a fresh log; `hand_back(…, skill)` appends the ask to propose a skill with the `skill` tool plus the numbered steps to the note the persona's next browser call carries.
- `routes/screen.rs`: `POST /personas/{id}/screen/back {note, skill}` (`skill` defaults to false).
- `docs/mcp.md`: one sentence on the flag.

Decided: the steps ride the existing hand-back note (W3's path to the waiting call) rather than a new queued message; the proposal and its card are #218's existing `skill propose` flow. No app UI yet: there is no Screen view in the app to put the choice on, so this is the API.

src/ net +66 (budget: none stated): the lease log, label lookup and hand-back text. Total 28328 / cap 28390.

## Evidence (head 2ddb75df8108805cea47c99a27acf9e30afc6eef)

- `sh ci/gates/test.sh` → exit 0 (workspace tests; new `a_lease_saved_as_a_skill_hands_the_persona_its_steps_and_no_typed_value_and_a_declined_card_saves_nothing` in `crates/cooee/tests/mcp.rs`, new unit test `a_step_names_the_field_and_a_no_frames_or_account_site_gives_its_origin_only` in `browser.rs`)
- `sh ci/gates/clippy.sh` → 0, `cargo fmt --all --check` → 0, `sh ci/gates/comments.sh` → 0, `bans.sh` → 0, `neutral.sh` → 0, `cap.sh` → 0, `hygiene.sh` → 0, `doc.sh` → 0, `drover arch-check` → 0
- `ci/gates/fmt.sh` → 127 locally only because `topcoat` isn't installed on this host; its `cargo fmt` half passed; CI runs the rest.
