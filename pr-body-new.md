Closes #416

## For humans

A chat with any persona, such as the Orchestrator, now starts clean: no personal CLAUDE.md, no personal plugins or skills, no claude.ai connectors (Docs, Drive, Microsoft 365, Supabase). The same goes for persona members of a room.
A plain chat with no persona still loads your personal Claude setup, as before.

## What changed

- `recipe` in `crates/cooee-server/src/lib.rs`: in the no-profile branch (`logins.home`), a thread with a persona gets the provider's `no_personal_setup()` options on `launch.meta`. A plain thread gets none. Room members start through the same `recipe` (`starter`), so they are covered too.
- Profile threads, including an owner profile read as a persona, keep #320's path: `Logins::profile` still sets the options. The new `Logins::no_personal_setup(agent)` helper is the lookup for the persona branch.
- Round 2: `arch-check` failed because `lib.rs` (over 800 lines) grew by 3. The persona folder match had two arms that differed only in the folder; folding them into one frees the line the new assignment takes. `lib.rs` now has exactly as many lines as staging (2320).

src/ net +6 (the issue gives no budget): a 6-line helper in `login.rs`; `lib.rs` net 0.

## Evidence at 9567e08daf1a33195e62c67d400d1da7d6e0c243

- `cargo test -p cooee --test profiles --test personas --test rooms --test chat` → exit 0. New test: `a_persona_with_no_profile_and_a_room_member_start_without_personal_setup_plain_keeps_it`. It checks the `shed run … --meta` line of a no-profile persona (options present), a plain thread (no `--meta`), and both members of a room: a no-profile persona and a profile read as a persona (options present).
- The same test run against staging's `crates/cooee-server` (`80c359e8a`) → exit 101: `#416: no options in … run --acp fake-acp …`.
- `cargo test --workspace --locked --no-fail-fast` → exit 101, from one test only: `accounts_env::ls_json_carries_usage_and_a_reading_past_its_reset_reads_zero_with_its_age` (`(12, 0)` vs `(12, 40)`). It fails the same way (exit 101) on a clean staging `80c359e8a` checkout, so this PR did not cause it. Every other test passes.
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `drover arch-check` (0 violations), `scripts/line-cap.sh`, `scripts/neutral-core.sh`, `scripts/hygiene.sh`, `scripts/dep-direction.sh` → all exit 0.
- CI's earlier `chat` failure (`SHED_VERSION` in `a_send_to_a_shed_on_another_protocol…`) was on the old base; after merging staging it passes locally.

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
