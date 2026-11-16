#!/bin/sh
# #142: the Mac app as the server, as a .dmg in <out>: cooee and shed (scripts/mac-bins.sh) inside the app, which runs
# them as login items. With MAC_SIGN_IDENTITY (a "Developer ID Application" identity in the keychain) and MAC_TEAM,
# everything is signed with the hardened runtime and a timestamp, ready to notarize; without them, ad hoc (no hardened
# runtime: its library validation would refuse the ad hoc Sparkle.framework). #333: with
# SPARKLE_ED_KEY (Sparkle's private EdDSA key, base64) its public key goes in Info.plist and turns the updater on;
# MAC_BUILD is the bundle version Sparkle compares (default 1); SPARKLE_FEED_URL overrides the appcast's address.
# Usage: scripts/mac-app.sh <out>
set -eu
cd "$(dirname "$0")/.."
out=${1:?usage: scripts/mac-app.sh <out>}
scripts/mac-bins.sh "$out/bin"
identity=${MAC_SIGN_IDENTITY:--}
flags=""
runtime=NO
[ "$identity" = - ] || { flags="--timestamp"; runtime=YES; }
sign() {
  if [ "$runtime" = YES ]; then
    codesign --force --preserve-metadata=entitlements --timestamp --options runtime --sign "$identity" "$1"
  else
    codesign --force --preserve-metadata=entitlements --sign - "$1"
  fi
}
public_key=""
[ -z "${SPARKLE_ED_KEY:-}" ] || public_key=$(swift scripts/sparkle-public-key.swift)
set -- CURRENT_PROJECT_VERSION="${MAC_BUILD:-1}" SPARKLE_PUBLIC_KEY="$public_key"
[ -z "${SPARKLE_FEED_URL:-}" ] || set -- "$@" SPARKLE_FEED_URL="$SPARKLE_FEED_URL"
xcodebuild -skipMacroValidation -project apple/Cooee.xcodeproj -scheme Cooee -configuration Release \
  -destination 'generic/platform=macOS' -derivedDataPath "$out/derived" COOEE_BIN_DIR="$out/bin" \
  CODE_SIGN_IDENTITY="$identity" CODE_SIGN_STYLE=Manual DEVELOPMENT_TEAM="${MAC_TEAM:-}" \
  ENABLE_HARDENED_RUNTIME=$runtime OTHER_CODE_SIGN_FLAGS="$flags" "$@" build
app="$out/derived/Build/Products/Release/Cooee.app"
sparkle="$app/Contents/Frameworks/Sparkle.framework"
for code in Versions/B/XPCServices/Installer.xpc Versions/B/XPCServices/Downloader.xpc Versions/B/Autoupdate \
  Versions/B/Updater.app .; do
  sign "$sparkle/$code"
done
sign "$app"
codesign --verify --deep --strict --verbose=2 "$app"
mkdir -p "$out/dmg"
ditto "$app" "$out/dmg/Cooee.app"
ln -sfn /Applications "$out/dmg/Applications"
hdiutil create -volname cooee -srcfolder "$out/dmg" -ov -format UDZO "$out/cooee.dmg"
[ "$identity" = - ] || codesign --sign "$identity" --timestamp "$out/cooee.dmg"
echo "built $out/cooee.dmg"
