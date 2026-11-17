# The app in the simulator, once per appearance, set with simctl (the test's own XCUIDevice.appearance left this
# simulator light); each run pairs afresh, so the keychain is emptied first. The app builds before any code is minted:
# a code lives 10 min. $ONLY: the scope's `-only-testing:` arguments (empty runs every test).
cd apple
xcodebuild -skipMacroValidation -project Cooee.xcodeproj -scheme Cooee -destination "id=$SIM" \
  -derivedDataPath "$E2E/derived" build-for-testing
xcrun simctl bootstatus "$SIM" -b >>"$E2E/boot.log" 2>&1 || { cat "$E2E/boot.log"; exit 1; }
read -ra only <<< "${ONLY:-}"
# #484: the phone, unpaired, asks the CLI server by address and `cooee pair approve` answers it, as on a Mac that runs
# `cooee setup`; no Mac app.
case " ${ONLY:-LiveUITests} " in
  *LiveUITests[\ /]*)
    xcrun simctl keychain "$SIM" reset
    pair=$(cat "$E2E/pair.again")
    link=$(pair_link)
    TEST_RUNNER_COOEE_ASK_ADDRESS="${link%%/pair*}" xcodebuild test-without-building \
      -xctestrun "$(ls "$E2E"/derived/Build/Products/*.xctestrun)" -destination "id=$SIM" \
      -resultBundlePath "$E2E/e2e-ask.xcresult" -only-testing:CooeeAppUITests/LiveUITests/testAsksAndTheCLIApprovesIt &
    asking=$!
    approved=""
    for _ in $(seq 300); do
      id=$(sh -c "$pair asks" | grep -o '"id":"[0-9a-f]*"' | head -1 | cut -d'"' -f4)
      [ -z "$id" ] || { sh -c "$pair approve $id" && approved=$id && break; }
      sleep 1
    done
    wait "$asking"
    [ -n "$approved" ] || { echo "no ask reached \`cooee pair asks\`"; exit 1; }
    ;;
esac
for look in dark light; do
  xcrun simctl ui "$SIM" appearance "$look"
  xcrun simctl keychain "$SIM" reset
  link=$(pair_link)
  rm -rf "${E2E:?}/e2e-$look.xcresult"
  TEST_RUNNER_COOEE_LOOK="$look" TEST_RUNNER_COOEE_PAIR_LINK="$link" xcodebuild test-without-building \
    -xctestrun "$(ls "$E2E"/derived/Build/Products/*.xctestrun)" -destination "id=$SIM" \
    -resultBundlePath "$E2E/e2e-$look.xcresult" "${only[@]}"
done
