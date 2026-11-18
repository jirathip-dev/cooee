Closes #485

## For humans

The Mac app now reaches you through TestFlight, from the same run that uploads the iPhone app, and gets updates there.
The old .dmg build with its Developer ID signing and notarization is gone.
One-time step: on the cooee app in App Store Connect, Add Platform → macOS (same app, same bundle id). Then install TestFlight from the Mac App Store.
After this merges, the mac-release secrets (MAC_CERT_*, NOTARY_*, SPARKLE_ED_KEY) and the MAC_RUNNER_APPLE_MAC variable can be deleted.

## Apple's rules, checked first

- Every Mac build uploaded to App Store Connect, TestFlight included, must have the App Sandbox on, or the upload fails with ITMS-90296 ([Apple forums](https://forums.developer.apple.com/forums/thread/768361)).
- One record holds both platforms: Add Platform puts macOS on the iOS app record under the same bundle id, SKU and Apple ID ([App Store Connect Help](https://developer.apple.com/help/app-store-connect/create-an-app-record/add-platforms/)). Once both platforms pass App Review, macOS can't be removed from the record.

So TestFlight can take the Mac app; this PR goes ahead and removes `dmg`.

## What changed

- `apple-testflight.yml`: a matrix with `testflight (iOS)` and `testflight (macOS)` in the same run, `fail-fast: false`. The build number is the latest build of that platform (`filter[preReleaseVersion.platform]`) + 1. Each job writes the same summary (version, build, TestFlight link to `/testflight/ios` or `/macos`). Each platform uploads its own failure log.
- `apple/Cooee-mac.entitlements` (new, macOS only through `CODE_SIGN_ENTITLEMENTS[sdk=macosx*]`):

| Entitlement | Feature that needs it |
| --- | --- |
| `com.apple.security.app-sandbox` | required for TestFlight / App Store Mac builds |
| `com.apple.security.network.client` | the API, streaming chat (SSE/WebSocket), pairing by address and code, the This Mac port probe, and Bonjour browsing for Nearby (sandboxed apps need network.client for NWBrowser) |
| `com.apple.security.device.camera` | the Mac's QR scanner in pairing (`CooeeUI/Scanner.swift`) |
| `com.apple.security.files.user-selected.read-only` | attachments (`Composer.swift` fileImporter) and theme image import (`ThemeDraw.swift`) |
| `com.apple.developer.aps-environment` | notifications (remote push, `CooeeApp/Push.swift`); the export sets production |
| `com.apple.security.temporary-exception.files.home-relative-path.read-only` = `/.config/cooee/cooee/` | reading `config.toml`'s port so a cooee on this Mac shows as This Mac in Nearby |

  Keychain: none needed. The client writes generic-password items in its own sandboxed keychain access, so no `keychain-access-groups` entitlement.
- `Info-mac.plist`: `LSApplicationCategoryType`, which a Mac App Store upload requires, and `NSBonjourServices` (`_cooee._tcp`). The `INFOPLIST_KEY_NSBonjourServices` setting reached the iOS plist only; I confirmed this in the built Mac app's Info.plist.
- `CooeeAPI/Local.swift`: `LocalConfig.standard` reads the real home folder from `getpwuid`. In a sandbox, `URL.homeDirectory` is the app's container.
- Deleted `.github/workflows/apple-mac.yml` and `scripts/mac-app.sh`. `scripts/ci-scope.sh`, `apple/README.md` and `docs/ci-runners.md` no longer name them.
- `docs/testflight.md`: covers the Mac, including the owner's one-time Add Platform → macOS step (same bundle id `com.jirathip.cooee`) and installing through TestFlight for Mac.

Required checks: `gh api repos/jirathip-dev/cooee/rules/branches/staging` returns `[]` and classic protection 404s for this lane's token, so I can't see whether staging's required checks name `dmg`. **Owner: if they do, drop `dmg` from the staging ruleset.**

## Evidence (head 0640aed730e9cb618bd396fb4e198ff59d61a3eb)

At 0640aed730e9cb618bd396fb4e198ff59d61a3eb (`origin/staging` merged in again; staging's new commits touch none of the files this PR changes):

| Command | Exit |
| --- | --- |
| `cargo test -q -p cooee --test routines` (red in CI at 620e5ad; 3 repeats of the failing test passed too) | 0 |
| `ci/gates/comments.sh` | 0 |
| `ci/gates/ci-scripts.sh` | 0 |

CI's red `stable (ubuntu-latest)` at 620e5ad was `routines::a_signed_post_to_a_webhook_routine_starts_one_thread_per_delivery_under_the_rate`. The 6th post was skipped for rate, then an earlier delivery's `ran` record landed later (`last_run.at` 1794991760890, after the last `recent` entry 1794991760882) and overwrote it. That race is in staging's routines code, which this PR doesn't touch. The round before, `skills` in `msrv` failed the same way (also a flake, in code this PR doesn't touch).

Apple evidence at 0444a74b97c4c35e38eb92ae2be90269cd221ad6 (the same Apple tree as the head):

| Command | Exit |
| --- | --- |
| `scripts/apple-check.sh` (format, anti-slop, SwiftLint, Swift 6 build, tests + coverage) | 0 |
| `xcodebuild -skipMacroValidation -project Cooee.xcodeproj -scheme Cooee -configuration Release -destination 'generic/platform=macOS' CODE_SIGNING_ALLOWED=NO build` | 0 |
| `codesign --force --deep --sign - --entitlements <Cooee-mac.entitlements without aps> Cooee.app`, then `codesign -d --entitlements -` shows sandbox, network.client, camera, user-selected read-only, home-relative exception | 0 |
| `ci/gates/comments.sh` | 0 |
| `ci/gates/ci-scripts.sh` | 0 |
| `ci/gates/cap.sh` (with the ignored local `apple/.build` moved aside; it counts build output on disk) | 0 |
| `plutil -lint Cooee-mac.entitlements Info-mac.plist` | 0 |

I launched the ad-hoc, sandboxed Release app locally. It ran, macOS created its container `~/Library/Containers/com.jirathip.cooee`, and Cooee log lines showed no sandbox denials. Then I stopped it.

## Not verified

- **No TestFlight Mac build yet.** Uploads run only on a merge to `main`, which a lane can't trigger. Until the owner adds the macOS platform, `testflight (macOS)` fails at upload and `testflight (iOS)` still uploads. The first main run shows the Mac build or names what blocked it.
- **Client features not exercised sandboxed:** Nearby pairing, address and code pairing, streaming chat, attachments and notifications were not tried against a server on the Mac. Push also needs the real profile (ad-hoc signing can't carry `aps-environment`).
- **#484's login-item cleanup** runs `launchctl bootout` from the app. A sandbox may deny it. It fails silently (no notice), so a Mac that still runs the old `serve`/`shed` login items may need `launchctl bootout gui/$(id -u)/com.jirathip.cooee.serve` (and `.shed`) once by hand.

src/ net +4 (no budget in the issue): only `Local.swift`'s real-home lookup; the rest is workflow, plist and docs.

## Why (round 1)

Decided: Apple's rules verified first: every Mac build uploaded to App Store Connect (TestFlight too) must carry the App Sandbox, and Add Platform puts macOS on the iOS record under the same bundle id
Because: "If TestFlight can't take the Mac app, the PR stops there and says why"
Decided: one matrix job per platform in the same apple-testflight run; the build number reads the latest build filtered by preReleaseVersion.platform, so each platform counts on its own
Because: "with the same build number rule (latest TestFlight build + 1, per platform)"
Decided: LocalConfig reads the real home from getpwuid, since a sandboxed URL.homeDirectory is the container; a read-only temporary exception opens only ~/.config/cooee/cooee/
Because: apple/Sources/CooeeAPI/Local.swift:49
Decided: LSApplicationCategoryType goes in Info-mac.plist, which a Mac App Store upload requires
Because: apple/Info-mac.plist:6
Rejected: a second App Store Connect record or bundle id for the Mac — the issue asks for the same bundle id, and Universal Purchase keeps one record
Rejected: keychain-access-groups — the legacy keychain items the client writes need no entitlement in the sandbox
Check: the testflight (macOS) job on the first main merge uploads; until the owner adds the macOS platform it fails and iOS still uploads (fail-fast off)
Owner: The Mac app now arrives through TestFlight like the iPhone app; add macOS to the app record once, then delete the mac-release secrets.

<sub>note on 0444a74 · git log --notes=why</sub>

## Why (round 2)

Decided: Apple's rules verified first: every Mac build uploaded to App Store Connect (TestFlight too) must carry the App Sandbox, and Add Platform puts macOS on the iOS record under the same bundle id
Because: "If TestFlight can't take the Mac app, the PR stops there and says why"
Decided: one matrix job per platform in the same apple-testflight run; the build number reads the latest build filtered by preReleaseVersion.platform, so each platform counts on its own
Because: "with the same build number rule (latest TestFlight build + 1, per platform)"
Decided: LocalConfig reads the real home from getpwuid, since a sandboxed URL.homeDirectory is the container; a read-only temporary exception opens only ~/.config/cooee/cooee/
Because: apple/Sources/CooeeAPI/Local.swift:49
Decided: fix round: the red msrv skills test is a flake in code this PR does not touch; merged staging and re-ran CI instead of editing it
Because: crates/cooee/tests/skills.rs:289
Rejected: a second App Store Connect record or bundle id for the Mac — the issue asks for the same bundle id, and Universal Purchase keeps one record
Check: the testflight (macOS) job on the first main merge uploads; until the owner adds the macOS platform it fails and iOS still uploads (fail-fast off)
Owner: The Mac app now arrives through TestFlight like the iPhone app; add macOS to the app record once, then delete the mac-release secrets.

<sub>note on 620e5ad · git log --notes=why</sub>

