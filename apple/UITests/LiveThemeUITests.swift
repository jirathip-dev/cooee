#if os(iOS)
    import XCTest

    internal final class LiveThemeUITests: LiveUITestCase {
        @MainActor
        internal func testEveryTheme() throws {
            let app = try launchHome()
            themes(app, newChat(app))
        }
    }

    extension LiveUITestCase {
        @MainActor
        internal func themes(_ app: XCUIApplication, _ new: XCUIElement) {
            let idles = [
                ("Kitchen", "standing by"), ("Garden", "resting"), ("Bush", "having a kip"), ("Farm", "grazing"),
            ]
            for (theme, idle) in idles {
                look(app, theme, "Modern")
                keep("theme-\(theme.lowercased())-modern")
                new.tap()
                app.buttons["Coder"].firstMatch.tap()
                XCTAssert(field(app, "Message Coder").waitForExistence(timeout: 10))
                keep("theme-\(theme.lowercased())-chat")
                app.buttons["Back"].firstMatch.tap()
                XCTAssert(new.waitForExistence(timeout: 10))
                look(app, theme, "Scene")
                XCTAssert(app.staticTexts[idle].firstMatch.waitForExistence(timeout: 10))
                keep("theme-\(theme.lowercased())-scene")
            }
            look(app, "Farm", "Modern")
        }

        @MainActor
        private func look(_ app: XCUIApplication, _ theme: String, _ style: String) {
            app.buttons["Account"].firstMatch.tap()
            labelled(app, "Look").tap()
            XCTAssert(app.buttons[theme].firstMatch.waitForExistence(timeout: 10))
            app.buttons[theme].firstMatch.tap()
            XCTAssert(picked(app, theme).waitForExistence(timeout: 5))
            app.buttons[style].firstMatch.tap()
            confirmIcon()
            XCTAssert(picked(app, style).waitForExistence(timeout: 5))
            closeSheet(app, "Look")
            sleep(2)
        }

        @MainActor
        private func closeSheet(_ app: XCUIApplication, _ title: String) {
            let bar = app.navigationBars[title].firstMatch
            XCTAssert(bar.waitForExistence(timeout: 5))
            bar.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).press(
                forDuration: 0.1, thenDragTo: app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.98)))
            XCTAssert(bar.waitForNonExistence(timeout: 10))
        }
    }
#endif
