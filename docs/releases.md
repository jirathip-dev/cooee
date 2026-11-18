# Releases and updates (#584)

cooee ships the way drover does: CI builds and signs, hosts only download. Nothing builds cooee from source on a host,
and no host needs a signing certificate.

## The release workflow

`.github/workflows/release.yml` runs after the `ci` workflow ends green on a push to main or staging (`workflow_run`).
It builds `aarch64-apple-darwin` (macOS runner) and `x86_64-unknown-linux-gnu` (Ubuntu 22.04, for a low glibc floor)
with `COOEE_REV` set to the commit, so `cooee --version` names it, then publishes one GitHub release:

| | main | staging |
|---|---|---|
| tag | `build-<sha7>` | `build-staging-<sha7>` |
| kind | release, kept for good, marked latest | prerelease; the staging run deletes those over 14 days old, tag included |
| assets | `cooee-aarch64-apple-darwin`, `cooee-x86_64-unknown-linux-gnu`, `SHA256SUMS` | the same |

`scripts/release-sign.sh` signs the macOS binary with the Developer ID Application certificate (hardened runtime,
secure timestamp, identifier `cooee`), notarizes it with an App Store Connect API key, and checks it with
`codesign -dv`, `codesign -v -R=notarized --check-notarization` and `spctl`. Only the signed binary is published.

A failed build, signing or publish step ends its job green with an error annotation and publishes nothing, so the
workflow never turns a branch red. Staging builds are for verify lanes and TestFlight dry runs; hosts follow main.

### Secrets

Set by the owner in the repo's `release` environment; only the macOS job reads them (drover's are the same cert and key):

| Name | What |
|---|---|
| `MACOS_CERT_P12` | the Developer ID Application certificate and key, a base64 `.p12` |
| `MACOS_CERT_PASSWORD` | that file's password |
| `ASC_API_KEY_P8` | the App Store Connect API key, a base64 `.p8` |
| `ASC_API_KEY_ID` | its key id |
| `ASC_API_ISSUER_ID` | its issuer id |

## Install and update

`scripts/install.sh` downloads main's latest release to `~/.local/bin/cooee`, checks it against `SHA256SUMS` and runs
`cooee setup`.

`cooee update` (and its launchd timer, `dev.cooee.<instance>.update`, every 15 minutes) reads main's head, downloads
`build-<sha7>`'s asset to `~/.local/bin/cooee.new`, checks it against `SHA256SUMS` and that its `--version` names the
head's sha, swaps it in and restarts serve. A head with no release yet waits; a build that fails a check, or whose serve
doesn't answer, is rolled back and not tried again. `cooee update --check` says what it would do.

## shed

cooee never installs, upgrades or restarts shed; shed updates itself from its own releases. `cooee setup` and
`cooee doctor` only check that the running shed's protocol is at or above `SHED_MIN_PROTOCOL` (any newer one is
accepted), and name the fix when it isn't: setup then stops before it changes anything.
