| `[history] hot_days`, `trim_after_days`, `trim_over_kb` | `30`, `7`, `4` | Thread retention (#364), hourly off the request path: a closed or ended thread quiet for `hot_days` moves to `<data>/archive/threads-YYYY-MM.jsonl.zst`; a tool item older than `trim_after_days` over `trim_over_kb` KB keeps its head and tail, its whole text archived. |
# Configuration

## Today

- **Setup:** one command installs CI's signed release, the services, the config, Tailscale serve and prints a pairing code (#45, #584).
- **Perf:** requests, shed calls and first token timed; `cooee perf` prints the table (#69).

## How it works

One TOML file per instance, `$XDG_CONFIG_HOME/cooee/<instance>/config.toml` (else `~/.config/cooee/<instance>/…`, or
`--config <file>`). Every key is optional; an unknown key is an error that lists the known ones.

**Instances.** `--instance <name>` (default `cooee`) picks one: its own config, port, data folder (threads, devices,
push key) and pairing, so two instances never share state; both use the one shed server. Plugins are per instance, so
one instance's tabs never touch another's. `cooee install --instance <name> --port <p>` writes
`~/Library/LaunchAgents/dev.cooee.<name>.plist` (KeepAlive, log in the data folder) and the config's `port` if it has
no config yet, then prints the `launchctl bootstrap`, `tailscale serve` and (if shed isn't running) the shed fix
for the owner to run. `cooee doctor` checks the service, port, shed, APNs, push key and tailnet URL.

| Key | Default | Meaning |
|---|---|---|
| `port` | `4747` | The port `cooee serve` listens on (`0` picks a free one). |
| `bind` | `127.0.0.1` | The address it listens on. Anything but loopback needs `allowed_hosts`. |
| `allowed_hosts` | none | The names requests may use besides localhost and `bind`: the Tailscale name. Matched exactly, so `*` widens nothing. |
| `roots` | none | The folders a thread may work in: each root and the folders right inside it, nothing outside. |
| `shed` | `shed` | The `shed` binary. |
| `shed_dir` | shed's per-user server | shed's `--dir` (its socket `shed.sock` and state). |
| `data` | `$XDG_DATA_HOME/cooee/<instance>`, else `~/.local/share/cooee/<instance>` | Where threads, devices and push subscriptions are stored. |
| `providers` | Claude Code, Codex, Antigravity | The sign-ins: `[[providers]]` with `name` (shed's provider of its accounts, which shed keeps: [accounts.md](accounts.md)), `agent` (its ACP agent id), `login` and `status` (commands, as arrays, that `cooee machine check` runs on another machine), `install` and `settings` (the dotted `_meta` key its ACP adapter takes an enforced settings object at, for agent profiles' guards; default none). An older row's `home`, `logout` and `usage` are read and dropped. `efforts` are the efforts a thread offers; `effort` the words that start its agent on one (`{}` is the pick; `NAME=value` goes in its environment, the rest after its command): Codex `-c model_reasoning_effort={}`. Models are the agent's own (#193): each session's start (and a daily probe) keeps the models its ACP session reports, per agent and account, in `<data>/models.json`; a short name (`opus`, `gpt-5`) runs on the newest model of its family, a full id as is, and a pinned id that is gone falls back to its family's short name with a line on the thread (and its persona's notes). A pick applies when the agent starts. The program facts (`login`, `status`, `efforts`, `effort`) come from shed's record of the agent (`shed agents --json`, shed#129) over cooee's built-in fallback; a shed with no record leaves the fallback. Setting it (`providers = []`: none) replaces the list: each row sets only its own fields over the provider of its `name`, so `name = "codex"` with `efforts` changes only the efforts; a new `name` needs `agent`, `login` and `status`. |
| `theme` | `farm` (when installed, else `terminal`) | The theme a device shows until it picks its own ([themes.md](themes.md)); installed theme packs and own schemes live in `themes/` and `schemes/` beside the config file, themes being built in `themes-dev/`. |
| `plugins` | none | `[[plugins]]` entries: `name`, `label`, `url`, `module`, `token` ([plugins/](plugins/README.md)). |
| `plugin_index` | the built-in `plugins/index.toml` | The plugin index `cooee plugin add <name>` and Settings › Plugins list ([plugins/](plugins/README.md)). |
| `[apns]` | none | Apple push for the native app: `key_id`, `team_id`, `topic` (the app's bundle id), `env` (`production` for TestFlight and App Store builds, `sandbox` for Xcode debug builds) and `url` (default Apple's server for each token's env). The key is `<data>/apns.p8`. |
| `[hooks]` | none | GitHub webhooks for routines (#261, [routines](routines.md)): `bind` (a second listener serving only `POST /hooks/github`), `url` (its public URL, as the GitHub webhook has it) and `api` (default `https://api.github.com`, for the catch-up). |
| `approval_expiry_minutes` | `30` | How long an agent's approval card raised by a routine's run waits before it counts as Deny: the agent gets the denial and the card reads `expired`. A card in a turn the owner sent never expires (#505). |
| `[perf] slow_ms` | `250` | A request taking this many ms or more is a `warn` line in `<data>/perf.log` and on stderr (#53). |
| `[logs] max_mb`, `keep` | `16`, `3` | `cooee.log`, `terminal.log` and `perf.log` in the data folder rotate past `max_mb` MB, at start and every 10 minutes, keeping `keep` old files (`<log>.1` the newest) (#363). |

`cooee serve` times every request (method, route pattern, status), every shed call (method, socket or CLI) and each
prompt's first reply chunk (thread, agent) into `<data>/perf.log`, one JSON line each, never a prompt or a token;
the app posts its own `client` timings there too (#606: `tap` to painted, stream `event` to painted, `send` to
echo). `cooee perf [--minutes 60]` prints p50/p95/max of each as a table; `--budget` fails when a client p95 is over
its budget (tap 100 ms, event and echo 150 ms).

**Version your config (#361).** The config folder may be kept in git. Cooee writes a `.gitignore` there (secrets,
keys, tokens, caches and build folders) when it creates the folder, or on `cooee serve` when the folder has none; it
never touches one that is already there. `cooee config init --git` runs `git init` and makes one first commit of the
folder as it stands, and refuses when the folder is a git repo already. After that cooee never runs git on the folder:
the owner commits, sets a remote and pushes. `config.toml` holds plugins' `token`s, so keep a remote private.

## API

### Folders

| Call | Body | Answer |
|---|---|---|
| `GET /folders?path=P&hidden=1` | | `{path, home, dirs: string[], roots: string[]}`: `P` (default home) inside home or a root; a root or a folder in one is spelled from the root as listed |
| `POST /roots` | `{path, remove?}` | the roots, `string[]` |
| `POST /folders/new` | `{parent, name}` | the new folder's path, `string` |
