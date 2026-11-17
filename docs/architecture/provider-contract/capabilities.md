# The capability table (#503)

What each harness supplies, what cooee replaces, and the exact switch. Evidence for every cell:

- **S** read in the source installed on this Mac: claude-agent-acp 0.101.0 (`dist/acp-agent.js`,
  `dist/session-mode.js`) and Agent SDK 0.3.318 (`sdk.d.ts`, `sdk-tools.d.ts`). Read, not run.
- **D** the vendor's docs, read 2026-11-17: codex-acp's README and `docs/permission-profiles.md`, Codex's config
  reference; Gemini CLI's configuration, policy engine and ACP mode pages.
- **P** not confirmed. Slice 1's nightly run proves or corrects it before the slice that relies on it merges.

Versions: `claude-acp` 0.101.0 and `codex-acp` 2.5.0 are installed in shed here; no Gemini agent is.

## The table

| Capability | The contract | Claude Code | Codex | Gemini CLI |
|---|---|---|---|---|
| Loop, sign-in | harness's | kept | kept | kept |
| File and shell tools | harness's | kept | kept; every command runs in its OS sandbox | kept |
| Todo list | harness's, drawn from ACP `plan` | kept | kept | kept |
| System prompt | harness's; cooee adds, never replaces | preset kept S | kept | kept |
| The brief | once per session, frozen | system prompt, appended S | developer instructions D, P | first prompt: no channel that keeps the vendor's prompt D |
| Global instruction file | not read | off S (#416) | `$CODEX_HOME/AGENTS.md`: no switch D | `.gemini/GEMINI.md` under `GEMINI_CLI_HOME`: no switch D |
| Personal settings, plugins, MCP | not loaded | off S (#416) | `$CODEX_HOME/config.toml`: no switch D | user settings load; cooee's system settings file wins D |
| Project instruction file | harness's own, from the folder | `CLAUDE.md` | `AGENTS.md` | `GEMINI.md` |
| Web search and fetch | harness's until a web connector, then off | off by the tool list S | off D | off by the tool list D, names P |
| Browser | off; a connector | off by the tool list S | none | off by the tool list D |
| Memory | off; `remember`, `history` | off S | off (its default) D | off by the tool list D, P |
| Skills | off; cooee's `skill` | off by the tool list; personal ones gone with #416 S | bundled off D; folder ones P | off D |
| Sub-agents | off; `ask_persona` | off by the tool list S | off D | off D |
| Hooks | personal ones not loaded; a lent profile's guard only | the `settings` variable (today) | none: such a profile can't start (today) | possible through the system settings file D; not used |
| Permissions | ask for everything; cooee answers | reachable S | not reachable D | unproven P |
| Asks before an MCP tool | yes | yes P | P | yes D |
| Compaction | harness's | own | own | own |
| Models | from the ACP session | yes | yes | P |
| Efforts | the provider's words (data) | none set | low, medium, high (today) | none |
| Context use | ACP `usage_update` | yes S | yes D | P |
| Plan usage windows | the provider's command | `/usage` (today) | none | none |
| Limit hit | shed's `rate-limited` and reset | shed has its line | shed has its line | no line yet: a data entry in shed |
| Hibernate and resume | shed, where the harness loads a session | yes | P | yes D |

## Claude Code

All in `--meta`, as today's `no_personal_setup` (shed sends it as `_meta` on `session/new`, `session/load` and every
wake):

```json
{ "systemPrompt": { "append": "<the brief>", "excludeDynamicSections": true },
  "claudeCode": { "options": {
    "settingSources": ["project", "local"],
    "strictMcpConfig": true,
    "tools": ["Read", "Write", "Edit", "NotebookEdit", "Glob", "Grep", "Bash", "TaskStop",
              "TodoWrite", "TaskCreate", "TaskGet", "TaskUpdate", "TaskList"],
    "settings": {
      "claudeMdExcludes": ["{/Users/*,/home/*,/root}/.claude/{CLAUDE.md,rules/**}"],
      "autoMemoryEnabled": false,
      "permissions": { "defaultMode": "default", "disableAutoMode": "disable",
                       "disableBypassPermissionsMode": "disable",
                       "ask": ["Read", "Glob", "Grep", "Bash", "Write", "Edit", "NotebookEdit"] } } } } }
```

- `systemPrompt`: the adapter keeps the `claude_code` preset and forwards `append` and `excludeDynamicSections` (S,
  `acp-agent.js` 9077–9091). The second moves the working folder and git status out of the system prompt into the
  first user message, so the vendor's prompt plus the brief is one static prefix.
- `tools` is the SDK's base set of built-ins (S, `sdk.d.ts` 1709). Everything not named is off: `WebSearch`,
  `WebFetch`, `Agent`, `Workflow`, `Skill`, the cron, monitor and worktree tools. With a web connector absent,
  `WebSearch` and `WebFetch` join the list. The names are from SDK 0.3.318's tool list; the slice confirms each (P).
- `permissions.ask`: an ask rule sends even a read to the client (S: `matchedAskRule` reaches `canUseTool`).
  `defaultMode` matters: without it the adapter picks `auto` when the account offers it (S, `session-mode.js` 47–49).

## Codex

In the adapter's environment (`Launch.env`):

```text
INITIAL_AGENT_MODE=read-only
CODEX_CONFIG={"developer_instructions":"<the brief>","approval_policy":"on-request",
              "tools":{"web_search":false},"features":{"multi_agent":false,"memories":false},
              "skills":{"bundled":{"enabled":false}}}
```

- `CODEX_CONFIG` is "a JSON object merged into the Codex session config"; `INITIAL_AGENT_MODE` takes `read-only`,
  `workspace-write`, `agent`, `agent-full-access` or a permission profile (D, the adapter's README). That
  `developer_instructions` arrives this way is P.
- **Global files: no switch.** Codex reads `AGENTS.md` and `config.toml` from `$CODEX_HOME`. An account cooee added has
  its own `CODEX_HOME` with neither; the default login (`~/.codex`) is the owner's own. Gap: `PersonalFiles` on the
  default login.
- **Ask for everything: not reachable.** In `read-only` with `on-request`, an edit and a command that writes or uses
  the network ask; a read, and any command that stays inside the read-only sandbox, run unasked. The `untrusted`
  policy that asked for more is no longer supported (D). Lost: `ask` and `deny` rules on reads, searches and
  side-effect-free commands do nothing on Codex; its sandbox, not a rule, is what stops a write. Gaps:
  `AsksNot(Read)`, `AsksNot(Search)`, and `AsksNot(Run)` for sandboxed commands.
- The effort words stay `-c model_reasoning_effort={}` as today. Adapter 2.5.0 runs Codex's app server, so whether
  that argument still reaches it is P.

## Gemini CLI

A settings file cooee writes under the session's run folder, named by `GEMINI_CLI_SYSTEM_SETTINGS_PATH` (the system
settings file "takes precedence over all other settings files", D), and a policy file it names:

```json
{ "tools": { "core": ["read_file", "write_file", "replace", "glob", "search_file_content", "list_directory",
                      "run_shell_command", "write_todos"] },
  "skills": { "enabled": false },
  "experimental": { "enableAgents": false },
  "security": { "disableYoloMode": true, "disableAlwaysAllow": true },
  "adminPolicyPaths": ["<run>/ask-all.toml"] }
```

```toml
[[rule]]
toolName = "*"
decision = "ask_user"
priority = 999
```

- `tools.core` is the allow-list of built-ins (D); the tool names are P. Web search and fetch, `save_memory` and the
  browser agent are off by not being in it; with no web connector the two web tools join the list.
- **The brief: first prompt.** `GEMINI_SYSTEM_MD` replaces the vendor's system prompt, which the contract forbids, and
  a context file would have to be written into the owner's folder. So the brief is sent ahead of the first message,
  as today, and may be summarised by a later compression.
- **Global files: no switch.** `GEMINI.md` and `settings.json` are read from `.gemini` under `GEMINI_CLI_HOME`. As
  Codex: clean in an account cooee added, the owner's own on the default login. Gap: `PersonalFiles` there.
- **Ask for everything: unproven.** A policy rule can ask for any tool (D), but the docs don't say the policy engine
  runs in ACP mode, and say `ask_user` is a deny in non-interactive mode. If the probe shows either, cooee drops the
  policy file and Gemini's `default` mode stays: edits, shell, web fetch and MCP tools ask; reads and web search
  don't. The gaps are then `AsksNot(Read)` and `AsksNot(Search)`.

## Any other agent

A `[[providers]]` row with no registered module is `Plain`: `session()` returns nothing to set, the brief as the first
prompt, and every gap. It runs, cooee's rules answer what it asks, and the agent picker says what it doesn't do.
