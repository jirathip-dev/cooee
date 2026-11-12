set -u
OUT="$1"; DD="$2"; cd apple
sim="$SIMULATOR_UDID"
xcrun simctl boot "$sim" 2>/dev/null; xcrun simctl bootstatus "$sim" -b >/dev/null
build=(xcodebuild -skipMacroValidation -project Cooee.xcodeproj -scheme Cooee -destination "id=$sim" -derivedDataPath "$DD")
"${build[@]}" build-for-testing >"$OUT/xcb-build.log" 2>&1; echo "build-for-testing exit=$?"
until [ -s "$OUT/pair" ]; do sleep 5; done
for look in dark light; do
  xcrun simctl ui "$sim" appearance "$look"
  xcrun simctl keychain "$sim" reset
  link=$(sh -c "$(cat "$OUT/pair.again")" | sed -n 's/^link  //p')
  rm -rf "$OUT/e2e-$look.xcresult"
  HERDSMAN_XCODEBUILD_DIRECT=1 TEST_RUNNER_COOEE_LOOK="$look" TEST_RUNNER_COOEE_PAIR_LINK="$link" "${build[@]}" \
    -resultBundlePath "$OUT/e2e-$look.xcresult" -only-testing:CooeeAppUITests/LiveUITests/testPairsStreamsAndShowsHome \
    test-without-building >"$OUT/xcb-test-$look.log" 2>&1
  echo "test $look exit=$?"
  xcrun xcresulttool export attachments --path "$OUT/e2e-$look.xcresult" --output-path "$OUT/att-$look"; echo "export $look exit=$?"
done
