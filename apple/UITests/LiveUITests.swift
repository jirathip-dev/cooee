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
        avatarMenu(app)
    }

    @MainActor
    private func personaRoundTrip(_ app: XCUIApplication) {
        let name = "Scout \(ProcessInfo.processInfo.environment["COOEE_LOOK"] ?? "look")"
        app.buttons["New chat"].firstMatch.tap()
        app.buttons["New persona…"].firstMatch.tap()
        type(app, "Name", name)
        type(app, "Character", "Finds the facts first.")
        app.buttons["Save"].firstMatch.tap()
        XCTAssert(labelled(app, name).waitForExistence(timeout: 10))
        app.buttons["New chat"].firstMatch.tap()
        app.buttons["Edit a persona"].firstMatch.tap()
        app.buttons["Edit \(name)"].firstMatch.tap()
        XCTAssert(labelled(app, "Finds the facts first.").waitForExistence(timeout: 10))
        type(app, "Instructions", "Cite every source.")
        keep("persona")
        app.buttons["Save"].firstMatch.tap()
        XCTAssert(app.buttons["Save"].waitForNonExistence(timeout: 10))
    }

    @MainActor
    private func avatarMenu(_ app: XCUIApplication) {
        app.buttons["Account"].firstMatch.tap()
        XCTAssert(labelled(app, "Notifications").waitForExistence(timeout: 10))
        keep("avatar-menu")
        labelled(app, "Look").tap()
        labelled(app, "Terminal").tap()
        let picked = NSPredicate(format: "label == 'Terminal' AND selected == true")
        XCTAssert(app.buttons.matching(picked).firstMatch.waitForExistence(timeout: 5))
    }

    @MainActor
    private func type(_ app: XCUIApplication, _ hint: String, _ text: String) {
        let box = field(app, hint)
        XCTAssert(box.waitForExistence(timeout: 10))
        box.tap()
        box.typeText(text)
    }

    @MainActor
    private func pair(_ app: XCUIApplication, _ link: String) {
        app.buttons["Enter address"].firstMatch.tap()
        let address = field(app, "server.example.com")
        XCTAssert(address.waitForExistence(timeout: 10))
        address.tap()
        address.typeText(link)
        app.buttons["Connect"].firstMatch.tap()
        XCTAssert(app.staticTexts["Paired with 127.0.0.1"].waitForExistence(timeout: 20))
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
    private func labelled(_ app: XCUIApplication, _ text: String) -> XCUIElement {
        app.descendants(matching: .any).matching(NSPredicate(format: "label CONTAINS %@", text)).firstMatch
    }

    @MainActor
    private func keep(_ name: String) {
        let shot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        shot.name = "\(name)-\(ProcessInfo.processInfo.environment["COOEE_LOOK"] ?? "look")"
        shot.lifetime = .keepAlways
        add(shot)
    }
}
