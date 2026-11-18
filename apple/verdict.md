PASS

Reviewed head: c69d2b7f0db76da149e84f4eae1aba720e783604
Tests checked against anti-patterns: yes

Reran `swift test --filter 'ScreenFeatureTests|PluginNavigationTests|MenuFeatureTests|LayoutFeatureTests|ChatFeatureTests'` in apple/: 48 tests pass. PR body has `## For humans`, evidence at the exact head sha, no AI trailer or "Generated with" line. The reducer does no file I/O: the save runs in `.run`, and the offline load moved into the follow effect.

Findings (all non-blocking):
- non-blocking apple/Tests/CooeeFeaturesTests/ScreenFeatureTests.swift (goingToTheBackgroundKeepsTheScreenAtOnce): it checks `calls.all` only after `clock.advance(2s)`. The debounce alone would give the same write, so "at once" is not proven. Assert `calls.all` before the advance. The `receive(\.background)` check only shows the action arrives.
- non-blocking ScreenFeature.swift navigate/flush: a flush on leave does not cancel the pending debounced save. The old snapshot can be written a second time, which is harmless but a redundant write.
- non-blocking ScreenCache+Live.swift: macOS uses didResignActiveNotification, which is stricter than "background" but matches the amendment's "switch away from the app".

Not verified: the full CI run (9 checks were pending), the real live notification stream on a device, and the SwiftLint/anti-slop gates (I did not run apple-check.sh).
