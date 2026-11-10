import XCTest

/// #143: the app in the simulator against a real `cooee serve` (apple/e2e/serve.sh): it pairs by the link typed as the
/// address, opens Home, starts a chat with Coder and streams its reply. Each screen is kept, dark and light, as the
/// result's attachments (`pair-paired-*`, `chat-streaming-*`, `home-*`). `COOEE_PAIR_LINK` reaches the runner as
/// `TEST_RUNNER_COOEE_PAIR_LINK`; without it the test is skipped.
final class LiveUITests: XCTestCase {
    @MainActor func testPairsStreamsAndShowsHome() throws {
        let link = try XCTUnwrap(ProcessInfo.processInfo.environment["COOEE_PAIR_LINK"], "no COOEE_PAIR_LINK")
        let app = XCUIApplication()
        app.launch()

        app.buttons["Enter address"].firstMatch.tap()
        let address = field(app, "server.example.com")
        XCTAssert(address.waitForExistence(timeout: 10))
        address.tap()
        address.typeText(link)
        app.buttons["Connect"].firstMatch.tap()
        XCTAssert(app.staticTexts["Paired with 127.0.0.1"].waitForExistence(timeout: 20))
        keep("pair-paired")
        app.buttons["Go to Home"].firstMatch.tap()

        let new = app.buttons["New chat"].firstMatch
        XCTAssert(new.waitForExistence(timeout: 20))
        new.tap()
        app.buttons["Coder"].firstMatch.tap()
        let box = field(app, "Message Coder")
        XCTAssert(box.waitForExistence(timeout: 10))
        box.tap()
        box.typeText("slow 60")
        app.buttons["Send"].firstMatch.tap()

        // The reply streams in a word at a time; a message typed meanwhile waits in the queue.
        let words = app.staticTexts.containing(NSPredicate(format: "label CONTAINS 'w3 '")).firstMatch
        XCTAssert(words.waitForExistence(timeout: 30))
        XCTAssert(app.buttons["Stop"].exists)
        let queue = field(app, "Queue a message")
        XCTAssert(queue.waitForExistence(timeout: 10))
        queue.tap()
        queue.typeText("Then open the PR")
        app.buttons["Send"].firstMatch.tap()
        XCTAssert(app.staticTexts["Then open the PR"].waitForExistence(timeout: 10))
        app.scrollViews.firstMatch.swipeDown()  // the keyboard away
        keep("chat-streaming")

        app.buttons["Back"].firstMatch.tap()
        XCTAssert(app.staticTexts["Coder"].firstMatch.waitForExistence(timeout: 10))
        keep("home")
    }

    /// The text field or (one that grows) text view showing `hint`.
    func field(_ app: XCUIApplication, _ hint: String) -> XCUIElement {
        app.descendants(matching: .any).matching(NSPredicate(format: "(elementType == %d OR elementType == %d) AND (label == %@ OR placeholderValue == %@)",
                                                             XCUIElement.ElementType.textField.rawValue,
                                                             XCUIElement.ElementType.textView.rawValue, hint, hint)).firstMatch
    }

    /// The screen now, dark then light.
    @MainActor func keep(_ name: String) {
        for (mode, look) in [("dark", XCUIDevice.Appearance.dark), ("light", .light)] {
            XCUIDevice.shared.appearance = look
            sleep(1)
            let a = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
            a.name = "\(name)-\(mode)"
            a.lifetime = .keepAlways
            add(a)
        }
    }
}
