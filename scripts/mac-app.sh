#!/bin/sh
# #142: the Mac app as the server, as a .dmg in <out>: cooee and shed (scripts/mac-bins.sh) inside the app, which runs
# them as login items. With MAC_SIGN_IDENTITY (a "Developer ID Application" identity in the keychain) and MAC_TEAM,
# everything is signed with the hardened runtime and a timestamp, ready to notarize; without them, ad hoc.
# Usage: scripts/mac-app.sh <out>
set -eu
cd "$(dirname "$0")/.."
out=${1:?usage: scripts/mac-app.sh <out>}
scripts/mac-bins.sh "$out/bin"
identity=${MAC_SIGN_IDENTITY:--}
flags=""
[ "$identity" = - ] || flags="--timestamp"
xcodebuild -skipMacroValidation -project apple/Cooee.xcodeproj -scheme Cooee -configuration Release \
  -destination 'generic/platform=macOS' -derivedDataPath "$out/derived" COOEE_BIN_DIR="$out/bin" \
  CODE_SIGN_IDENTITY="$identity" CODE_SIGN_STYLE=Manual DEVELOPMENT_TEAM="${MAC_TEAM:-}" \
  ENABLE_HARDENED_RUNTIME=YES OTHER_CODE_SIGN_FLAGS="$flags" build
app="$out/derived/Build/Products/Release/Cooee.app"
codesign --verify --deep --strict --verbose=2 "$app"
mkdir -p "$out/dmg"
ditto "$app" "$out/dmg/Cooee.app"
ln -sfn /Applications "$out/dmg/Applications"
hdiutil create -volname cooee -srcfolder "$out/dmg" -ov -format UDZO "$out/cooee.dmg"
[ "$identity" = - ] || codesign --sign "$identity" --timestamp "$out/cooee.dmg"
echo "built $out/cooee.dmg"
