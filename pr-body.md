Closes #603

## For humans
Plugin screens no longer save themselves to disk on every live update. The saved copy (shown when the Mac is unreachable) is now written in the background, at most once every two seconds while updates arrive. It is also written straight away when you leave the screen and when the app goes to the background, so the copy you see on reopening is never more than a moment old.

## What changed
- `ScreenFeature.keep` returns a cancellable effect that waits 2 s on the injected clock, then saves. Each new event or action reply resets it, so a burst ends in one write. The encode and the write run inside `.run`, off the main actor.
- The offline fallback `cache.load` moved from the reducer into the follow effect's failure path (`.failed(who, kept)`), so the reducer does no file I/O.
- Leaving a screen saves it at once: in-screen `navigate` flushes the old screen, and the new `keepsLeftScreens` reducer wrapper (`ScreenKeeping.swift`) saves every screen its host drops. It is applied to the menu page, the chat's plugin sheet and the sidebar screens. This covers dismissal, which would otherwise cancel the pending save with the child's effects.
- Going to the background saves at once: `ScreenCache.backgrounds` is a stream that `.task` listens to and that sends `.background`, which flushes. The live stream comes from `UIApplication.didEnterBackgroundNotification` on iOS and `NSApplication.didResignActiveNotification` on macOS.

## Tests
- `aHundredEventsInASecondAreKeptOnceTwoSecondsAfterTheLast`: 100 distinct events 10 ms apart cause 0 writes until 2 s after the last one, then exactly 1.
- `leavingAScreenKeepsItAtOnce`: navigating away writes the old screen at once.
- `goingToTheBackgroundKeepsTheScreenAtOnce`: a background signal writes the screen before the debounce ends.
- `aServerRowOpensCoresScreenAndClosingTheMenuKeepsAndDropsIt` (MenuFeatureTests): closing the menu while a save is pending writes the page once. Without the wrapper on MenuFeature it fails with 0 writes (checked locally).

## Evidence at 04a4630473029dca3147cb88fdd414ce2318d417
- `scripts/apple-check.sh` → exit 0 (format, anti-slop, SwiftLint, strict Swift 6 build, all tests with coverage)
- `ci/gates/comments.sh` → exit 0
- Merged origin/staging (b91c4869e). Staging's new `TimingTests` block-action test now advances its test clock 2 s so the debounced cache save finishes before `store.finish()`; nothing else changed.

`src/ net +90 (budget: none stated)`: the debounce, the leave wrapper and the live background stream that the q2036 amendment asked for.

Decided: a sidebar screen whose view goes offscreen but stays in the layout keeps its pending save (its state and effect live on), so it needs no flush.

## Why (round 2)

Decided: keep returns a 2 s debounced cancellable effect on the injected clock; save runs inside .run, off the main actor
Because: "The screen cache is written off the main thread, at most once per 2 s"
Decided: hosts wrap their reducer in keepsLeftScreens, which saves every screen the host drops, so a dismissal loses no pending save
Because: "plus on leaving the screen and on the app going to the background"
Decided: ScreenCache.backgrounds streams app-background notifications; .task listens and .background flushes at once
Because: q2036
Rejected: a .leave action sent from onDisappear — it arrives after ifLet has nilled the child, so it is dropped
Rejected: writing on effect cancellation — a newer event's cancelInFlight would also write, defeating the debounce
Check: MenuFeatureTests closing the menu while a save is pending writes once; it fails without the wrapper
Check: KeepsLeftScreens calls base._reduce because TCA deprecates calling reduce(into:) directly (warnings are errors)
Owner: busy plugin screens save their offline copy in the background at most every 2 s, and at once when you leave or switch away from the app

<sub>note on c69d2b7 · git log --notes=why</sub>

