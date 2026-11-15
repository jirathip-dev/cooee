#if os(iOS)
    import XCTest

    internal final class LiveControlsUITests: LiveUITestCase {
        @MainActor
        internal func testHeaderChipSheetAvatarAndDelete() throws {
            let app = try launchHome()
            send(app, "slow 3")
            app.buttons["Agent"].firstMatch.tap()
            let agent = app.buttons["fake-acp"].firstMatch
            XCTAssert(agent.waitForExistence(timeout: 10))
            keep("chat-agent-menu")
            agent.tap()
            app.buttons["Coder"].firstMatch.tap()
            let avatar = app.buttons["Edit avatar"].firstMatch
            XCTAssert(avatar.waitForExistence(timeout: 10))
            keep("persona-from-header")
            avatar.tap()
            let green = app.buttons["Green"].firstMatch
            XCTAssert(green.waitForExistence(timeout: 10))
            keep("persona-avatar-menu")
            green.tap()
            let delete = app.buttons["Delete Coder…"].firstMatch
            app.swipeUp()
            XCTAssert(delete.waitForExistence(timeout: 10))
            delete.tap()
            XCTAssert(app.alerts.firstMatch.waitForExistence(timeout: 10))
            keep("persona-delete-asks")
            app.alerts.firstMatch.buttons["Cancel"].tap()
            app.buttons["Close"].firstMatch.tap()
        }
    }
#endif
