Closes #142

## For humans

- On a Mac, cooee's app can now be the server: its first run sets up folders, sign-ins and Tailscale, starts cooee and shed in the background as login items, and shows a QR code for the phone.
- When a phone asks to pair, the Mac shows the check word and an Approve button; "Connect to a server elsewhere" keeps the Mac a client only.
- CI builds a downloadable `cooee.dmg`; it is signed and notarized once the owner adds the Apple Developer ID secrets, and until then the job says which secrets are missing.

## What changed

- `cooee pair asks` / `cooee pair approve|decline <id>`: the Mac app answers a phone's request from the server's own machine (as "this Mac", no passkey, like `cooee pair` makes a code without one).
- `MacSetupFeature` (TCA): welcome → folders & agents → sign-ins → Tailscale → QR → Approve; Done pairs the Mac itself. `MacServerClient` live value (`CooeeApp/MacLive.swift`): `SMAppService` login items for the bundled `cooee` and `shed` (`apple/LaunchAgents/`), config edits, `cooee pair`, Touch ID before approving when the Mac has it.
- The `mac-setup-*` views now render reducer state; their fixtures reproduce the `design/v1` frames unchanged.
- CI: `apple.yml` `mac-setup` job (the first run on a macOS runner, the iPhone simulator asks, the Mac approves; screens in `mac-setup-shots`); `apple-mac.yml` builds `cooee.dmg`, notarizes and checks it with `spctl -a -vv` when the `mac-release` secrets exist.

## Evidence

All at head `6fb572a99aab09d23ad9e0098b792417cecc074e` (PR #197, 14 of 14 checks `SUCCESS`, read with `herdsman pr cooee 142 --json`, exit 0).

| Check | Result | Run |
|---|---|---|
| mac-setup (AC2) | success | [https://github.com/jirathip-dev/cooee/actions/runs/42916784938/job/130181164510](https://github.com/jirathip-dev/cooee/actions/runs/42916784938/job/130181164510) |
| e2e | success | [job 130181164145](https://github.com/jirathip-dev/cooee/actions/runs/42916784938/job/130181164145) |
| build (generic/platform=macOS) (AC1) | success | [job 130181164517](https://github.com/jirathip-dev/cooee/actions/runs/42916784938/job/130181164517) |
| build (generic/platform=iOS Simulator) (AC1) | success | [job 130181164445](https://github.com/jirathip-dev/cooee/actions/runs/42916784938/job/130181164445) |
| apple-quality (AC1: unit tests incl. `MacSetupFeatureTests`, coverage, lint) | success | [job 130181164439](https://github.com/jirathip-dev/cooee/actions/runs/42916784938/job/130181164439) |
| dmg (AC3) | success | [job 130181163482](https://github.com/jirathip-dev/cooee/actions/runs/42916784721/job/130181163482) |
| ui | success | [job 130181164124](https://github.com/jirathip-dev/cooee/actions/runs/42916784737/job/130181164124) |
| msrv / stable (ubuntu, macos), ts, no-node, actionlint | success | [run 42916784737](https://github.com/jirathip-dev/cooee/actions/runs/42916784737) |

- AC2: the `mac-setup` job log at this head has `Waiting 600.0s for "Paired with 127.0.0.1" StaticText to exist` → found at t=34.86s, and `Executed 1 test, with 0 failures` for the phone-ask and the Mac-approve tests. The head run's screenshots are artifact [`mac-setup-shots`](https://github.com/jirathip-dev/cooee/actions/runs/42916784938/artifacts/13812029500): `mac-setup-welcome`, `-folders`, `-signins`, `-tailscale`, `-qr`, `-approve`, `-approved` (Mac, footer "cooee on · shed on") and `pair-waiting-look` / `pair-approved-look` (iPhone simulator, "Paired with 127.0.0.1"). The copies in `docs/screens/142/` are from the same job at 0885ebd. I checked the head run's `-approve` and `pair-approved-look` against them and they show the same screens. I did not commit them again: a new commit would make this head stale.
- AC3 without secrets: the `dmg` log at this head says `Building an unsigned cooee.dmg: no notarization without the secrets MAC_CERT_P12 MAC_CERT_PASSWORD NOTARY_KEY_P8 NOTARY_KEY_ID NOTARY_ISSUER_ID (environment mac-release; see apple/README.md).` Not verified: the notarized `.dmg` and `spctl -a -vv` need the owner's Apple Developer ID secrets.
- Rust at this head: the reviewer ran `cargo test -p cooee-core devices`, `cargo test -p cooee --test pairing` and `cargo clippy` and all were clean. The same tests run in CI `stable`/`msrv` above.

`src/ net`: unchanged from the earlier rounds of this PR; the apple cap was raised to 7500 (q1214, #204).
