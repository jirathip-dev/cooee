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

Pending CI at the head; updated below once it is green.

