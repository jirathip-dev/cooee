#!/bin/sh
# #142, #484: the Mac app, a client only, as a .dmg in <out>; the server on a Mac is the CLI install (`cooee setup`).
# With MAC_SIGN_IDENTITY (a "Developer ID Application" identity in the keychain) and MAC_TEAM it is signed with the
# hardened runtime and a timestamp, ready to notarize; without them, ad hoc. MAC_BUILD is the bundle version (default 1).
# Usage: scripts/mac-app.sh <out>
set -eu
cd "$(dirname "$0")/.."
out=${1:?usage: scripts/mac-app.sh <out>}
identity=${MAC_SIGN_IDENTITY:--}
flags=""
runtime=NO
[ "$identity" = - ] || { flags="--timestamp"; runtime=YES; }
xcodebuild -skipMacroValidation -project apple/Cooee.xcodeproj -scheme Cooee -configuration Release \
  -destination 'generic/platform=macOS' -derivedDataPath "$out/derived" CURRENT_PROJECT_VERSION="${MAC_BUILD:-1}" \
  CODE_SIGN_IDENTITY="$identity" CODE_SIGN_STYLE=Manual DEVELOPMENT_TEAM="${MAC_TEAM:-}" \
  ENABLE_HARDENED_RUNTIME=$runtime OTHER_CODE_SIGN_FLAGS="$flags" build
app="$out/derived/Build/Products/Release/Cooee.app"
codesign --verify --deep --strict --verbose=2 "$app"
mkdir -p "$out/dmg"
ditto "$app" "$out/dmg/Cooee.app"
ln -sfn /Applications "$out/dmg/Applications"
hdiutil create -volname cooee -srcfolder "$out/dmg" -ov -format UDZO "$out/cooee.dmg"
[ "$identity" = - ] || codesign --sign "$identity" --timestamp "$out/cooee.dmg"
echo "built $out/cooee.dmg"
