# The conformance suite (#503)

One scenario, written as data, run against every provider through the real entry point: `cooee serve`'s HTTP API, as
the app uses it. It never looks at a harness's tool names or wording, only at what a user sees: replies, rows, cards,
files, states.

## The scenario

A scratch persona with a folder of its own, a brief holding `The code word is PLOVER-7.`, one skill `conformance`
(body: `Reply with the single word HERON.`), and no rules but the starter ones.

| # | Step | The owner says | Passes when |
|---|---|---|---|
| 1 | The brief is honoured | "What is the code word?" | the reply holds `PLOVER-7` |
| 2 | A cooee tool is called | "Keep a note that the check ran, with your remember tool." | the persona's `memory.md` has the note |
| 3 | A skill is read | "Use your conformance skill." | cooee served the skill's body and the reply holds `HERON` |
| 4 | An approval is raised | "Run `printf conformance > probe.txt`." | a card appears with action `run` and that command; the suite taps **Deny**; `probe.txt` does not exist |
| 5 | A rule answers one | the suite adds `allow run printf conformance*`, then the same message | no card; the row says the rule answered; `probe.txt` exists |
| 6 | Stop works | "Run `sleep 120`." (a rule allows it) then stop while the row is running | the turn ends as cancelled within 10 s; the row is cancelled; no `sleep` is left |
| 7 | A limit is reported | the fake: its scripted limit line. A real harness: nothing is said | the fake: the turn state is the neutral limit state with its reset time. A real harness: the session reported its context use |
| 8 | Built-ins are off (nightly only) | "List the names of every tool you have, one per line." | none of the names the provider's module turned off is in the reply |

Step 7 cannot hit a real limit on purpose, so on a real harness it checks what can be checked live (context use
arrives). The limit lines themselves are shed's data, tested there against lines captured from each harness.

## In CI: the fake

`crates/cooee/tests/conformance.rs`, in the `test` gate on every PR. It runs steps 1–7 on `FakeShed` and its scripted
agent (`crates/cooee/tests/support/`), which gains what the steps need: the ACP call on its `approval_request`, an
answer by request id, a tool call it can leave running, and a limit line. The provider is `providers/fake.rs`, whose
`session()` and `action()` are filled like a real one's, so the whole path runs: `hub/session.rs`, the provider,
`hub/approvals.rs`, the rules, the rows. No shed and no harness is installed in CI.

## Nightly: the real harnesses

`.github/workflows/conformance.yml`, its own file: `schedule` and `workflow_dispatch` (with a `ref`, so a slice's
branch can be proven before it merges), never `pull_request`, so no fork's code runs on it. It runs on the Mac runner
(`self-hosted, cooee-mac`) as one job per provider:

1. builds `cooee` and the pinned `shed` into `RUNNER_TEMP`, and starts `shed serve --dir` and `cooee serve` there on
   their own port: no login item, no launchd job, nothing outside `RUNNER_TEMP`;
2. runs `cargo test --test conformance -- --ignored` with the provider's name: the same scenario, steps 1–8, against
   the real adapter on the owner's own login for that provider;
3. writes the report to the job summary, one row per step: `pass`, `fail`, `gap` (a gap the provider's module
   declares, with its words) or `closed` (a declared gap that passed: the table is out of date);
4. on a `fail` or a `closed`, opens or updates one issue `conformance: <provider> <step>`, as `scripts/ci-nightly.sh`
   does for the nightly CI.

A provider that is not installed or not signed in on the runner reports `skipped: <why>` and is not red.

### The token ceiling

Per provider, per night:

- at most **10 prompts** (the scenario has 7) and **10 minutes**;
- the run stops, and reports `over ceiling`, when the session's reported context passes **40,000 tokens**;
- on the smallest model the provider offers, named in its module.

So a night costs a provider at most about 400,000 input tokens, most of them cache reads of one prefix, and a few
thousand output tokens. The scenario's messages are short on purpose and no step reads a large file.

## What the suite does not claim

It proves the contract's switches and cooee's own path. It does not prove that two models behave alike: the vendor's
system prompt, compaction and the model's habits are outside it ([`adr.md`](adr.md), "What cannot be made the same").
