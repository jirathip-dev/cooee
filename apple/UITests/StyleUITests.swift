#if os(iOS)
    import XCTest

    extension LiveUITests {
        @MainActor
        internal func everyStyle(_ app: XCUIApplication, _ new: XCUIElement) {
            for style in ["Terminal", "Scene"] {
                labelled(app, style).tap()
                XCTAssert(picked(app, style).waitForExistence(timeout: 5))
                keep("look-\(style.lowercased())")
                closeMenu(app, picked(app, style))
                XCTAssert(new.waitForExistence(timeout: 10))
                keep("home-\(style.lowercased())")
                cardsIn(app, new, style)
                app.buttons["Account"].firstMatch.tap()
                labelled(app, "Look").tap()
            }
        }

        @MainActor
        private func closeMenu(_ app: XCUIApplication, _ look: XCUIElement) {
            let bottom = app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 1))
            for _ in 0..<3 where look.exists {
                let bar = app.navigationBars.firstMatch.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5))
                bar.press(forDuration: 0.1, thenDragTo: bottom, withVelocity: .fast, thenHoldForDuration: 0)
                _ = look.waitForNonExistence(timeout: 3)
            }
            XCTAssertFalse(look.exists, "the menu sheet is still open")
        }

        @MainActor
        private func cardsIn(_ app: XCUIApplication, _ new: XCUIElement, _ style: String) {
            new.tap()
            app.buttons["Coder"].firstMatch.tap()
            let box = field(app, "Message Coder")
            XCTAssert(box.waitForExistence(timeout: 10))
            box.tap()
            box.typeText("Show me the cards")
            app.buttons["Send"].firstMatch.tap()
            XCTAssert(app.buttons["Approve"].waitForExistence(timeout: 30))
            keep("chat-cards-\(style.lowercased())")
            app.buttons["Back"].firstMatch.tap()
            XCTAssert(new.waitForExistence(timeout: 10))
        }
    }
#endif
