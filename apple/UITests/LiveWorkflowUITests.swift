#if os(iOS)
    import XCTest

    extension LiveUITests {
        @MainActor
        internal func othersHoldsTheLanesNoPluginClaims(_ app: XCUIApplication, _ new: XCUIElement) {
            XCTAssert(new.waitForExistence(timeout: 10))
            XCTAssert(labelled(app, "Coder").exists)
            keep("home")
            let others = labelled(app, "Others")
            guard others.waitForExistence(timeout: 10) else { return }
            others.tap()
            XCTAssert(labelled(app, "cooee #33").waitForExistence(timeout: 10))
            keep("others")
            app.buttons["Back"].firstMatch.tap()
            XCTAssert(new.waitForExistence(timeout: 10))
        }

        @MainActor
        internal func opensTheWorkflowSheet(_ app: XCUIApplication, _ new: XCUIElement) {
            app.buttons["Done"].firstMatch.tap()
            let workflow = app.buttons["Workflow"].firstMatch
            XCTAssert(workflow.waitForExistence(timeout: 20))
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
