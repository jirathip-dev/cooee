# The capability table (#503)

What each harness supplies, what cooee replaces, and the exact switch. Evidence for every cell:

- **S** read in the source installed on this Mac: claude-agent-acp 0.101.0 (`dist/acp-agent.js`,
  `dist/session-mode.js`) and Agent SDK 0.3.318 (`sdk.d.ts`, `sdk-tools.d.ts`). Read, not run.
- **D** the vendor's docs, read 2026-11-17: codex-acp's README and `docs/permission-profiles.md`, Codex's config
  reference.
- **A** read in the source of `antigravity-acp` 1.5.2 as shed installed it here (`acp_server/request_meta.py`,
  `tool_filter.py`, `system_prompt.py`, `paths.py`, `server.py`, the SDK's `BuiltinTools` and `policy.py`), and
  Antigravity's docs (antigravity.google/docs/getting-started, /docs/install), read 2026-11-23. Read, not run.
- **P** not confirmed. Slice 1's nightly run proves or corrects it before the slice that relies on it merges.

Versions: `claude-acp` 0.101.0, `codex-acp` 2.5.0 and `antigravity-acp` 1.5.2 are installed in shed here.

## The table

| Capability | The contract | Claude Code | Codex | Antigravity |
|---|---|---|---|---|
| Loop, sign-in | harness's | kept | kept | kept |
| File and shell tools | harness's | kept | kept; every command runs in its OS sandbox | kept |
| Todo list | harness's, drawn from ACP `plan` | kept | kept | kept |
| System prompt | harness's; cooee adds, never replaces | preset kept S | kept | kept |
| The brief | once per session, frozen | system prompt, appended S | developer instructions D, P | system prompt, appended (`systemPrompt.append`) A |
| Global instruction file | not read | off S (#416) | `$CODEX_HOME/AGENTS.md`: no switch D | global config, skills, hooks and MCP under `GEMINI_HOME`: no switch A |
| Personal settings, plugins, MCP | not loaded | off S (#416) | `$CODEX_HOME/config.toml`: no switch D | load: no switch A |
| Project instruction file | harness's own, from the folder | `CLAUDE.md` | `AGENTS.md` | its own, from the folder |
| Web search and fetch | harness's until a web connector, then off | off by the tool list S | off D | off by `disabledTools` A |
| Browser | off; a connector | off by the tool list S | none | off with sub-agents, else admin-only A |
| Memory | off; `remember`, `history` | off S | off (its default) D | no switch: past conversations stay on A |
| Skills | off; cooee's `skill` | off by the tool list; personal ones gone with #416 S | bundled off D; folder ones P | no switch A |
| Sub-agents | off; `ask_persona` | off by the tool list S | off D | off by `disabledTools` A |
| Hooks | personal ones not loaded; a lent profile's guard only | the `settings` variable (today) | none: such a profile can't start (today) | global `hooks.json`: not used A |
| Permissions | ask for everything; cooee answers | reachable S | not reachable D | its default mode asks for all but read-only tools A |
| Asks before an MCP tool | yes | yes P | P | yes A |
| Compaction | harness's | own | own | own |
| Models | from the ACP session | yes | yes | P |
| Efforts | the provider's words (data) | none set | low, medium, high (today) | none |
| Context use | ACP `usage_update` | yes S | yes D | P |
| Plan usage windows | the provider's command | `/usage` (today) | none | none |
| Limit hit | shed's `rate-limited` and reset | shed has its line | shed has its line | no line yet: a data entry in shed |
| Hibernate and resume | shed, where the harness loads a session | yes | P | yes (`session/load`) A |

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

## Antigravity

`_meta.antigravity` on `session/new` (A, `request_meta.py`: "Clients send Antigravity-specific session options (e.g.
`enabledTools`, `disabledTools`, `eagerMcpServers`, `systemPrompt`) under `_meta.antigravity`"):

```json
{ "antigravity": { "systemPrompt": { "append": "<the brief>" },
                   "disabledTools": ["search_web", "read_url_content", "invoke_subagent", "run_workflow"] } }
```

- `disabledTools` is a deny-list of built-in names (A, `tool_filter.py`). Web: `search_web`, `read_url_content`; with
  no web connector the two stay on. Sub-agents: `invoke_subagent` and `run_workflow` ("Execute a multi-agent workflow
  script"); the browser sub-agent goes with them ("Browser subagent skipped because subagents are disabled by tool
  filter").
- **The brief: system channel.** `systemPrompt` takes only `{"append": ...}` (A, `system_prompt.py`): the vendor's
  prompt stays.
- **Memory: no switch.** No built-in tool is a memory, but the harness keeps past conversations on
  (`cascade-conversation-history-config`, a flag no client sets). Gap: `StillOn(Memory)`.
- **Skills: no switch.** Skills load from `<home>/config/skills`, `<home>/antigravity-cli/skills` and the folder's
  `.agents/skills` on every session (A, `paths.py`, `server.py`); the filter can't name them. Gap: `StillOn(Skills)`.
- **Global files: no switch.** Everything is under `GEMINI_HOME` (A, `paths.py`): clean in an account cooee added, the
  owner's own on the default login. Gap: `PersonalFiles` there.
- **Ask for everything: its default.** The default mode wires `policy.safe_defaults`: "Allows all read-only tools and
  asks the user for any other tool calls" (A). `view_file` and `read_url_content` are read-only, so they never ask.
  Gaps: `AsksNot(Read)`, `AsksNot(Search)` (its directory tools' policy is not read), and `AsksNot(Fetch)` while web
  stays on.

## Any other agent

A `[[providers]]` row with no registered module is `Plain`: `session()` returns nothing to set, the brief as the first
prompt, and every gap. It runs, cooee's rules answer what it asks, and the agent picker says what it doesn't do.
