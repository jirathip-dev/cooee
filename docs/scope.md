# Scope and non-goals

What cooee owns and what it leaves to [shed](https://github.com/jirathip-dev/shed) and
[drover](https://github.com/jirathip-dev/drover) (#762). scripts/gen.sh copies the sections below into the README.

## Scope

cooee is the user-facing bots and chat: chat, personas, threads and the sign-in screens, and it draws other tools'
screens (drover's included) from their screens JSON. It calls shed for accounts, usage and runs, and keeps only the
policy and the UI over them. cooee sends every notification: drover emits events and cooee pushes them.

## Non-goals

- [shed](https://github.com/jirathip-dev/shed)'s jobs: accounts, credentials and secrets, the token proxy, usage,
  provider facts, the shed protocol (cooee uses shed-client), machines, VMs, installs and updates, process limits.
- [drover](https://github.com/jirathip-dev/drover)'s jobs: the workflow: issues, queues, lanes, reviews, merges. A new
  chat or thread feature for drover is built in cooee; drover adds only its backend (API, screens JSON), and its own
  phone page and terminal stay as they are.
