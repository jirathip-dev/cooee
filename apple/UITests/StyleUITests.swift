#if os(iOS)
    import XCTest

    extension LiveUITests {
        @MainActor
        internal func everyStyle(_ app: XCUIApplication, _ new: XCUIElement) {
            for style in ["Terminal", "Scene"] {
                labelled(app, style).tap()
                XCTAssert(picked(app, style).waitForExistence(timeout: 5))
                keep("look-\(style.lowercased())")
                app.navigationBars.firstMatch.swipeDown(velocity: .fast)
                XCTAssert(new.waitForExistence(timeout: 10))
                keep("home-\(style.lowercased())")
                cardsIn(app, new, style)
                app.buttons["Account"].firstMatch.tap()
                labelled(app, "Look").tap()
            }
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
