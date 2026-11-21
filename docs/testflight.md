# TestFlight: the native app on your phone and your Mac (#144, #485)

No Mac needs Xcode. CI archives, signs and uploads the iOS and the macOS app; TestFlight puts them on the phone and
the Mac.

## How a build reaches the phone and the Mac

1. A merge to `main` that touches `apple/**` (or **Run workflow** on `apple-testflight` in the Actions tab, from
   `main`) starts `.github/workflows/apple-testflight.yml`: one job for iOS and one for macOS, in the same run.
2. The job reads the App Store Connect API key from the `testflight` environment. Only `main` may deploy from that
   environment, so pull request and fork runs can't read the key, and the job itself refuses any other branch.
3. It registers the bundle id `com.jirathip.cooee` if it is missing, finds the app and the latest TestFlight build of the job's
   platform, and sets the build number to that build + 1. Nobody bumps it by hand.
4. `xcodebuild archive` builds unsigned (no Development certificate is ever made); `-exportArchive` signs with
   `-allowProvisioningUpdates` and the API key. Apple manages the distribution certificate and profiles (cloud
   signing), so there is no `.p12`, no password and no fastlane. A dry run exports the signed build without uploading it.
   The Mac build carries the App Sandbox (`apple/Cooee-mac.entitlements`), which TestFlight requires of every Mac build.
5. Each job's summary shows the version, the build number and a TestFlight link. Apple processes the build for a
   few minutes, then the TestFlight app on the phone or the Mac offers it. If a job fails, its `xcodebuild-log-<platform>` artifact
   holds the raw log for 7 days, with the key id, issuer id and team scrubbed out.

## The owner's one-time steps

- Secrets in the `testflight` environment: `ASC_KEY_ID`, `ASC_ISSUER_ID` and `ASC_KEY_P8` (the base64 of the `.p8`).
  The key needs the **Admin** role so it may use cloud-managed distribution certificates. If it can't, the run fails
  and says so.
- The App Store Connect app record exists (created once by hand: Apps → + → New App): platform **iOS**, name
  **cooee: agent chat**, primary language **English (U.S.)**, bundle id **com.jirathip.cooee** (registered with Push
  Notifications), SKU **cooee-ios**, user access **Full Access**. If it is ever missing, the run fails and names these fields.
- The same record holds the Mac app: on the app's page in App Store Connect, **Add Platform** (in the sidebar) →
  **macOS** → Add. It keeps the bundle id **com.jirathip.cooee**; there is no second record. Until then the macOS job's
  upload fails, and the iOS one still uploads. Adding a platform can't be undone once both platforms pass App Review.
- In the app's TestFlight tab, add yourself to an internal testing group, then install the TestFlight app on the phone.

## Installing on the Mac

Install **TestFlight** from the Mac App Store and sign in with the Apple Account in the testing group. The cooee build
shows there: **Install** puts `Cooee.app` in `/Applications`, and TestFlight offers each newer build as an update. It
replaces an older `Cooee.app` that came from the `.dmg`. A TestFlight build expires after 90 days, and a newer merge
uploads a new one long before that.

The build declares that it uses no non-exempt encryption (only HTTPS), so TestFlight doesn't ask about export
compliance for each build.

## Local development without Xcode

With the Command Line Tools only:

- `cd apple && swift build` compiles `CooeeUI`, the screens and fixtures.
- `cd apple && swift run shots` renders every design frame to `apple/design/v1/`, so you can look at views without a simulator.
- `xcodebuild` (the app target, the iOS build) runs in CI: `.github/workflows/apple.yml` on each pull request,
  and `apple-testflight.yml` on `main`.
- On a Mac, install the TestFlight build (above); `xcodebuild` builds the Mac app locally.
- On a real device, use TestFlight.
