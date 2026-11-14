# Agent folders (#359)

## Summary

Decision: One folder per agent in the config folder replaces persona and profile with one thing; notes become
Markdown bullets and routine definitions one settings file. The first start converts everything in one step, keeps
the old folders as dated backups, and leaves the old layout untouched if it can't finish.
Slices: the agent folder (read, write, start a chat); notes and skills inside it; routines split into settings and
run state; the migration with its dry run and the upgrade test. One PR, in that order.
Open questions: three, each with a default: personal setup, wire names, comments lost on app edits (see the ADR).

## Files

| File | What it holds |
|---|---|
| [`adr.md`](adr.md) | The question, what the code forces, the options, the decision, the consequences, the open questions |
| [`callers.md`](callers.md) | How the code that uses agents reads after the change, written first |
| [`shapes.md`](shapes.md) | The folder, `agent.toml`, `memory.md`, `routines.toml`, the run-state file, and their Rust types |
| [`boundaries.md`](boundaries.md) | Which module may call which, and the statechart of an agent folder |
| [`migration.md`](migration.md) | The plan as data, the statechart of a migration, the merge table, the fixtures and tests |

## The layout, at a glance

```text
<config>/                         the config file's folder (~/.config/cooee/<instance>/): one git-able tree
├── config.toml                   unchanged
├── routines.toml                 each routine's name, schedule, prompt and place
└── agents/
    └── kai/                      the folder's name is the agent's id
        ├── agent.toml            every structured key
        ├── INSTRUCTIONS.md       plain Markdown, read as-is
        ├── CHARACTER.md          plain Markdown, read as-is
        ├── memory.md             - [3f] one note per line <!-- by:persona at:1731283200000 -->
        └── skills/release/SKILL.md

<data>/                           runtime only: threads, run state, devices, accounts, logs (and, as today, plugin
└── routine-runs.json             builds, secrets' files and keys)
```
