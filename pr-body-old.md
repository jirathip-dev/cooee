Closes #416

## For humans

A chat with any persona, such as the Orchestrator, now starts clean: no personal CLAUDE.md, no personal plugins or skills, no claude.ai connectors (Docs, Drive, Microsoft 365, Supabase). The same goes for persona members of a room.
A plain chat with no persona still loads your personal Claude setup, as before.

## What changed

- `recipe` in `crates/cooee-server/src/lib.rs` sets the provider's `no_personal_setup()` options on `launch.meta` for any thread with a persona or a profile. Plain threads get none. Room members start through the same `recipe` (`starter`), so they are covered too.
- `Logins::profile` no longer sets meta itself. That logic moved into the one place above, behind a new `Logins::no_personal_setup(agent)`, so a profile thread (including an owner profile read as a persona) behaves as it did under #320.

src/ net +7 (the issue gives no budget). It is one condition plus a lookup helper; the profile path loses its own copy.

## Evidence at 7d8863e067b3238a66f3117691915a204641af7d

- `cargo test -p cooee --test profiles --test personas --test rooms` → exit 0. New test: `a_persona_with_no_profile_and_a_room_member_start_without_personal_setup_plain_keeps_it`. It checks the `shed run … --meta` line of a no-profile persona (options present), a plain thread (no `--meta`), and both members of a room: a no-profile persona and a profile read as a persona (options present).
- The same test without the production change (base `crates/cooee-server` plus the new test) → exit 101: `#416: no options in … run --acp fake-acp …`.
- `cargo test --workspace --locked` → exit 0.
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `scripts/line-cap.sh` (19097 / 27500), `scripts/neutral-core.sh`, `scripts/hygiene.sh`, `scripts/dep-direction.sh`, `drover arch-check` → all exit 0. `topcoat` and `just` are not installed on this host, so `topcoat fmt --check` did not run.

### Real session (AC 3)

I ran this branch's `cooee serve` as a scratch instance on the mini: the real shed, the default claude login, and a copy of `personas/orchestrator.toml` without its `plugins` line (drover is not installed in a scratch instance). I asked the same question in two threads: whether a global CLAUDE.md is loaded, which MCP servers and connectors it has, and which plugins and skills.

**Orchestrator persona (no profile):**
> 1) Global CLAUDE.md: None. No `~/.claude/CLAUDE.md` content is loaded in my context.
> 2) There is one MCP server, `cooee`, under the prefix `mcp__cooee__` … No claude.ai connectors are present: no Docs, Drive, Microsoft 365, Supabase or anything else.
> 3) No plugins are loaded. The skills available are: dataviz, update-config, keybindings-help, code-review, simplify, fewer-permission-prompts, loop, schedule, claude-api, workflow-authoring, run, plugin-authoring, init, security-review

These skills are Claude Code's bundled ones. None of the owner's personal plugins (vercel, the LSP plugins) or synced `anthropic-skills:*` appear.

**Plain thread, same login (control):**
> 1) Yes, `~/.claude/CLAUDE.md` is loaded. Its first line is `# Global preferences`.
> 2) Connected: claude.ai Claude Docs … cooee. Listed but not authorized: claude.ai Google Drive, claude.ai Microsoft 365, claude.ai Supabase, plugin:vercel:vercel
> 3) Vercel plugin (vercel:* skills and agents), cowork-plugin-management, anthropic-skills: …

So `strictMcpConfig` does drop the claude.ai connectors (q1701), and no follow-up fix is needed. Both test terminals were removed from shed afterwards.

## Why (round 1)

Decided: recipe sets the provider's no-personal-setup options for any thread with a persona or a profile; Logins::profile no longer sets them itself
Because: crates/cooee-server/src/lib.rs:619
Because: "every persona chat starts without the owner's personal setup; only a plain chat with no persona keeps it"
Decided: room members are covered by the same line, since starter builds their recipe through recipe()
Because: crates/cooee-server/src/lib.rs:776
Rejected: setting meta in the persona branch while also keeping it in Logins::profile — two places doing one thing
Rejected: a meta flag on Logins::home — the plain path must stay unchanged, and the caller already knows whether a persona is set
Check: profiles.rs a_persona_with_no_profile_and_a_room_member_start_without_personal_setup_plain_keeps_it fails with base crates/cooee-server
Check: a real Orchestrator session on the default login reports no global CLAUDE.md, no plugins and no claude.ai connectors; a plain thread shows all three
Owner: Persona chats and room members now start without your personal Claude setup and connectors; plain chats keep it.

<sub>note on 7d8863e · git log --notes=why</sub>

