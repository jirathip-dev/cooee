#if os(iOS)
    import XCTest

    internal final class LivePersonaUITests: LiveUITestCase {
        @MainActor
        internal func testPersonaRoundTrip() throws { personaRoundTrip(try launchHome()) }
    }

    extension LiveUITestCase {
        @MainActor
        internal func personaRoundTrip(_ app: XCUIApplication) {
            let name = "Scout \(ProcessInfo.processInfo.environment["COOEE_LOOK"] ?? "look")"
            app.buttons["New chat"].firstMatch.tap()
            app.buttons["New persona…"].firstMatch.tap()
            type(app, "Name", name)
            pin(app)
            type(app, "Character", "Finds the facts first.")
            app.buttons["Save"].firstMatch.tap()
            XCTAssert(app.buttons["Save"].waitForNonExistence(timeout: 10))
            XCTAssert(labelled(app, name).waitForExistence(timeout: 10))
            app.buttons["New chat"].firstMatch.tap()
            app.buttons["Edit a persona"].firstMatch.tap()
            app.buttons["Edit \(name)"].firstMatch.tap()
            let character = NSPredicate(format: "value CONTAINS 'Finds the facts first.'")
            XCTAssert(app.descendants(matching: .any).matching(character).firstMatch.waitForExistence(timeout: 10))
            XCTAssertEqual(app.switches["Pin to Home"].firstMatch.value as? String, "1")
            picksAgentAndFolder(app)
            type(app, "Instructions", "Cite every source.")
            keep("persona")
            app.buttons["Save"].firstMatch.tap()
            XCTAssert(app.buttons["Save"].waitForNonExistence(timeout: 10))
        }

        @MainActor
        internal func picksAgentAndFolder(_ app: XCUIApplication) {
            app.buttons["Agent"].firstMatch.tap()
            let agent = app.buttons["fake-acp"].firstMatch
            XCTAssert(agent.waitForExistence(timeout: 10))
            keep("persona-menu")
            agent.tap()
            app.buttons["Folder"].firstMatch.tap()
            let root = app.buttons["root"].firstMatch
            XCTAssert(root.waitForExistence(timeout: 10))
            keep("persona-folders")
            root.tap()
            let project = app.buttons["project"].firstMatch
            XCTAssert(project.waitForExistence(timeout: 10))
            project.tap()
            let use = app.buttons["Use project"].firstMatch
            XCTAssert(use.waitForExistence(timeout: 10))
            use.tap()
            XCTAssert(labelled(app, "/root/project").waitForExistence(timeout: 10))
        }

        @MainActor
        internal func pin(_ app: XCUIApplication) {
            let toggle = app.switches["Pin to Home"].firstMatch
            XCTAssert(toggle.waitForExistence(timeout: 10))
            let knob = toggle.switches.firstMatch
            (knob.exists ? knob : toggle).tap()
            XCTAssertEqual(toggle.value as? String, "1")
        }
    }
#endif
