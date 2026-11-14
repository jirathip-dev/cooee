#if os(iOS)
    import XCTest

    internal final class LiveIconUITests: LiveUITestCase {
        @MainActor
        internal func testAppIcon() throws {
            let app = try launchHome()
            app.buttons["Account"].firstMatch.tap()
            labelled(app, "Look").tap()
            pickStyle(app, "Terminal")
            appIcon(app)
            pickStyle(app, "Modern")
        }

        @MainActor
        internal func testAvatarMenu() throws { avatarMenu(try launchHome()) }
    }

    extension LiveUITestCase {
        @MainActor
        internal func pickStyle(_ app: XCUIApplication, _ style: String) {
            labelled(app, style).tap()
            confirmIcon()
            XCTAssert(picked(app, style).waitForExistence(timeout: 5))
        }

        @MainActor
        internal func appIcon(_ app: XCUIApplication) {
            homeScreen(app, "home-icon-terminal")
            openIconPicker(app)
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
            openIconPicker(app)
            app.buttons["Follow the look"].firstMatch.tap()
            confirmIcon()
            XCTAssert(picked(app, "Follow the look").waitForExistence(timeout: 5))
            app.navigationBars.buttons.element(boundBy: 0).tap()
        }

        @MainActor
        private func openIconPicker(_ app: XCUIApplication) {
            labelled(app, "Modern").swipeUp()
            let koala = app.buttons["Koala"].firstMatch
            for _ in 0..<3 where !koala.exists {
                app.buttons.matching(NSPredicate(format: "label CONTAINS 'App icon'")).firstMatch.tap()
                _ = koala.waitForExistence(timeout: 3)
            }
            XCTAssert(koala.exists, "the icon picker opens")
        }

        @MainActor
        internal var springboard: XCUIApplication { XCUIApplication(bundleIdentifier: "com.apple.springboard") }

        @MainActor
        internal func confirmIcon() {
            let done = springboard.alerts.buttons["OK"].firstMatch
            guard done.waitForExistence(timeout: 5) else { return }
            for _ in 0..<3 where done.exists {
                done.tap()
                _ = done.waitForNonExistence(timeout: 2)
            }
            XCTAssertFalse(done.exists, "SpringBoard's icon alert closed")
        }

        @MainActor
        internal func homeScreen(_ app: XCUIApplication, _ name: String) {
            confirmIcon()
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
