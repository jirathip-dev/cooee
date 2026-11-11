Closes #218

## For humans
A persona can now keep skills: its own written-down ways of doing a task. Each new session's opening lists only their one-line descriptions, and the persona reads the full text when it needs it.
When a persona wants to add or change a skill, a card shows up in the chat with the full text, or for a change the lines that differ. Nothing is saved until you approve the card.
The native app can list, read, add, edit and delete a persona's skills. Clearing a persona's notes no longer deletes its skills.

## What changed
- `cooee-core/src/skills.rs`: `<data>/personas/<id>/skills/<name>/SKILL.md` (front matter `name`, `description` of at most 120 characters, then the body). Covers list (by name), read, check, save (new, or an edit of `replaces`, which may rename), remove, and a line diff. At most 20 skills; names are `[a-z0-9-]`, so a path can't reach outside the folder.
- `Persona::brief` adds `Your skills (read one with cooee's skill tool):` with one `- name: description` line per skill, sorted by name and capped at 20. Bodies never go in the brief, and the bytes stay the same while the files don't change.
- `personas::forget` (clear notes) now removes `memory.jsonl` only. It used to remove the whole persona data folder, which would have taken the skills with it. Deleting a persona still removes everything.
- MCP tool `skill {read?, propose?}` → `POST /mcp/agent/skill`. `read` returns the body, capped at 8,000 characters, from the thread's own persona only. `propose` raises a card with `form: "skill"`, and its detail is the new file or the edit's diff. Proposals share the pending map and answer path with `propose_server`. `approve` saves; `decline` or no answer saves nothing.
- API: `GET|POST /personas/{id}/skills`, `GET|PUT|DELETE /personas/{id}/skills/{name}`, documented in `docs/api.md`.
- `mcp::serve`'s tool definitions moved into `own_tools()` to keep it under clippy's line limit. Behaviour is unchanged.

Decided: approving a skill card is a plain tap. Face ID stays on MCP server cards only, since a skill runs nothing.

`src/ net +347 (budget: none stated)`: a new core module, the tool's hub path and five API routes. Total 25343 / 27500 after merging staging.

## Evidence (at head 05dabba41a80769f29fc4099a537f244397a9985)
Round 3: the round-2 FAIL was CI only. `dmg` was still pending, and `stable (ubuntu-latest)` failed in `crates/cooee/tests/chat_first.rs:164`, a sidebar test this PR doesn't touch. It checks for the unread dot on a side thread while the fake agent is still `working`, so it depends on timing. Locally it passed 5 of 5 runs (`cargo test -q -p cooee --test chat_first`, exit 0 each time). Staging moved (#253), so it is merged in again with no conflicts, and that push starts a fresh CI run.

| Command | Exit |
|---|---|
| `cargo test -p cooee --test skills` | 0 (2 passed) |
| `cargo test -q -p cooee --test chat_first` ×5 | 0 ×5 |
| `cargo fmt --all --check` | 0 |
| `just clippy` | 0 |
| `just test` | 0 (263 passed, 0 failed) |
| `just doc` | 0 |
| `just cap` | 0 (25343 / 27500) |
| `just deps` / `just hygiene` / `just neutral` / `just themes` | 0 / 0 / 0 / 0 |
| `just ts` | 1: `web` has no installed node modules on this host; this PR touches no `web/` file |
| `just deny` | 101: `cargo-deny` isn't installed on this host; no dependency changed |
| `just fmt` / `just check` | 127: `topcoat` isn't installed on this host; CI installs it |

Tests (`crates/cooee/tests/skills.rs`) drive the real `cooee mcp serve` with the thread's token, and the real approve route:
- an approved proposal is saved at the real path and listed in the next session's brief;
- a declined proposal saves nothing, and neither does one left unanswered;
- the brief holds the description only and stays byte-identical across sessions;
- the 21st skill is refused over both the API and the tool;
- ada's skill read from bo's thread is refused, and so is a `../` name;
- an edit shows as a diff and is saved only once approved;
- clearing notes keeps skills.

