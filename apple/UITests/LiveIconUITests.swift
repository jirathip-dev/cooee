#if os(iOS)
    import XCTest

    extension LiveUITests {
        @MainActor
        internal func appIcon(_ app: XCUIApplication) {
            homeScreen(app, "home-icon-terminal")
            app.buttons.matching(NSPredicate(format: "label CONTAINS 'App icon'")).firstMatch.tap()
            XCTAssert(picked(app, "Follow the look").waitForExistence(timeout: 5))
            keep("app-icon-following")
            app.buttons["Koala"].firstMatch.tap()
            confirmIcon()
            XCTAssert(picked(app, "Koala").waitForExistence(timeout: 5))
            keep("app-icon-picked")
            app.navigationBars.buttons.element(boundBy: 0).tap()
            labelled(app, "Scene").tap()
            XCTAssert(picked(app, "Scene").waitForExistence(timeout: 5))
            homeScreen(app, "home-icon-koala-after-scene")
            app.buttons.matching(NSPredicate(format: "label CONTAINS 'App icon'")).firstMatch.tap()
            app.buttons["Follow the look"].firstMatch.tap()
            confirmIcon()
            XCTAssert(picked(app, "Follow the look").waitForExistence(timeout: 5))
            app.navigationBars.buttons.element(boundBy: 0).tap()
        }

        @MainActor
        internal var springboard: XCUIApplication { XCUIApplication(bundleIdentifier: "com.apple.springboard") }

        @MainActor
        internal func confirmIcon() {
            let done = springboard.alerts.buttons["OK"].firstMatch
            if done.waitForExistence(timeout: 5) { done.tap() }
        }

        @MainActor
        internal func homeScreen(_ app: XCUIApplication, _ name: String) {
            XCUIDevice.shared.press(.home)
            let icon = springboard.icons["cooee"]
            _ = icon.waitForExistence(timeout: 10)
            for _ in 0..<2 where !(icon.exists && icon.isHittable) { springboard.swipeLeft() }
            XCTAssert(icon.isHittable)
            keep(name)
            keep("\(name)-icon", icon.screenshot())
            app.activate()
        }
    }
#endif
