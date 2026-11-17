Closes #498

## For humans
A persona can now check in by itself on a schedule: when something new happened in its chats or routines, it takes one cheap look and tells you only if your own rule (a plain-words note in its folder) says so.
When nothing changed it does nothing and spends nothing; it never acts on its own, anything it wants to do waits on a card that expires as Deny, and it runs at most 4 times a day unless you change that.
The persona panel shows its last run: when, what it read, and why it stayed quiet. It is off until you switch it on.

## What changed
- `[heartbeat]` in a persona's `agent.toml` (`on`, `when` default hourly, `model`/`effort` default the provider's cheapest, `per_day` default **4** in any 24 hours); the notify rule is `notify.md` in the persona folder, empty by default.
- The runner reuses #219's `watermarks.json` (`heartbeat-<persona>`): nothing new in the persona's threads or routines' results since the last line read means no run. Over `per_day`, the news waits for the next allowed run.
- A run is a `Heartbeat`-marked thread with shed policy `ask`, one prompt holding the rule and at most 20,000 characters of news. Its cards and turn ends don't push (background cards, #505 expiry). Its end pushes `<persona> · heartbeat` only when the rule is set and the reply ends `NOTIFY`, once per run (a restart doesn't re-push).
- The persona panel's routines block gets a Heartbeat row: last run time, what it read, why it stayed silent.
- Consolidation and the heartbeat now share one persona-run start (`Runner::as_persona`).

src/ net +255 (no budget stated): config type, runner tick, run-end handler and panel row; the cap holds at 26882 / 27500.

## Evidence (head 43f7523ee7d9009cc9003787caf56d01d1049428)
Fix round 4: merged origin/staging (95c3cb2db), no conflicts, no code change. The `test` FAIL at 3d68fad (`mcp::a_saved_secret_is_typed_again_without_asking_and_reaches_the_personas_environment_only`) was staging's own break from #547 (staging's ci failed the same test at 4e58adbf7 and e62471485); #557 on staging fixed it in `secrets::of_persona`, which this merge brings in.
- `ci/gates.sh`: exit 1, but only `deny` (`cargo deny` not installed) and `fmt` (`topcoat` not installed) failed: missing tools on this host. Every other gate passed, including `test` (with `--test heartbeat`; that mcp test is Linux-only and runs in CI), `clippy` and `cap` (26882 / 27500). `cargo fmt --all --check`: exit 0. CI runs `deny` and `fmt`.

## Why (round 1)

Decided: the heartbeat is a second built-in persona routine beside #219's consolidation, sharing its watermark file and one start path (Runner::as_persona).
Because: "Shares the watermark with #219, which builds it; this reuses it"
Decided: news is inlined in the run's one prompt, capped by history::NEW, and the watermark moves when the run starts.
Because: "reads at most 20,000 characters, ends after one pass"
Decided: the server, not the model, gates the push: no notify.md rule means no push whatever the reply says; with a rule, only a reply ending NOTIFY pushes.
Because: "a run pushes only when the persona's notify rule matches"
Decided: heartbeat runs start with shed policy ask and skip the needs/done/fail pushes, so their cards are background cards that #505 expires as Deny.
Because: "any action a heartbeat run wants raises a background approval card (#72), which expires as Deny"
Decided: per_day defaults to 4 runs in any rolling 24 hours, schedule defaults to hourly; an over-limit due time leaves the watermark so the news waits.
Rejected: polling connectors for a change signal — they have none without a model call, so the run reads them itself and only history and routine results are watermarked.
Check: a restarted server re-reporting an old run's end must not push again (Beat.run ties the end to its run; covered by the push test's restart).
Owner: A persona can now check in by itself and tell you only when your own rule in notify.md says so; it is off until you switch it on.

<sub>note on f6aa702 · git log --notes=why</sub>

## Why (round 2)

Decided: merged origin/staging and changed no code; the apple FAIL is the air-cooee runner drawing SwiftUI Sliders in theme-colours frames as placeholders.
Because: apple/Sources/CooeeUI/ThemeColours.swift:124
Rejected: re-recording the theme-colours designs with placeholders, or swapping the Slider — both change another issue's UI.
Check: the apple job on mini-cooee passes; on air-cooee staging itself fails the same three frames.
Owner: Nothing changed in the heartbeat; the iPhone check fails only on one build machine (air-cooee), which needs your look.

<sub>note on 3d68fad · git log --notes=why</sub>

