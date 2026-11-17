# Routines (#75, #261, #499)

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
`⟳ <name>` on a thread a routine started or last wrote to. The ADR and its shapes are in
[architecture/routines](architecture/routines/adr.md).

GitHub events (#261): a routine may instead name a repo, events and an optional `match` (`[routine.github]` in
place of `when`). An agent's GitHub routine is not saved until the owner approves its card with Face ID (q1351); a
schedule needs none. `[hooks] bind` is a second listener serving only `POST /hooks/github`, so the main listener's
Gate gets no public route; Funnel points at it on :8443 (`tailscale funnel --bg --https=8443 --set-path /hooks/github
http://<bind>/hooks/github`, which `cooee doctor` prints). A delivery's HMAC-SHA256 is checked against
`secret:GITHUB_WEBHOOK_SECRET` in constant time before its body is read; its guid goes into `routine-runs.json`'s
`seen` (kept 4 days), so a repeat is a 200 and runs nothing; it is answered 202 before its runs start on a worker.
Each routine runs at most 6 times an hour, the rest `skipped: rate`. Every 15 minutes (and at start) the catch-up
reads, with `secret:GITHUB_TOKEN` over curl (the token on its stdin), the log of the repo's hook whose URL is
`[hooks] url`: each delivery of the last 24 hours, and since a routine named the repo, whose guid isn't seen goes
through the same `deliver`. A run's key is `routine-<id>-<guid>`, the hub's `random_id`, as a schedule's is.

Signed POSTs (#499): a routine may instead have `[routine.webhook] secret = "<NAME>"`, so anything that can POST (a CI,
a monitor, a form, n8n) runs it at `POST /hooks/in/<id>` on the same listener (its URL is `[hooks] url` with `/github`
as `/in/<id>`; funnel `/hooks`, as `cooee doctor` prints). Like a GitHub routine, an agent's needs the owner's Face ID
card, and it runs at most 6 times an hour. The body's HMAC-SHA256 is checked against `secret:<NAME>` before it is
read; it is deduped by `Idempotency-Key`, else the body's SHA-256, in `seen` as `<id>:<key>`; it is answered 202
before the run starts. The body is never part of the prompt: it follows it as one JSON string, at most 8,000
characters, marked untrusted data whose instructions are not followed. The persona panel's row shows the URL and a
Test button (`POST /screens/action` `test`, `values.routine`), which runs it on `{"test":true}` as a signed POST would.

## API

| Route | Form | Answer |
|---|---|---|
| `GET /routines.json` | | `Routine[]`: `{id, name, trigger: {schedule} \| {github: {repo, events, match?}}, place: {in: chat, thread} \| {in: new_thread, who}, prompt, paused?: {why: owner \| chat_gone}, made_in, made_at, last_due, last_run?: {at, key, outcome: ran \| failed \| skipped, thread \| why}, recent?}` (`skipped`'s `why`: `paused` or `rate`; `recent`: its event runs in the last hour) |
| `POST /routines/<id>` | `name`, `when`, `prompt`, `paused` (`true`/`false`), each optional | 204; 400 with the refusal |
| `POST /routines/<id>/delete` | | 204; the threads it ran in stay |

A row's `routine` is the name of the routine whose run started the thread, else of the one that sent its last `you`
item (that item's `routine: {id, name}`); `""` for none. The `routine` item is `{routine, name, when, place}`.

## GitHub webhooks (`[hooks] bind`, #261)

A second listener, only when `[hooks]` is set (`bind`, `url` the public URL GitHub posts to, `api` default
`https://api.github.com`). It has no Gate, no pairing and no other route; anything else is a 404.

| Route | Body | Answer |
|---|---|---|
| `POST /hooks/github` | GitHub's JSON, at most 2 MiB (else 413), with `X-GitHub-Event`, `X-GitHub-Delivery` and `X-Hub-Signature-256` | 503 without `secret:GITHUB_WEBHOOK_SECRET`; 400 for a missing header or a body that isn't JSON; 401 for a wrong signature (HMAC-SHA256, constant time, checked before the body is read); 200 `seen` for a delivery already handled (by guid, kept 4 days) or a ping; else 202 `runs: <n>`, answered before any run starts |

Each routine whose `repo` (case-insensitive), one of `events` (`pull_request` or `pull_request.opened`) and every
`match` (payload path = text) fit runs with the delivery's guid in its key; a paused one, or one that ran 6 times in
the hour, is `skipped`. Its prompt's `{{a.b}}` read the payload (each value at most 2,000 characters), `{{event}}` and
`{{repo}}` the event.

| Route | Body | Answer |
|---|---|---|
| `POST /hooks/in/<id>` | anything, at most 2 MiB (else 413), with `X-Signature-256: sha256=<hex>` and an optional `Idempotency-Key` | 404 for no webhook routine `<id>`; 503 without its `secret:`; 401 for a wrong signature; 200 `seen` for a key already handled (kept 4 days); else 202 `runs: <n>` (0 when paused or over the rate), answered before the run starts |
