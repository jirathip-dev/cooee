#if os(iOS)
    import XCTest

    extension LiveUITests {
        @MainActor
        internal func followsTheQueueAndFoldsARowOpen(_ app: XCUIApplication) {
            XCTAssert(labelled(app, "review 1 ▸ herdsman-rev-722").waitForExistence(timeout: 30))
            keep("queue-live")
            labelled(app, "#641").tap()
            let link = app.buttons["PR #704 ›"].firstMatch
            XCTAssert(link.waitForExistence(timeout: 10))
            keep("queue-row-open")
            opensTheLaneChatInTheSheet(app)
            labelled(app, "#641").tap()
            XCTAssert(link.waitForNonExistence(timeout: 10))
        }

        @MainActor
        private func opensTheLaneChatInTheSheet(_ app: XCUIApplication) {
            let horse = labelled(app, "#641 fix 1, on the trail")
            let chats = app.buttons.matching(identifier: "lane chat ›")
            (horse.exists ? horse : chats.element(boundBy: chats.count - 1)).tap()
            let back = app.buttons["‹ Queue"].firstMatch
            XCTAssert(back.waitForExistence(timeout: 10))
            XCTAssert(
                labelled(app, "herdsman-impl-641").exists && labelled(app, "Claude Code · opus 5.5 · mini").exists)
            XCTAssert(labelled(app, "An ambiguous label now refuses").waitForExistence(timeout: 10))
            keep("lane-tapped")
            back.tap()
            XCTAssert(back.waitForNonExistence(timeout: 10))
            XCTAssert(app.buttons["PR #704 ›"].firstMatch.waitForExistence(timeout: 10))
        }
    }
#endif
