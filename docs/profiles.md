# Agent profiles (#40)

## Today

- **Secrets:** kept in the macOS keychain, readable only by the signed cooee (#212).

## How it works

An agent profile is a named recipe for a thread's agent, picked in the new-thread form and fixed for the thread's
life (a new thread changes it). **Plain**, the default, is the agent with the owner's own setup and nothing added. The
owner's profiles are `<config>/profiles/*.toml`, one per file; a plugin brings its own as `[[profiles]]` in its
`cooee-plugin.toml` ([plugins.md](plugins.md)), shown with the plugin's name, listed while it is installed and on.

```toml
name = "coder"                  # its id: lowercase letters, digits and - (a plugin's: <plugin>/<name>)
label = "Coder"                 # the picker and the thread's header
agent = "claude-acp"            # the ACP agent
account = "work"                # optional: its provider's account (else the default)
folder = "~/Projects/app"       # optional: the folder (a root or inside one), instead of the one picked
instructions = "coder.md"       # optional: sent ahead of the first message (the chat shows only the message)
settings = "settings.json"      # optional: an agent settings file, its hooks the profile's guard
env = { MODE = "review", API_KEY = "secret:API_KEY" }   # optional: plain values ({thread}: the thread's id), secrets as secret:<NAME>
policy = "ask"                  # optional: shed's approval policy, ask, allow or deny
avatar = "https://x/me.png"     # optional: the sidebar's avatar, an image URL or a theme sprite id (none: initials)

[[mcp]]                         # optional, repeatable: an MCP server from the registry (#56)
server = "github"
tools = ["get_issue"]           # optional: the tools it may use (none: all)
```

Paths are relative to the profile's file (a plugin's: its checkout); `~/` is the home folder. A secret is never in a
profile: an env value that looks like a token is refused, and `secret:<NAME>` reads `<data>/secrets/<NAME>` at start.
Its MCP servers are in [mcp.md](mcp.md).
At spawn the agent's adapter runs as `env <account folder> <settings variable>=<file> K=V… <adapter>`; secrets go
through a 0600 file that the adapter's shell reads and removes (`sh -c '. file; rm file; exec adapter'`), so no
secret is on a command line, in shed's state or in cooee's files or log. **Guarded profiles:** `settings` reaches the
agent only through its provider's `settings` variable, the one its ACP adapter reads an extra settings file from
and enforces, hooks included; an agent with none (Codex's and Gemini's) can't start the profile: "this profile's guard
can't run with <agent>". Claude Code's is `CLAUDE_CONFIG_DIR` (herdsman#565: claude-acp enforces the user settings
there), so a Claude profile's `settings` is a config folder (its `settings.json` holds the hooks), in place of the
account's. The form preselects each folder's last-used profile.
