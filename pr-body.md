Closes #516

## For humans
On Home and the Mac sidebar, a persona with several topics now gets one row. It shows how many topics there are, which topic a tap opens, and the unread total across them.
Tapping that row or the pinned avatar opens the topic with the newest unread message. With nothing unread, it opens the topic you last had open.
Tapping a notification always opens the topic that message belongs to.

## What changed
- Core: `Listing.opens` (newest unread, else last open, else newest) replaces #432's "newest" rule. `Listing.collapsed` draws one row per persona the server counts topics for, at its newest topic's place. The row shows the server's `topics` and summed `unread` (`HomeAnswer.counts`, decoded from `/screens/home`'s personas).
- Features: `.pin` uses the rule. A trailing reducer records each persona's last-open topic (`home.opened`). `PushClient.opened` streams tapped notifications, and `.taps` opens each one's thread.
- App: `PushDelegate` is the notification-center delegate. A tap yields `aps.thread-id` (the server already sends the thread id there), and a tap that comes before the app listens is held until it does.
- UI: `ChatRow` draws the topics variant (`N topics` chip, folder glyph, **title** · last line, summed badge) when a persona has more than one topic. A tap on such a row goes through `.pin`.

## Evidence (head 4e09ebdb779aaa8468ecba8be979867ad4ee3ab4)
- `scripts/apple-check.sh` → exit 0 (format, anti-slop, SwiftLint, Swift 6 strict build, tests with coverage)
- `swift test --filter SnapshotTests` (in apple/) → exit 0, including the 4 new `home-persona-*` frames against `apple/design/topics/`
- `scripts/line-cap.sh` on a clean export of the head → exit 0: non-test 28262 / 28390, apple 16228 / 16300
- `ci/gates/comments.sh` → exit 0
- New TestStore tests: `PinnedTests.unreadInAnOlderTopicWinsOverANewerReadOne`, `withNothingUnreadThePinOpensTheTopicLastOpenNotTheNewest`, `aPushOpensItsOwnTopicOverAnUnreadOne`, `HomeFeatureTests.aPersonasRowCountsAndSumsAsTheServerDoesAndNamesTheTopicATapOpens`. `ClientTests` decodes the counts.

src/ net: apple +124 (16104 → 16228), crates/web +0. No budget is stated in the issue. Most of the growth is the push delegate and the row variant.

## Not covered
- The `mac-*` design frames are not in the snapshot suite. The sidebar row matched the design. The chat column did not: #515's wide chat draws a second Workflow chip row and does not cap the transcript at 760pt. That is outside this issue.
- "Last open" is kept in memory, so after a relaunch the fallback is the newest topic until a topic is opened again.
- A real push tap on a device was not tried here.
