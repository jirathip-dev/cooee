#if os(iOS)
    import XCTest

    internal final class LiveUITests: XCTestCase {
        @MainActor
        internal func testPairsStreamsAndShowsHome() throws {
            let link = try XCTUnwrap(ProcessInfo.processInfo.environment["COOEE_PAIR_LINK"], "no COOEE_PAIR_LINK")
            let app = XCUIApplication()
            app.launch()
            pair(app, link)
            let new = app.buttons["New chat"].firstMatch
            XCTAssert(new.waitForExistence(timeout: 20))
            new.tap()
            app.buttons["Coder"].firstMatch.tap()
            let box = field(app, "Message Coder")
            XCTAssert(box.waitForExistence(timeout: 10))
            box.tap()
            box.typeText("slow 60")
            app.buttons["Send"].firstMatch.tap()
            queueWhileStreaming(app, box)
            keep("chat-streaming")
            app.buttons["Back"].firstMatch.tap()
            XCTAssert(new.waitForExistence(timeout: 10))
            XCTAssert(labelled(app, "Coder").exists)
            keep("home")
            personaRoundTrip(app)
            copiesAReply(app, new)
            answersEachCard(app, new)
            swipeBack(app, new)
            opensDroversScreens(app, new)
            avatarMenu(app, new)
        }

        @MainActor
        private func opensDroversScreens(_ app: XCUIApplication, _ new: XCUIElement) {
            XCTAssert(new.waitForExistence(timeout: 10))
            new.tap()
            app.buttons["Conductor"].firstMatch.tap()
            let queue = app.buttons["Queue"].firstMatch
            if !queue.waitForExistence(timeout: 10) {
                app.buttons["More"].firstMatch.tap()
                app.buttons["Attach drover…"].firstMatch.tap()
                XCTAssert(labelled(app, "Queue chip on the header strip").waitForExistence(timeout: 10))
                keep("attach")
                app.buttons["Attach"].firstMatch.tap()
            }
            XCTAssert(queue.waitForExistence(timeout: 20))
            keep("conductor")
            app.buttons["Events"].firstMatch.tap()
            XCTAssert(labelled(app, "merged").waitForExistence(timeout: 20))
            keep("events")
            app.buttons["Done"].firstMatch.tap()
            queue.tap()
            XCTAssert(labelled(app, "#722").waitForExistence(timeout: 30))
            keep("queue")
            XCTAssert(labelled(app, "review 1 ▸ herdsman-rev-722").waitForExistence(timeout: 30))
            keep("queue-live")
            app.buttons["Pause"].firstMatch.tap()
            XCTAssert(labelled(app, "actions are off").waitForExistence(timeout: 30))
            keep("queue-offline")
        }

        @MainActor
        private func answersEachCard(_ app: XCUIApplication, _ new: XCUIElement) {
            new.tap()
            app.buttons["Coder"].firstMatch.tap()
            let box = field(app, "Message Coder")
            XCTAssert(box.waitForExistence(timeout: 10))
            box.tap()
            box.typeText("Show me the cards")
            app.buttons["Send"].firstMatch.tap()
            XCTAssert(app.buttons["Approve"].waitForExistence(timeout: 30))
            keep("chat-cards")
            app.buttons["Allow once · recommended"].firstMatch.tap()
            XCTAssert(labelled(app, "Allow once ✓").waitForExistence(timeout: 10))
            app.buttons["apply"].firstMatch.tap()
            XCTAssert(labelled(app, "apply ✓").waitForExistence(timeout: 10))
            app.buttons["EDGAR_API_KEY"].firstMatch.tap()
            let secret = app.secureTextFields.firstMatch
            XCTAssert(secret.waitForExistence(timeout: 10))
            secret.typeText("k3y")
            app.buttons["OK"].firstMatch.tap()
            app.buttons["Send to plugin"].firstMatch.tap()
            XCTAssert(labelled(app, "EDGAR_API_KEY set ✓").waitForExistence(timeout: 10))
            app.buttons["Approve"].firstMatch.tap()
            app.buttons["Run"].firstMatch.tap()
            XCTAssert(labelled(app, "ran at 3b7e0c55a1d9: exit 0").waitForExistence(timeout: 10))
            keep("chat-cards-answered")
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

        @MainActor
        private func pair(_ app: XCUIApplication, _ link: String) {
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
        private func queueWhileStreaming(_ app: XCUIApplication, _ box: XCUIElement) {
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
        private func field(_ app: XCUIApplication, _ hint: String) -> XCUIElement {
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

    extension LiveUITests {
        @MainActor
        private func swipeBack(_ app: XCUIApplication, _ home: XCUIElement) {
            let edge = app.coordinate(withNormalizedOffset: CGVector(dx: 0.01, dy: 0.5))
            edge.press(forDuration: 0.1, thenDragTo: app.coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5)))
            XCTAssert(field(app, "Message Coder").waitForNonExistence(timeout: 10), "the edge swipe left the chat")
            XCTAssert(home.waitForExistence(timeout: 10), "the edge swipe went back to Home")
            keep("swipe-back")
        }

        @MainActor
        private func personaRoundTrip(_ app: XCUIApplication) {
            let name = "Scout \(ProcessInfo.processInfo.environment["COOEE_LOOK"] ?? "look")"
            app.buttons["New chat"].firstMatch.tap()
            app.buttons["New persona…"].firstMatch.tap()
            type(app, "Name", name)
            pin(app)
            type(app, "Character", "Finds the facts first.")
            app.buttons["Save"].firstMatch.tap()
            XCTAssert(labelled(app, name).waitForExistence(timeout: 10))
            app.buttons["New chat"].firstMatch.tap()
            app.buttons["Edit a persona"].firstMatch.tap()
            app.buttons["Edit \(name)"].firstMatch.tap()
            let character = NSPredicate(format: "value CONTAINS 'Finds the facts first.'")
            XCTAssert(app.descendants(matching: .any).matching(character).firstMatch.waitForExistence(timeout: 10))
            XCTAssertEqual(app.switches["Pin to Home"].firstMatch.value as? String, "1")
            type(app, "Instructions", "Cite every source.")
            keep("persona")
            app.buttons["Save"].firstMatch.tap()
            XCTAssert(app.buttons["Save"].waitForNonExistence(timeout: 10))
        }

        @MainActor
        private func copiesAReply(_ app: XCUIApplication, _ new: XCUIElement) {
            new.tap()
            app.buttons["Coder"].firstMatch.tap()
            type(app, "Message Coder", "paragraphs")
            app.buttons["Send"].firstMatch.tap()
            let reply = labelled(app, "HTML or SwiftUI?")
            XCTAssert(reply.waitForExistence(timeout: 30))
            keep("chat-paragraphs")
            reply.press(forDuration: 1.2)
            XCTAssert(app.buttons["Copy"].firstMatch.waitForExistence(timeout: 10))
            keep("message-menu")
            app.buttons["Select text"].firstMatch.tap()
            XCTAssert(app.textViews.firstMatch.waitForExistence(timeout: 10))
            keep("select-text")
            app.buttons["Done"].firstMatch.tap()
            app.buttons["Back"].firstMatch.tap()
            XCTAssert(new.waitForExistence(timeout: 10))
        }

        @MainActor
        private func pin(_ app: XCUIApplication) {
            let toggle = app.switches["Pin to Home"].firstMatch
            XCTAssert(toggle.waitForExistence(timeout: 10))
            let knob = toggle.switches.firstMatch
            (knob.exists ? knob : toggle).tap()
            XCTAssertEqual(toggle.value as? String, "1")
        }

        @MainActor
        private func avatarMenu(_ app: XCUIApplication, _ new: XCUIElement) {
            app.buttons["Done"].firstMatch.tap()
            app.buttons["Back"].firstMatch.tap()
            XCTAssert(new.waitForExistence(timeout: 10))
            app.buttons["Account"].firstMatch.tap()
            XCTAssert(labelled(app, "Notifications").waitForExistence(timeout: 10))
            keep("avatar-menu")
            labelled(app, "Look").tap()
            labelled(app, "Terminal").tap()
            confirmIcon()
            XCTAssert(picked(app, "Terminal").waitForExistence(timeout: 5))
            appIcon(app)
            labelled(app, "Modern").tap()
            confirmIcon()
            XCTAssert(picked(app, "Modern").waitForExistence(timeout: 5))
        }

        @MainActor
        internal func picked(_ app: XCUIApplication, _ label: String) -> XCUIElement {
            app.buttons.matching(NSPredicate(format: "label == %@ AND selected == true", label)).firstMatch
        }

        @MainActor
        private func type(_ app: XCUIApplication, _ hint: String, _ text: String) {
            let box = field(app, hint)
            XCTAssert(box.waitForExistence(timeout: 10))
            box.tap()
            box.typeText(text)
        }
    }
#endif
