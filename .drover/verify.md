# cooee verify recipe (native app)

No `sim-task` needed: `apple/e2e/run.sh` makes its own simulator (`xcrun simctl`), starts a real `cooee serve` with
fixtures (`apple/e2e/serve.sh`), runs the live UI tests in dark and light, exports their screenshots and deletes the
simulator.

## Doctor

- `xcrun simctl list devices available | grep 'iPhone 17'` lists a device type (run.sh creates an "iPhone 17").
- `command -v cargo node python3 xcodebuild` finds all four.

## Launch

From the repo root, with the scenario's test classes from the Feature map. `E2E_DIR` must be a new empty folder:
run.sh waits for its `pair` file, and a stale one pairs with a dead code.

```
E2E_DIR=$(mktemp -d "$DROVER_SCRATCH_TARGET/e2e.XXXX") SHOTS="$DROVER_SCRATCH_TARGET/shots" PORT="$DROVER_PORT" \
  ONLY="-only-testing:CooeeAppUITests/<Class>" PERSONA=false apple/e2e/run.sh > verify-run.log 2>&1; echo "exit=$?"
```

Ready signal: `** TEST EXECUTE SUCCEEDED **` once per look in verify-run.log; exit 0. `PERSONA=false` skips
LivePersonaTests, which reads back personas only `LivePersonaUITests` makes; set it true only with that class.
Takes 10 to 20 minutes (cargo and xcodebuild builds).

## Drive

The UI tests drive the app; each `keep("<name>")` in `apple/UITests/` saves the screen at that step. A FAIL names the
XCTAssert message in verify-run.log.

## Evidence

`$SHOTS/dark/` and `$SHOTS/light/` hold one PNG per `keep`, named by UUID; each look's `manifest.json` maps it to
its suggested name `<name>-<look>_0_….png`; `$SHOTS/serve.log` is the server's log.

## Feature map

- Chat header's … persona panel (#236): class `LiveDroverUITests`, shot `persona-panel` (Edit Conductor, New chat with
  Conductor, no Attach row).
- Other screens: `apple/e2e/map.toml` pairs paths with their classes.

## Known noise

- `xcrun xcresulttool export` warnings at teardown are harmless when the PNGs are listed after them.
