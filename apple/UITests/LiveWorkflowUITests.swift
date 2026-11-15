#if os(iOS)
    import XCTest

    internal final class LiveWorkflowUITests: LiveUITestCase {
        @MainActor
        internal func testWorkflowSheetHomeAndOthers() throws {
            let app = try launchHome()
            let new = newChat(app)
            conductorWithDrover(app, new)
            opensTheWorkflowSheet(app)
            app.buttons["Done"].firstMatch.tap()
            app.buttons["Back"].firstMatch.tap()
            XCTAssert(new.waitForExistence(timeout: 10))
            XCTAssert(labelled(app, "Coder").exists)
            XCTAssertFalse(labelled(app, "cooee #33").exists)
            keep("home-no-lanes")
            othersHoldsTheChatsWithNoPersona(app, new)
        }
    }

    extension LiveUITestCase {
        @MainActor
        internal func conductorWithDrover(_ app: XCUIApplication, _ new: XCUIElement) {
            XCTAssert(new.waitForExistence(timeout: 10))
            new.tap()
            app.buttons["Conductor"].firstMatch.tap()
            guard !app.buttons["Workflow"].firstMatch.waitForExistence(timeout: 10) else { return }
            app.buttons["More"].firstMatch.tap()
            app.buttons["Attach drover…"].firstMatch.tap()
            app.buttons["Attach"].firstMatch.tap()
        }

        @MainActor
        internal func othersHoldsTheChatsWithNoPersona(_ app: XCUIApplication, _ new: XCUIElement) {
            let others = labelled(app, "Others")
            XCTAssert(others.waitForExistence(timeout: 10))
            XCTAssert(labelled(app, "Bonjour probe").exists)
            others.tap()
            XCTAssert(labelled(app, "CHATS · NO PERSONA").waitForExistence(timeout: 10))
            XCTAssert(labelled(app, "Bonjour probe").exists)
            XCTAssertFalse(labelled(app, "cooee #33").exists)
            keep("others")
            app.buttons["Back"].firstMatch.tap()
            XCTAssert(new.waitForExistence(timeout: 10))
        }

        @MainActor
        internal func opensTheWorkflowSheet(_ app: XCUIApplication) {
            let workflow = app.buttons["Workflow"].firstMatch
            XCTAssert(workflow.waitForExistence(timeout: 20))
            keep("conductor-workflow")
            workflow.tap()
            let issue = labelled(app, "cooee #33")
            XCTAssert(issue.waitForExistence(timeout: 20))
            issue.tap()
            let round = labelled(app, "open chat")
            XCTAssert(round.waitForExistence(timeout: 10))
            keep("workflow")
            round.tap()
            XCTAssert(labelled(app, "cooee #33 · impl 1").waitForExistence(timeout: 10))
            keep("workflow-round")
            app.buttons["Back"].firstMatch.tap()
            XCTAssert(labelled(app, "open chat").waitForExistence(timeout: 10))
        }
    }
#endif
