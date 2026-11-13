#if os(iOS)
    import XCTest

    extension LiveUITests {
        @MainActor
        internal func chatStyles(_ app: XCUIApplication, _ new: XCUIElement) {
            commands(app, new, "Coder")
            let fold = app.buttons.matching(NSPredicate(format: "label BEGINSWITH 'Worked'")).firstMatch
            XCTAssert(fold.waitForExistence(timeout: 10))
            XCTAssertFalse(labelled(app, "$ cargo fmt").exists)
            keep("chat-style-chat")
            fold.tap()
            for step in ["$ cargo fmt", "$ cargo clippy", "$ cargo test"] {
                XCTAssert(labelled(app, step).waitForExistence(timeout: 10))
            }
            keep("trace")
            app.buttons["Close"].firstMatch.tap()
            app.buttons["Back"].firstMatch.tap()
            commands(app, new, "Scout \(ProcessInfo.processInfo.environment["COOEE_LOOK"] ?? "look")")
            XCTAssert(labelled(app, "$ cargo test").exists)
            XCTAssertFalse(fold.exists)
            keep("chat-style-detailed")
            app.buttons["Back"].firstMatch.tap()
        }

        @MainActor
        internal func commands(_ app: XCUIApplication, _ new: XCUIElement, _ persona: String) {
            open(app, new, persona).typeText("commands")
            app.buttons["Send"].firstMatch.tap()
            XCTAssert(labelled(app, "Ran 3 commands.").waitForExistence(timeout: 30))
        }

        @MainActor
        internal func open(_ app: XCUIApplication, _ new: XCUIElement, _ persona: String) -> XCUIElement {
            XCTAssert(new.waitForExistence(timeout: 10))
            new.tap()
            app.buttons[persona].firstMatch.tap()
            let box = field(app, "Message \(persona)")
            XCTAssert(box.waitForExistence(timeout: 10))
            box.tap()
            return box
        }
    }
#endif
