#if os(iOS)
    import XCTest

    internal final class LiveUITests: LiveUITestCase {
        @MainActor
        internal func testPairsStreamsAndQueues() throws {
            let app = try launchHome()
            let box = send(app, "slow 60")
            queueWhileStreaming(app, box)
            keep("chat-streaming")
        }

        @MainActor
        internal func testShowsHome() throws {
            let app = try launchHome()
            send(app, "slow 3")
            app.buttons["Back"].firstMatch.tap()
            XCTAssert(newChat(app).waitForExistence(timeout: 10))
            XCTAssert(labelled(app, "Coder").exists)
            keep("home")
        }

        @MainActor
        internal func testAsksAndTheMacApprovesIt() throws {
            guard let server = ProcessInfo.processInfo.environment["COOEE_ASK_ADDRESS"] else {
                throw XCTSkip("CI's mac-setup job sets COOEE_ASK_ADDRESS, once the Mac shows its QR")
            }
            let app = XCUIApplication()
            app.launch()
            app.buttons["Enter address"].firstMatch.tap()
            let field = field(app, "server.example.com")
            XCTAssert(field.waitForExistence(timeout: 10))
            field.tap()
            field.typeText(server)
            app.buttons["Connect"].firstMatch.tap()
            XCTAssert(labelled(app, "Waiting").waitForExistence(timeout: 30))
            keep("pair-waiting")
            XCTAssert(app.staticTexts["Paired with 127.0.0.1"].waitForExistence(timeout: 600))
            keep("pair-approved")
        }
    }

    internal class LiveUITestCase: XCTestCase {
        @MainActor
        internal func launchHome() throws -> XCUIApplication {
            let link = try XCTUnwrap(ProcessInfo.processInfo.environment["COOEE_PAIR_LINK"], "no COOEE_PAIR_LINK")
            let app = XCUIApplication()
            let modern = Data(#"{"scheme":"system","style":"modern"}"#.utf8).map { String(format: "%02x", $0) }
            app.launchArguments = ["-cooee.settings", "<\(modern.joined())>"]
            app.launch()
            let start = app.buttons.matching(NSPredicate(format: "label IN %@", ["New chat", "Enter address"]))
                .firstMatch
            XCTAssert(start.waitForExistence(timeout: 20))
            if start.label == "Enter address" {
                pair(app, link)
                XCTAssert(newChat(app).waitForExistence(timeout: 20))
            }
            return app
        }

        @MainActor
        internal func newChat(_ app: XCUIApplication) -> XCUIElement { app.buttons["New chat"].firstMatch }

        @MainActor
        @discardableResult
        internal func send(_ app: XCUIApplication, _ text: String) -> XCUIElement {
            newChat(app).tap()
            app.buttons["Coder"].firstMatch.tap()
            type(app, "Message Coder", text)
            app.buttons["Send"].firstMatch.tap()
            return field(app, "Message Coder")
        }

        @MainActor
        internal func pair(_ app: XCUIApplication, _ link: String) {
            app.buttons["Enter address"].firstMatch.tap()
            let address = field(app, "server.example.com")
            XCTAssert(address.waitForExistence(timeout: 10))
            address.tap()
            address.typeText(link)
            app.buttons["Connect"].firstMatch.tap()
            XCTAssert(app.staticTexts["Paired with 127.0.0.1"].waitForExistence(timeout: 60))
            keep("pair-paired")
            app.buttons["Go to Home"].firstMatch.tap()
        }

        @MainActor
        internal func queueWhileStreaming(_ app: XCUIApplication, _ box: XCUIElement) {
            XCTAssert(labelled(app, "w3 ").waitForExistence(timeout: 30))
            XCTAssert(app.buttons["Stop"].exists)
            box.tap()
            box.typeText("Then open the PR")
            app.buttons["Send"].firstMatch.tap()
            XCTAssert(labelled(app, "Then open the PR").waitForExistence(timeout: 10))
            let queue = field(app, "Queue a message")
            XCTAssert(queue.waitForExistence(timeout: 10))
            XCTAssert(app.keyboards.firstMatch.waitForNonExistence(timeout: 5))
            let left = queue.value as? String ?? ""
            XCTAssert(left.isEmpty || left == "Queue a message", "the field holds \(left)")
        }

        @MainActor
        internal func field(_ app: XCUIApplication, _ hint: String) -> XCUIElement {
            let types = [XCUIElement.ElementType.textField.rawValue, XCUIElement.ElementType.textView.rawValue]
            let predicate = NSPredicate(
                format: "(elementType == %d OR elementType == %d) AND (label == %@ OR placeholderValue == %@)",
                types[0], types[1], hint, hint
            )
            return app.descendants(matching: .any).matching(predicate).firstMatch
        }

        @MainActor
        internal func labelled(_ app: XCUIApplication, _ text: String) -> XCUIElement {
            app.descendants(matching: .any).matching(NSPredicate(format: "label CONTAINS %@", text)).firstMatch
        }

        @MainActor
        internal func keep(_ name: String, _ screenshot: XCUIScreenshot = XCUIScreen.main.screenshot()) {
            let shot = XCTAttachment(screenshot: screenshot)
            shot.name = "\(name)-\(ProcessInfo.processInfo.environment["COOEE_LOOK"] ?? "look")"
            shot.lifetime = .keepAlways
            add(shot)
        }
    }

    extension LiveUITestCase {
        @MainActor
        internal func pin(_ app: XCUIApplication) {
            let toggle = app.switches["Pin to Home"].firstMatch
            XCTAssert(toggle.waitForExistence(timeout: 10))
            let knob = toggle.switches.firstMatch
            (knob.exists ? knob : toggle).tap()
            XCTAssertEqual(toggle.value as? String, "1")
        }

        @MainActor
        internal func picked(_ app: XCUIApplication, _ label: String) -> XCUIElement {
            app.buttons.matching(NSPredicate(format: "label == %@ AND selected == true", label)).firstMatch
        }

        @MainActor
        internal func type(_ app: XCUIApplication, _ hint: String, _ text: String) {
            let box = field(app, hint)
            XCTAssert(box.waitForExistence(timeout: 10))
            box.tap()
            box.typeText(text)
        }
    }
#endif
