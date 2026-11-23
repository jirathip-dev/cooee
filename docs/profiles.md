# Agent profiles (#40)

## Today

- **Secrets:** kept in the macOS keychain, readable only by the signed cooee (#212).
- **Samples:** twelve ready agents in [examples/agents/](../examples/agents/), built in: `cooee agent add --list` lists them, `cooee agent add --from <sample> [--id <id>]` copies one in (never over a taken id). One whose MCP server isn't in the config says in its first reply what to connect (#362).

## How it works

An agent is a named recipe for a thread's agent, picked when the thread starts and fixed for its life. **Plain**, the
default, is the agent with the owner's own setup and nothing added; an agent starts without the owner's personal
setup. The owner's agents are folders `<config>/agents/<id>/` (#359; the folder's name is the id), so the whole config
folder is one hand-editable, git-able tree, read on each call:

```text
agents/kai/agent.toml           every structured key (below); an unknown key, or `id`, `character`, `instructions`, is malformed
agents/kai/INSTRUCTIONS.md      plain Markdown, read as-is (missing: empty)
agents/kai/CHARACTER.md         plain Markdown, read as-is
agents/kai/memory.md            - [3f] one note per line <!-- by:persona at:1731283200000 -->
agents/kai/skills/release/SKILL.md
```

```toml
name = "Kai"                    # shown; never the id
agent = "claude-acp"            # the ACP agent
role = "orchestrator"           # optional, with style, fleets, avatar, folder, model, effort, machine, plugins, can_message, pinned
account = "work"                # optional: its provider's account (else the one the chat names)
settings = "settings.json"      # optional: an agent settings file, its hooks the agent's guard
policy = "ask"                  # optional: shed's approval policy, ask, allow or deny

[[fallback]]                    # optional, repeatable, in order: at a usage limit a card offers these (#76)
agent = "codex-acp"             # the ACP agent, with `account` optional

[env]                           # optional: plain values ({thread}: the thread's id), secrets as secret:<NAME>
API_KEY = "secret:API_KEY"

[[mcp]]                         # optional, repeatable: an MCP server from the registry (#56)
server = "github"
tools = ["get_issue"]           # optional: the tools it may use (none: all)
```

A malformed `agent.toml` hides that agent only; the log names its file and the line or key, never its text. The API
never sets the launch keys (`account`, `settings`, `env`, `mcp`, `policy`, `fallback`): they are edited by hand. A plugin brings
its own as `[[profiles]]` in its `cooee-plugin.toml` ([plugins/](plugins/README.md)), lent as read-only agents
`<plugin>/<name>` while it is installed and on. The first `cooee serve` of this build (or `cooee config migrate`;
`--dry-run` prints the plan and writes nothing) moves the old `personas/`, `profiles/`, `<data>/personas/` and
`<data>/routines.json` into agent folders and `routines.toml` once, keeping each as `<name>.bak-<date>`.

Paths are relative to the agent's folder (a plugin's: its checkout); `~/` is the home folder. A secret is never in an
agent: an env value that looks like a token is refused, and `secret:<NAME>` reads `<data>/secrets/<NAME>` at start.
Its MCP servers are in [mcp.md](mcp.md).
At spawn the agent runs on its account (`shed run --account <name>`; shed sets the account's folder) and its adapter
as `env K=V… <adapter>`; secrets go through a 0600 file that the adapter's shell reads and removes (`sh -c '. file;
rm file; exec adapter'`), so no secret is on a command line, in shed's state or in cooee's files or log. **Guarded
profiles:** `settings` (a JSON settings file, or a folder holding `settings.json`) reaches the agent only in the
session's `_meta`, at its provider's `settings` key, merged over the persona's own settings; an agent whose provider
has none (Codex's and Gemini's) can't start the profile: "this profile's guard can't run with <agent>". Claude Code's
is `claudeCode.options.settings`, which claude-acp passes to Claude Code as enforced settings, hooks included; it
never replaces the account's folder. The form preselects each folder's last-used profile.
