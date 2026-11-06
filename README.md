# cooee

Chat with any coding agent from your phone. cooee is a small self-hosted app: one owner, one machine (Mac or
Linux), reached from your phone over Tailscale. Agents (Claude Code, Codex, Gemini, Pi, any ACP agent) run on the
machine through [shed](https://github.com/jirathip-dev/shed), keep working while you are away, and cooee pings your
phone when they need you.

> **Status: M2, and M4's plugin seam.** Chat: `cooee serve` lists threads and runs each as an ACP session on shed,
> with streamed replies, tool rows and approvals. Phones pair with `cooee pair` and get web push. Plugins add tabs
> ([docs/plugins.md](docs/plugins.md)). `just check` runs every CI gate; see
> [docs/architecture.md](docs/architecture.md).

A cooee is the bush call you use to reach someone far off.

## Quick start

On a Mac with [Rust](https://rustup.rs) and [Tailscale](https://tailscale.com/download) (signed in):

```sh
curl -fsSL https://raw.githubusercontent.com/jirathip-dev/cooee/main/scripts/install.sh | sh
```

or `./scripts/install.sh` from a clone. It installs cooee, then `cooee setup` installs shed at the pinned version and
its service, writes the config (asks for the folders threads may use, default `~/Projects`), adds Claude's ACP agent
(asks about Codex and Gemini), starts cooee's service, runs `tailscale serve`, checks it all with `cooee doctor` and
prints a pairing code to open on your phone. Run it again any time: finished steps are skipped. `--dry-run` prints
every command and writes nothing; `--yes` takes the defaults. A second instance:
`cooee setup --instance cooee-drover --port 4748`.

## The three projects

| Project | Role |
|---|---|
| **cooee** (this repo) | The app: chat, threads, phone pairing, push, provider logins, plugin tabs. |
| **shed** (was berth) | The runtime: runs agents in terminals or as ACP sessions, headless, across machines. |
| **drover** (was herdsman) | The coding workflow: issue queue, lanes, review gates, merge. A CLI plus an API; its tabs become a cooee plugin. |

cooee works without drover. drover moves its board into cooee as a plugin once cooee is in daily use.

## Owner decisions (2026-11-05)

- Chat talks to agent CLIs over ACP on the owner's own logins (they can use tools), not raw API keys.
- A new repo and its own product; drover moves its chat and tabs onto it later, in stages.
- v1 is chat only: phone/web chat with any ACP agent, pairing, push, logins. Telegram, scheduled jobs and memory stay
  with Hermes for now.
- The client is a web app (installable PWA) for phone and desktop, served by a Rust server.
- Self-hosted, one owner per install; public open-source repo.
- Provider logins happen from the phone: cooee runs the CLI's own login command in a shed terminal and shows it in
  the app; tokens stay with the CLI and are never shown in cooee.

## Milestones

- **D:** design of the v1 screens (thread list, chat, login sheet).
- **M0:** workspace skeleton with every CI gate green.
- **M1:** chat: one thread = one ACP session on shed.
- **M2:** phone pairing and web push.
- **M3:** provider logins from the phone.
- **M4:** the plugin seam: a tab = a UI module + a backend URL.

Later: agent profiles (a name, a provider, a folder, instructions, skills: research, health coach…), then drover's
tabs as a plugin.

## Licence

MIT or Apache-2.0, at your option.
