# The app in the simulator, once per appearance, set with simctl (the test's own XCUIDevice.appearance left this
# simulator light); each run pairs afresh, so the keychain is emptied first. The app builds before any code is minted:
# a code lives 10 min. $ONLY: the scope's `-only-testing:` arguments (empty runs every test).
cd apple
xcodebuild -skipMacroValidation -project Cooee.xcodeproj -scheme Cooee -destination "id=$SIM" \
  -derivedDataPath "$E2E/derived" build-for-testing
xcrun simctl bootstatus "$SIM" -b >>"$E2E/boot.log" 2>&1 || { cat "$E2E/boot.log"; exit 1; }
read -ra only <<< "${ONLY:-}"
for look in dark light; do
  xcrun simctl ui "$SIM" appearance "$look"
  xcrun simctl keychain "$SIM" reset
  link=$(pair_link)
  rm -rf "${E2E:?}/e2e-$look.xcresult"
  TEST_RUNNER_COOEE_LOOK="$look" TEST_RUNNER_COOEE_PAIR_LINK="$link" xcodebuild test-without-building \
    -xctestrun "$(ls "$E2E"/derived/Build/Products/*.xctestrun)" -destination "id=$SIM" \
    -resultBundlePath "$E2E/e2e-$look.xcresult" "${only[@]}"
done
