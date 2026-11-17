# Routines (#75)

## How it works

A routine is a schedule (five-field cron in the Mac's local time, its UTC offset read with `date +%z` hourly, at
least 15 minutes between due times), a place and a prompt: `<config>/routines.toml` (`[[routine]]`: `id`, `name`,
`when`, `prompt`, `paused`, `[routine.place]`), its run state in `<data>/routine-runs.json`, each written whole (0600)
when its part changed. One typed by hand first runs at its next due time. An agent makes one in chat with cooee's own `create_routine` tool, as itself: the chat gets a `⟳
Created routine` line; a Workflow session or a routine's own run can't. `POST /routines/<id>` renames,
retimes, pauses and resumes them, `POST /routines/<id>/delete` deletes them; there is no other way to make one. Every 30 seconds the runner runs each
routine whose newest due time is past both its last recorded one and its making, once (missed ones run once, late),
then records it. The run's key, `routine-<id>-<due>`, is the hub's `random_id`, so a crash between a run and its
record runs into the same message or thread again, never a second. `chat` runs queue the prompt in the making chat
as a message marked with the routine; a closed or deleted chat pauses the routine with one push. `new_thread` runs
start a thread as the making chat's agent, marked with the routine, whose finished turns don't push. The list shows
`⟳ <name>` on a thread a routine started or last wrote to. The GitHub event path is #261. The ADR and its shapes are
in [architecture/routines](architecture/routines/adr.md).

## API

| Route | Form | Answer |
|---|---|---|
| `GET /routines.json` | | `Routine[]`: `{id, name, trigger: {schedule}, place: {in: chat, thread} \| {in: new_thread, who}, prompt, paused?: {why: owner \| chat_gone}, made_in, made_at, last_due, last_run?: {at, key, outcome: ran \| failed, thread \| why}}` |
| `POST /routines/<id>` | `name`, `when`, `prompt`, `paused` (`true`/`false`), each optional | 204; 400 with the refusal |
| `POST /routines/<id>/delete` | | 204; the threads it ran in stay |

A row's `routine` is the name of the routine whose run started the thread, else of the one that sent its last `you`
item (that item's `routine: {id, name}`); `""` for none. The `routine` item is `{routine, name, when, place}`.
