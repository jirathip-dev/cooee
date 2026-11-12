# ADR: routines: a schedule or a GitHub event posts a prompt to an agent (#75)

Files: this ADR, [`callers.md`](callers.md) (how the callers read, written first), [`shapes.md`](shapes.md) (the
types and the file), [`boundaries.md`](boundaries.md) (who calls whom, the public edge, the statecharts).

## Question

How does a routine (trigger + who + folder + prompt template) live in cooee, so that a cron-style schedule or a
signed GitHub webhook starts a run exactly once, the run is a normal thread (or chat message) marked as a routine's,
and routines are made in chat (q764), listed with their last run, paused and deleted in Settings, with webhooks
published on one Funnel path and a slow catch-up poll (q718)?

## Constraints found in the code

1. A thread is started by `Hub::create_later` and spoken to by `Hub::send`/`queue`; both are idempotent on a
   `random_id` that is stored with the thread or item, across restarts. That is the run ledger we need.
2. `recipe()` (cooee-server `lib.rs`) already turns (agent, account, persona-or-profile, folder, model, effort) into a
   start. A routine's "agent profile + folder" is exactly its input.
3. The Gate lets only paired devices in (plus `/mcp/agent/*`, token-checked). A public endpoint on that listener
   would be the first unauthenticated non-pairing route.
4. Funnel turns on per port: every handler on `:443` (where `tailscale serve` carries the phone) would go public.
5. No time crate in the tree; `unsafe` is forbidden, so no `localtime_r`. `hkdf` re-exports `hmac`; `sha2` is here.
6. cooee's own MCP server (`cooee mcp serve`) already gives agents tools that write through `/mcp/agent/*`.
7. The approved design (`docs/design/chat-first` §4) draws the `.rt` "Created routine" line and says a routine
   "runs in the chat that made it and posts there".
8. Headroom: 24,874 / 27,500 non-test lines.

## Options

| | Choice | For | Against |
|---|---|---|---|
| Storage A | `<config>/routines/*.toml`, like profiles | Hand-editable | Agents write it from chat; run state (`last_due`, `seen`) would sit apart in data anyway |
| Storage **B** | **`<data>/routines.json`, one typed file with run state** | One lock, atomic, `format` versioned like threads | Not hand-edited (Settings edits it) |
| Edge A | `/hooks/github` on the main listener, a Gate bypass | No new listener | Public reach to the app's listener; one Gate typo exposes pairing |
| Edge **B** | **A second listener (`[hooks] bind`) with that one route; Funnel on :8443** | The Gate is untouched; nothing else is reachable even if Funnel is misset | ~30 lines of wiring; a second port |
| Time A | `jiff` (IANA zones) | Exact DST | A new dependency: an owner call |
| Time **B** | **Own 5-field cron, in the Mac's offset from `date +%z`, read hourly** | No dependency; Thailand has no DST; DST elsewhere follows within an hour | The transition hour itself can skip or double a due time (the run key still runs it once) |
| Catch-up A | Ask GitHub to redeliver failed deliveries | One signed path | Fails while Funnel is down, which is when it is needed |
| Catch-up **B** | **Read GitHub's delivery log with the owner's token and ingest unseen ones** | Works with Funnel down; same ingest | A second proof (`fetched`), since the log returns parsed JSON, not the signed bytes |
| Place | Chat only / new thread only / **both, typed** | — | The design wants the chat; a CI event wants its own thread so runs don't interleave |

## Decision

**Storage B, Edge B, Time B, Catch-up B, Place both.**

1. **A routine** is `{id, name, trigger, place, prompt, paused?, made_in, made_at, last_due, last_run?}` in
   `<data>/routines.json` ([shapes](shapes.md)). Types parse at the edge (the tool, the form, the file); inside,
   `Schedule`, `Template` and `Delivery` can't be built invalid.
