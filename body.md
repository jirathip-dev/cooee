Closes #417

## For humans

A persona that points at a profile and leaves its Instructions empty now uses that profile's instructions, whether the profile is your own or a plugin's (such as drover's).
In the app's persona editor, the Instructions box for such a persona reads "from profile …" and can't be edited.
A persona with its own instructions is unchanged.

## What changed

- `crates/cooee-server/src/lib.rs` `recipe()`: when the persona's `instructions` is blank, it takes the instructions text the profile's recipe already read, and the brief is built as before (character, instructions, notes, skills). This covers owner and plugin profiles the same way.
- `apple/Sources/CooeeUI/BlockPersonaEdit.swift (was PersonaSheet.swift before staging renamed it)`: if `profile` is set and `instructions` is empty, Instructions shows `from profile <id>` as read-only text.
- Tests: `personas.rs::a_persona_without_instructions_takes_its_profiles` (profile + no instructions → the profile's text, then notes; profile + own instructions → its own). `profiles.rs::a_plugins_profile_starts…` gains a step where a persona names `plug/conductor` and gets the plugin profile's instructions.

Decided: the label shown is the profile id (e.g. `drover/cooee`). The app has no profile list. A display-only label field on `Persona` would also end up in the saved TOML.

src/ net +11 (no budget stated): 3 lines of Rust, 8 of Swift view code.

## Evidence (head e896afd4984c2800a28d9b3a1bc369d745d7b1c3)

Update round: merged `origin/staging` (conflicts in `apple/Sources/CooeeUI/BlockPersonaEdit.swift` and `crates/cooee/tests/personas.rs`, both sides kept). Staging's comment rule (#201) meant dropping this PR's test comments. Staging's chat style adds a plain line to briefs, so the #417 test personas now set `style = "detailed"`.

- `cargo test -p cooee --test personas --test profiles` → exit 0 (12 + 6 passed)
- `ci/gates.sh test clippy comments doc neutral hygiene` → exit 0
- `ci/gates.sh` (all) → exit 1, none of it from this PR: `deny` (cargo-deny isn't installed on this host), `fmt` (topcoat isn't installed; `cargo fmt --all --check` → 0), `cap` (its `find` counts the untracked `apple/.build/`; the tracked apple Swift is about 13.2k / 14316 and Rust is 20109 / 27500), and one `workflow` test that failed under load and passed on its own (`cargo test -p cooee --test workflow` → exit 0)
- At round 1 (`1695bf6`): the new tests failed at base `06f657513` with the lib.rs change reverted (exit 101)
- `scripts/apple-check.sh` not rerun this round; CI's `apple` job covers it

## Why (round 1)

Decided: in recipe(), a persona with blank instructions takes the instructions text its profile's recipe already read, then builds its brief as before
Because: "has empty `instructions` gets the profile's instructions file text in that place of its brief"
Decided: the fallback lives where the persona's brief replaces the profile's, so owner and plugin profiles share one path
Because: crates/cooee-server/src/lib.rs:625
Decided: the editor shows "from profile <id>" read-only when the persona names a profile and has no instructions; the app holds no profile list, so the id (e.g. drover/cooee) is the label shown
Because: apple/Sources/CooeeUI/PersonaSheet.swift:134
Rejected: a profile_label field on Persona — Persona is also the saved TOML, so a display-only field would leak into owner files
Rejected: reading the profile's file again in Persona::brief — the recipe already read it, with its error refused at start
Check: a_persona_without_instructions_takes_its_profiles and the plugin step in a_plugins_profile_starts fail at base 06f657513 and pass at the head
Owner: A persona that points at a profile and leaves Instructions empty now uses that profile's instructions, so the Orchestrator can follow drover's rules without its own copy.

<sub>note on 1695bf6 · git log --notes=why</sub>

