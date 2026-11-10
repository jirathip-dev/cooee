# TestFlight: the native app on your phone (#144)

No Mac needs Xcode. CI archives, signs and uploads the iOS app; TestFlight puts it on the phone.

## How a build reaches the phone

1. A merge to `staging` that touches `apple/**` (or **Run workflow** on `apple-testflight` in the Actions tab, from
   `staging`) starts `.github/workflows/apple-testflight.yml` on GitHub's macOS runner.
2. The job reads the App Store Connect API key from the `testflight` environment. Only `staging` may deploy from that
   environment, so pull request and fork runs can't read the key.
3. It registers the bundle id `com.jirathip.cooee` if it is missing, finds the app and the latest TestFlight build, and
   sets the build number to that build + 1. Nobody bumps it by hand.
4. `xcodebuild archive` and `-exportArchive` sign with `-allowProvisioningUpdates` and the API key. Apple manages
   the distribution certificate and profiles (cloud signing), so there is no `.p12`, no password and no fastlane.
5. The run's summary shows the version, the build number and a TestFlight link. Apple processes the build for a
   few minutes, then the TestFlight app on the phone offers it. If the run fails, its `xcodebuild-log` artifact
   holds the raw log for 7 days, with the key id, issuer id and team scrubbed out.

## The owner's one-time steps

- Secrets in the `testflight` environment: `ASC_KEY_ID`, `ASC_ISSUER_ID` and `ASC_KEY_P8` (the base64 of the `.p8`).
  The key needs the **Admin** role so it may use cloud-managed distribution certificates. If it can't, the run fails
  and says so.
- The App Store Connect app record exists (created once by hand: Apps → + → New App): platform **iOS**, name
  **cooee: agent chat**, primary language **English (U.S.)**, bundle id **com.jirathip.cooee** (registered with Push
  Notifications), SKU **cooee-ios**, user access **Full Access**. If it is ever missing, the run fails and names these fields.
- In the app's TestFlight tab, add yourself to an internal testing group, then install the TestFlight app on the phone.

The build declares that it uses no non-exempt encryption (only HTTPS), so TestFlight doesn't ask about export
compliance for each build.

## Local development without Xcode

With the Command Line Tools only:

- `cd apple && swift build` compiles `CooeeUI`, the screens and fixtures.
- `cd apple && swift run shots` renders every design frame to `apple/design/v1/`, so you can look at views without a simulator.
- `xcodebuild` (the app target, the iOS build) runs in CI: `.github/workflows/apple.yml` on each pull request,
  and `apple-testflight.yml` on `staging`.
- On a real device, use TestFlight.
