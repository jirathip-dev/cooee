Closes #130

## For humans

The native app can now get Apple push notifications. Each alert cooee already sends (an agent asks, a finished reply, plugin pushes, the test push) also goes to every iPhone that registered for it.
The owner puts their Apple push key in the data folder as `apns.p8` and fills an `[apns]` section in the config. `cooee doctor` says whether Apple push is ready and never prints the key.

## What changed

- `[apns]` config: `key_id`, `team_id`, `topic`, `env` (`sandbox`/`production`), and an optional `url` (default Apple's server). The key is only `<data>/apns.p8`, tightened to 0600 before use like `vapid.key`.
- `POST /push/apns {token, env?}` / `DELETE /push/apns {token}` from a paired device. Tokens are kept in `push.json` beside the web subscriptions (an `apns` marker on the entry). An unpaired device's token is dropped, the same way as its web subscriptions. A VAPID key renewal leaves APNs tokens alone.
- Sending: the same `Note` goes over `curl --http2` to `/3/device/<token>`, with `authorization: bearer <ES256 JWT {alg,kid}/{iss,iat}>` (reused for 50 min), `apns-topic`, `apns-push-type: alert`, `apns-priority: 10`, `apns-collapse-id: <tag>`, and a body of `aps.alert{title,body}`, `thread-id`, `url` and `item`. A 410 or `BadDeviceToken` drops the token.
- `cooee doctor`: `ok   apns ready`, or `warn apns: <why>` (the file path, never its contents).
- Docs: `docs/api.md` (endpoint) and `docs/architecture.md` (key file, where to get it, the `[apns]` keys).

Decided: no new dependency. ES256 reuses `p256` (as VAPID does). HTTP/2 is the system curl's `--http2` (nghttp2), which web push already shells out to. The `.p8` is parsed by taking the SEC1 key out of the PKCS#8 DER (checked against an `openssl pkcs8 -topk8` key).
Decided: `[apns] url` lets the tests point at a fake APNs server. It also works as an operator override (e.g. a proxy).

src/ net +223 (no budget given): the line cap reads 22166 / 22800 at this head.

## Evidence (head 3fff2ab0df9db87bd98201ad3b89cab7b6d3577c)

Round 3. At 0eeac32 CI `ui` failed two tests:
- `parity.spec.mjs:151`: the check word comes from 16 × 16 words, and both pair asks drew "cedar walrus". The decline locator then matched two cards (a strict mode violation).
- `chat.spec.mjs:131`: a long-task timing flake that staging's CI also fails (run 42589063167). This PR doesn't touch that test.

**Staging build fix** (`web/e2e/parity.spec.mjs`, +2 −1): the decline step now takes the newest matching card (`.last()`). There is no product change.

Earlier staging build fix (round 2, `web/ts/chat.ts`, +3 −2): only a state change into `connecting` shows the connecting line. Without it, an answer that arrived before the `working` state left the line up and hid the pill (`threads.spec.mjs:60`).

`just` isn't on this host, so I ran each recipe's commands directly at 3fff2ab:

| Command | Exit |
|---|---|
| `cargo fmt --all --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 |
| `RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --locked` | 0 |
| `scripts/line-cap.sh` / `ci/line-cap-test.sh` | 0 / 0 |
| `scripts/dep-direction.sh` / `scripts/hygiene.sh` | 0 / 0 |
| `cargo run --quiet --locked -p cooee -- theme check packs/themes/* packs/starter` | 0 |
| `cd web && npx tsc -p ts && npx tsc -p ts/tsconfig.sw.json && npx biome check ts` | 0 |
| `cd web/e2e && npx playwright test parity.spec.mjs -g "row 6"` | 0 (3 passed) |
| `cd web/e2e && npx playwright test` | 0 (177 passed, 23 skipped) |

Not run: `topcoat fmt --check` and `cargo deny --locked check`. Neither tool is on this host (`cargo deny` exits 101 with "no such command"). `Cargo.lock` and every `Cargo.toml` are unchanged against staging.

Tests (fake APNs server in `push.rs`, doctor in `tests/instances.rs`):
- `the_apns_jwt_has_the_key_id_and_team_and_lasts_under_an_hour`: header and claims, signature verified, the token reused before 50 min and remade at 50 min.
- `a_push_goes_to_each_apns_token_and_a_410_drops_it`: one POST per paired token with the headers and body. 410 and `BadDeviceToken` drop the token, an unpaired device's token is dropped unsent, and the 500 log line doesn't contain the key.
- `without_the_key_or_apns_table_nothing_goes_to_apns`: no table, no file, or a bad file means no request, a reason, and a 0644 file tightened. The secret file contents never appear in the log.
- `a_renewed_vapid_key_keeps_apns_tokens`.
- `doctor_reports_a_missing_shed`: the doctor warns with no `[apns]` and with no key file, and says `ok   apns ready` with a real key, never printing it.

AC 3 (a live push to the owner's iPhone) can't be checked until native v1 pairs.

## Why (round 1)

Decided: APNs sends reuse p256 for the ES256 provider JWT and system curl --http2 for HTTP/2; no new crate
Because: "prefer what the workspace already has"
Decided: APNs tokens live in push.json as Sub entries marked `apns`, so unpairing drops them like web subscriptions
Because: "Kept in `push.json` beside the web subscriptions, tied to the device"
Decided: optional `[apns] url` (default Apple's host per token env) so tests drive a fake APNs server
Rejected: a cfg(test)-only host override — production code only for tests; a url key is also an operator override
Rejected: enabling p256's pkcs8 feature — new crates in the lock; the .p8's SEC1 key is read from the DER directly
Check: crates/cooee-server/src/push.rs tests drive real curl against a fake APNs listener (headers, 410, BadDeviceToken, no key)
Check: crates/cooee/tests/instances.rs runs the real `cooee doctor` with no [apns], no key file, and a real openssl key
Owner: The native app gets Apple push beside web push once you drop apns.p8 in the data folder and fill [apns].

<sub>note on e9362ee · git log --notes=why</sub>