2. **Made in chat only** (q764): cooee's MCP server gains `create_routine {name, when | github, prompt, place?}`.
   `who` is the calling chat's own agent, account, persona/profile, folder, model and effort: an agent can't make a
   routine that runs as someone else. Refused from a Workflow thread (herdsman's lanes) and from a routine's own run.
   The chat gets the `.rt` line. Settings › routines lists, renames, retimes, edits the prompt, pauses, resumes and
   deletes; it has no "new" form.
3. **Place**: `chat` (default, as drawn): each run is a message in that chat, a `you` item marked ⟳ with the
   routine; a busy turn queues it like the owner's. `new_thread`: each run starts a thread as `who`, its
   `Thread.routine` set. If a `chat` routine's chat is closed or gone, the run fails and the routine pauses
   (`ChatGone`) with one push.
4. **Schedules**: 5-field cron (no names) in the Mac's local time; at least 15 minutes between due times. A tick
   every 30 s runs the newest due time past `max(last_due, made_at)` once, then records it. The run's key
   `routine-<id>-<due>` is the hub's `random_id`, so a crash between run and record re-runs into the same thread,
   never a second one. Missed due times while asleep coalesce into one late run.
5. **GitHub events**: `[hooks] bind` (off when unset) serves only `POST /hooks/github`: body ≤ 2 MiB, HMAC-SHA256
   checked against `secret:GITHUB_WEBHOOK_SECRET` in constant time, deduped by `X-GitHub-Delivery`, answered
   before the run starts. A trigger names a repo, events (`pull_request` or `pull_request.opened`) and optional
   `match` (payload path = value). Each routine runs at most 6 times an hour; the rest are `Skipped(Rate)`.
6. **Catch-up** every 15 minutes, only for repos some routine names: with `secret:GITHUB_TOKEN`, find the hook
   whose URL is `[hooks] url`, list 24 h of deliveries, ingest each guid not seen. Same `deliver()`, same dedupe.
7. **The prompt** is a `{{path}}` template over the payload, plus `due`, `event`, `repo`. Values are cut at 2,000
   chars; nothing in a payload is ever run, only shown to the agent.
8. **Approvals and pushes**: a run's thread follows its profile's policy (q764: "the allow rules decide"; no
   drafts). Like a Workflow session, its turns end without a push; approvals and errors still push. These are the
   "background cards" of the issue's scope 3.
9. **The list** (AC 3): `Row.routine` is the routine's name for a thread a routine started, or whose last `you`
   item a routine sent; the web list shows `⟳ <name>`.

## Consequences

- New: `cooee-core/src/routines.rs` + `routines/cron.rs`; `cooee-server/src/routines.rs` (Runner, the hooks
  listener, `/routines` routes, `/mcp/agent/routine`); the tool in `mcp.rs`; the page and row mark in `cooee-web`;
  `[hooks]` in config; `Thread.routine`, `Body::You.routine`, `Body::Routine`, `Recipe.routine`, `Row.routine`.
  `docs/architecture.md` gains a Routines section; `docs/api.md` the four routes and the row field.
- Size: about `src/ net +900` (core ~350, server ~380, web ~120, wiring ~50); fits the 2,626 headroom.
- Tests (AC 2), all through the real entry points: (a) a schedule due in the past runs once; a second `Runner`
  opened on the same data dir (a restart) at the same `now` runs nothing; a crash between run and record (the
  record dropped by the test) adopts the same thread; (b) `POST /hooks/github` with a wrong signature → 401 and no
  thread; (c) a good one → 202 and a new thread whose first message has the payload's `pull_request.title`; the
  same delivery again → 200 and still one thread; (d) the catch-up against a fake GitHub (a local listener, curl
  pointed at it) ingests a missed delivery once. Plus unit tests for cron (`latest`, the 15-minute floor, DOM/DOW
  "or"), the template, and HMAC against GitHub's documented test vector.
- Security: the public surface is one route on its own port, every byte signature-checked before parsing. A public
  repo lets strangers fire events: the rate cap bounds runs, and the profile's policy bounds what a run may do. An
  agent can make a routine (persistence past its session) but only as itself, shown in its chat and in Settings.
- The owner's one-time setup: put the webhook secret in cooee (Settings › mcp servers › secret), add the webhook in
  GitHub (`[hooks] url`, content type JSON, that secret), run the Funnel line `cooee doctor` prints, and for the
  catch-up a fine-grained token with "Webhooks: read" as `GITHUB_TOKEN`.
- Out of scope: Sentry and non-GitHub senders, a native app routines screen (the API is there for it), run history
  beyond the last run.

## Open questions for the owner

1. **Secrets' home.** q718 says the webhook secret comes "via herdsman secret request"; that stores it in
   herdsman's store (`herdsman/<fleet>/…`), not cooee's. **Default:** cooee reads its own `secret:GITHUB_WEBHOOK_SECRET`
   (keychain); the impl lane uses `herdsman secret request` only to get one for its live end-to-end check. OK?
2. **Time zone.** **Default:** no new dependency; cron in the Mac's current offset (`date +%z`, hourly). Say if
   you want `jiff` for exact DST instead (a new dependency).
3. **Agent-made routines need no approval** (as drawn in q764). **Default:** none; the `.rt` line and a push tell
   you. Alternative: an approval card (Face ID) before an *event* routine is saved, since it runs on outsiders'
   input.
4. **Limits.** Defaults: 15-minute schedule floor, 6 event runs per routine per hour, 2 MiB bodies, 24 h catch-up
   window, 50 routines. Change any?
5. **One PR or two?** **Default:** one, as the issue is one. Alternative: schedules + chat + Settings first, then
   the GitHub path.
