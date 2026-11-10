#if os(macOS)
    import XCTest

    internal final class MacSetupUITests: XCTestCase {
        @MainActor
        internal func testSetsUpThisMacAndApprovesAPhone() throws {
            let enabled = ProcessInfo.processInfo.environment["COOEE_MAC_SETUP"] == "1"
            try XCTSkipUnless(enabled, "starts login items on this Mac: CI's mac-setup job sets COOEE_MAC_SETUP=1")
            let app = XCUIApplication()
            app.launch()
            let setUp = app.buttons["Set up this Mac"]
            XCTAssert(setUp.waitForExistence(timeout: 30))
            keep(app, "welcome")
            setUp.click()
            XCTAssert(labelled(app, "cooee on · shed on").waitForExistence(timeout: 60), "the login items run")
            XCTAssert(labelled(app, "Claude Code").waitForExistence(timeout: 90), "the agents are listed")
            keep(app, "folders")
            next(app, "Each agent signs in on this Mac", "signins")
            next(app, "Reach it from anywhere", "tailscale")
            XCTAssert(labelled(app, "Tailscale").exists)
            app.buttons["Wi-Fi only for now"].click()
            XCTAssert(labelled(app, "Waiting for a phone").waitForExistence(timeout: 30))
            XCTAssert(labelled(app, "or enter ").exists)
            keep(app, "qr")
            let approve = app.buttons["Approve · Touch ID"]
            XCTAssert(approve.waitForExistence(timeout: 900), "a phone asks to pair")
            XCTAssert(labelled(app, "wants to pair").exists)
            keep(app, "approve")
            approve.click()
            XCTAssert(labelled(app, "1 approved").waitForExistence(timeout: 30))
            keep(app, "approved")
        }

        @MainActor
        private func next(_ app: XCUIApplication, _ text: String, _ name: String) {
            app.buttons["Continue"].firstMatch.click()
            XCTAssert(labelled(app, text).waitForExistence(timeout: 30), "\(name) shows")
            keep(app, name)
        }

        @MainActor
        private func labelled(_ app: XCUIApplication, _ text: String) -> XCUIElement {
            let predicate = NSPredicate(format: "label CONTAINS %@ OR value CONTAINS %@", text, text)
            return app.staticTexts.matching(predicate).firstMatch
        }

        @MainActor
        private func keep(_ app: XCUIApplication, _ step: String) {
            let shot = XCTAttachment(screenshot: app.windows.firstMatch.screenshot())
            shot.name = "mac-setup-\(step)"
            shot.lifetime = .keepAlways
            add(shot)
        }
    }
#endif
