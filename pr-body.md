Closes #664

Built to the approved ADR in `docs/architecture/chief/` (q2346: slices A–D; the phone's Face ID proof is its own issue; adding a machine stays a Mac command).

## What changed
- `cooee setup` asks for Chief's agent and machine (no default) and makes `agents/chief/`; a re-run keeps it (`crates/cooee/src/setup/chief.rs`).
- The API refuses deleting Chief and no other persona can take its role; renaming works (`crates/cooee-core/src/chief.rs`, `routes/chief.rs`).
- Chief's tools (`crates/cooee-server/src/chief/tools/`): persona and routine create/edit/archive/pause apply at once; MCP servers, stored secrets, access (env/mcp), machine and `config.toml` changes make a Face ID card showing the exact change (a diff for files) and apply only on approval.
- `ask_persona` and `history` reach every persona from Chief's own chat; other personas still follow `can_message`. Chief can open a room; no tool sends as or approves for another persona. Only Chief's chat lists the tools.
- Untracked the fetched `packs/themes/` files a snapshot commit had added (staging fetches them from swag).

## Evidence (head 9305addd1fcc0f91b068c849a20f0c8772666055)
- `ci/gates.sh` → exit 1: only `deny` (`cargo deny` not installed on this host) and `fmt` (`topcoat` not installed); every other gate passed, workspace tests 458 passed 0 failed.
- `cargo fmt --all --check` → exit 0
- `cargo test -p cooee --test chief --test setup` → exit 0 (chief 7 passed, setup 16 passed)
- `drover arch-check` → exit 0 (0 violations)
- Not verified here: the phone check (Face ID card on a device) and the Swift app build; CI's apple job covers the latter.

src/ net +1315 (budget n/a): the new tools, the gate and the setup step; the cap holds (32142 / 36000).

## For humans
Cooee now has a built-in assistant persona called Chief that setup creates for you. In its chat you can ask it to make or change personas and routines and it does so at once; anything touching secrets, MCP servers, machines or the config file shows an approval card first and only happens after Face ID. Chief can be renamed but not deleted, and it can talk to every persona without acting as them.
