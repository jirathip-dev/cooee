# Routine guards (#690)

Every agent turn a routine starts passes one guard, the heartbeat's own (#498): skip when nothing changed, and a
runs-a-day cap. A skipped turn costs no tokens. The ADR is [architecture/routine-guards](architecture/routine-guards/adr.md).

- **What "changed" means for a schedule's prompt.** A due time runs when the routine is fresh (it has not run since it
  was made, its prompt or schedule was edited, or it was resumed), when the owner wrote in a thread of its persona
  after its last run (#219's rule: the chat's agent profile for a `chat` routine, `who.profile` for a `new_thread`
  one, the Plain chats for none), or when it has not run yet this local day. Else it is skipped as idle. Routine
  prompts, replies, script output and silence notes never count, so routines can't wake each other.
- **Events and wakes are never idle.** A GitHub delivery, a signed POST or a wake script's non-zero exit is the new
  input; only the cap applies, after the pause and the hourly rate.
- **The cap** is agent turns started in a local day: the routine's own `per_day`, else `routines.toml`'s top-level
  `per_day`, else 24; 1 to 144 (`BAD_CAP` else). Past it a due time, an event or a wake is skipped as capped
  (`skipped: capped` on an event's or a wake's last run); what the owner wrote waits for the next run.
- **Scripts.** A script's own run is never checked or counted. A `post` script is never skipped and has no cap
  (setting one is refused); a `wake` script is checked and counted only when it would wake the agent.
- **Paused and ended routines, the silence clock and an alive POST** stop before the guard and count for nothing.
  A resume makes the routine fresh and keeps the day's count.
- **The model** is the chat's own: a `chat` run is a message in the chat, a `new_thread` run starts with the model
  the chat had.

The state is `<data>/watermarks.json` under `routine-<id>`: `line` (the last turn's time, 0: fresh) and `today`
(`day` since 1970 in local time, `ran`, `idle`, `capped`). A delete drops the key.

| Route | Body | Answer |
|---|---|---|
| `GET /screens/settings/routines` | | a core screen `routines`: one list, the `default` row (`<n> runs a day`), then each routine: its `when`, script kind, cap (its own or default; a `post` script: no cap) and today's ran, idle and capped, badged `capped today` |
| `POST /screens/action` | `{screen: "/screens/settings/routines", action: "default", values: {per_day}}` or `action: "cap"`, `values: {routine, per_day}` (`""`: back to the default) | the screen again; 400 `BAD_CAP` |
| `POST /routines/<id>` | `per_day` too (the same as `cap`) | 204; 400 with the refusal |

`create_routine` and Chief's `routine` tool can't set a cap: an agent never sets its own.
