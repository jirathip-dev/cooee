#!/bin/bash
set -u
cd apple
out="$VOUT"
build=(xcodebuild -skipMacroValidation -project Cooee.xcodeproj -scheme Cooee -destination "id=$SIMULATOR_UDID" -derivedDataPath "$DD")
xcrun simctl bootstatus "$SIMULATOR_UDID" -b >/dev/null
"${build[@]}" build-for-testing >"$out/build.log" 2>&1; echo "build-for-testing exit=$?"
for look in dark light; do
  xcrun simctl ui "$SIMULATOR_UDID" appearance "$look"
  xcrun simctl keychain "$SIMULATOR_UDID" reset
  link=$(sh -c "$(cat "$out/pair.again")" | sed -n 's/^link  //p')
  rm -rf "$out/e2e-$look.xcresult"
  TEST_RUNNER_COOEE_LOOK="$look" TEST_RUNNER_COOEE_PAIR_LINK="$link" "${build[@]}" \
    -resultBundlePath "$out/e2e-$look.xcresult" test-without-building >"$out/test-$look.log" 2>&1
  echo "test $look exit=$?"
  xcrun xcresulttool export attachments --path "$out/e2e-$look.xcresult" --output-path "$out/shots-$look"; echo "export $look exit=$?"
done
