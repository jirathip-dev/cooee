# cooee

cooee is a phone and Mac app for chatting with any coding agent you run yourself. One owner, one machine (Mac or
Linux): agents (Claude Code, Codex, Gemini, Pi, any ACP agent) run there as ACP sessions on
[shed](https://github.com/jirathip-dev/shed), keep working while you are away, and cooee pings your phone when they
need you. Self-hosted, reached over Tailscale. A cooee is the bush call you use to reach someone far off.

## Today

On `staging`, each line with the PR or file that proves it:

- **Chat:** one thread is one ACP session on shed, with streamed replies, tool rows and approvals (#14).
- **Chat-first:** a new chat needs no folder; the sidebar lists agents with their chats (#88, #63).
- **Messages are kept:** a send the agent never got stays on its bubble, and a stopped agent refuses sends (#256).
- **Personas:** named agents with a role, notes, attached plugins and a machine, edited from the app (#131, #222, #135, #174, #195).
- **Persona memory and history:** a bounded `remember` tool and a `history` tool over the persona's own past threads (#227, #228).
- **Rooms:** several personas in one thread, taking turns, asking each other with `ask_persona` (#175, #187, #186, #207).
- **Native app:** one SwiftUI app for iPhone, iPad and Mac in [`apple/`](apple/README.md): pairing, Home, streaming chat, cards as blocks (#146, #196).
- **Mac app as the server:** the Mac app runs cooee and shed as login items and pairs phones by QR (#197).
- **TestFlight and push:** CI uploads the iOS app to TestFlight; pushes go by APNs and web push (#145, #134, #16).
- **Web app:** a server-rendered installable web app, HTML-first, strict TypeScript only where needed (#58).
- **Pairing:** QR or a short code, approved from a paired device with its passkey (#68).
- **Sign-ins and accounts:** the provider's own login runs from the phone; several accounts per provider, each with usage (#18, #46, #191).
- **Models:** each agent's live model list; short names track the newest model (#194).
- **Plugins:** git repos added from a curated index, supervised backends, tabs and screens as data (#51, #205, [`plugins/index.toml`](plugins/index.toml)).
- **Drover:** the `cooee-drover` plugin adds herdsman's Queue, Asks and Fleets; the app draws fleet settings (#71, #211).
- **First-party plugins:** `machines` (per-machine health) and `terminal` (your shells, behind Face ID) (#50, #91).
- **Workflow:** sessions other tools start in shed list beside your chats and open as chats (#61).
- **Run cards:** herdsman's commands shown with their sha, approved or denied from the phone (#114).
- **MCP:** a registry of MCP servers per persona through cooee's proxy; agents propose new ones for Face ID approval (#108).
- **Secrets:** kept in the macOS keychain, readable only by the signed cooee (#212).
- **Looks:** Modern, Terminal and Scene styles; farm, kitchen and garden theme packs (#65, #66, #67).
- **Setup:** one command installs shed, the services, the config, Tailscale serve and prints a pairing code (#45).
- **Perf:** requests, shed calls and first token timed; `cooee perf` prints the table (#69).

## In progress / next

- **Native motion and look:** [#246](https://github.com/jirathip-dev/cooee/issues/246), [#252](https://github.com/jirathip-dev/cooee/issues/252), [#225](https://github.com/jirathip-dev/cooee/issues/225), [#141](https://github.com/jirathip-dev/cooee/issues/141)
- **Native feel:** [#232](https://github.com/jirathip-dev/cooee/issues/232), [#251](https://github.com/jirathip-dev/cooee/issues/251), [#254](https://github.com/jirathip-dev/cooee/issues/254), [#234](https://github.com/jirathip-dev/cooee/issues/234)
- **Native chat and Home:** [#229](https://github.com/jirathip-dev/cooee/issues/229), [#242](https://github.com/jirathip-dev/cooee/issues/242), [#243](https://github.com/jirathip-dev/cooee/issues/243), [#245](https://github.com/jirathip-dev/cooee/issues/245), [#249](https://github.com/jirathip-dev/cooee/issues/249), [#231](https://github.com/jirathip-dev/cooee/issues/231)
- **Native screens:** [#236](https://github.com/jirathip-dev/cooee/issues/236), [#237](https://github.com/jirathip-dev/cooee/issues/237), [#239](https://github.com/jirathip-dev/cooee/issues/239), [#240](https://github.com/jirathip-dev/cooee/issues/240), [#244](https://github.com/jirathip-dev/cooee/issues/244), [#230](https://github.com/jirathip-dev/cooee/issues/230)
- **Drover in the app:** [#226](https://github.com/jirathip-dev/cooee/issues/226), [#181](https://github.com/jirathip-dev/cooee/issues/181), [#213](https://github.com/jirathip-dev/cooee/issues/213)
- **Routines:** [#75](https://github.com/jirathip-dev/cooee/issues/75), [#261](https://github.com/jirathip-dev/cooee/issues/261)
- **Personas:** skills [#218](https://github.com/jirathip-dev/cooee/issues/218), consolidation [#219](https://github.com/jirathip-dev/cooee/issues/219), inbox [#220](https://github.com/jirathip-dev/cooee/issues/220)
- **Rooms, native:** [#157](https://github.com/jirathip-dev/cooee/issues/157), [#176](https://github.com/jirathip-dev/cooee/issues/176), [#177](https://github.com/jirathip-dev/cooee/issues/177), [#178](https://github.com/jirathip-dev/cooee/issues/178), [#179](https://github.com/jirathip-dev/cooee/issues/179)
- **Machines:** [#149](https://github.com/jirathip-dev/cooee/issues/149), [#150](https://github.com/jirathip-dev/cooee/issues/150), [#151](https://github.com/jirathip-dev/cooee/issues/151), [#152](https://github.com/jirathip-dev/cooee/issues/152)
- **A browser per persona:** [#160](https://github.com/jirathip-dev/cooee/issues/160) to [#166](https://github.com/jirathip-dev/cooee/issues/166)
- **Desks (sandboxed machines):** [#168](https://github.com/jirathip-dev/cooee/issues/168) to [#173](https://github.com/jirathip-dev/cooee/issues/173)
- **Chat:** trace sheet [#79](https://github.com/jirathip-dev/cooee/issues/79), approval rules [#72](https://github.com/jirathip-dev/cooee/issues/72), agent fallback [#76](https://github.com/jirathip-dev/cooee/issues/76), voice [#81](https://github.com/jirathip-dev/cooee/issues/81)
- **Server:** SQLite store [#221](https://github.com/jirathip-dev/cooee/issues/221), sync audit [#73](https://github.com/jirathip-dev/cooee/issues/73), relay [#74](https://github.com/jirathip-dev/cooee/issues/74), version gating [#77](https://github.com/jirathip-dev/cooee/issues/77), scripting CLI [#82](https://github.com/jirathip-dev/cooee/issues/82)
- **Code rules:** [#201](https://github.com/jirathip-dev/cooee/issues/201), [#202](https://github.com/jirathip-dev/cooee/issues/202), [#203](https://github.com/jirathip-dev/cooee/issues/203), CI [#263](https://github.com/jirathip-dev/cooee/issues/263)
- **Web app retired** once the native app ships: [#189](https://github.com/jirathip-dev/cooee/issues/189)

## Setup

On a Mac with [Rust](https://rustup.rs) and [Tailscale](https://tailscale.com/download) (signed in):

```sh
curl -fsSL https://raw.githubusercontent.com/jirathip-dev/cooee/main/scripts/install.sh | sh
```

It installs cooee, then `cooee setup` installs shed, writes the config, adds the agents, starts the service, runs
`tailscale serve`, checks it all with `cooee doctor` and prints a pairing code. Open that code on your phone. Run it
again any time; `--dry-run` writes nothing.

On a Mac you can instead run the Mac app ([`apple/README.md`](apple/README.md)): its first run starts the server and
shows a QR code for the phone.

## Docs

- [docs/architecture.md](docs/architecture.md): how it fits together.
- [docs/api.md](docs/api.md): the JSON and SSE API.
- [docs/plugins.md](docs/plugins.md), [docs/themes.md](docs/themes.md), [docs/accounts.md](docs/accounts.md), [docs/logins.md](docs/logins.md).
- Designs: [docs/design/](docs/design/). Architecture records: [docs/architecture/](docs/architecture/), [docs/decisions/](docs/decisions/).

`just check` runs every CI gate.

## Licence

MIT or Apache-2.0, at your option.
