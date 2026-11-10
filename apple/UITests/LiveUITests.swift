import XCTest

/// #143: the app in the simulator against a real `cooee serve` (apple/e2e/serve.sh): it pairs by the link typed as the
/// address, opens Home, starts a chat with Coder and streams its reply. Each screen is kept as the result's attachments
/// (`pair-paired-*`, `chat-streaming-*`, `home-*`) in the simulator's appearance, which CI sets with `simctl ui` and
/// names in `COOEE_LOOK` (dark or light): setting it from the test left the CI simulator light. `COOEE_PAIR_LINK` and
/// `COOEE_LOOK` reach the runner as `TEST_RUNNER_…`.
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
        let words = labelled(app, "w3 ")
        XCTAssert(words.waitForExistence(timeout: 30))
        XCTAssert(app.buttons["Stop"].exists)
        box.tap()
        box.typeText("Then open the PR")
        app.buttons["Send"].firstMatch.tap()
        XCTAssert(labelled(app, "Then open the PR").waitForExistence(timeout: 10))
        // A send lets go of the keyboard and leaves the field empty, not holding part of the sent text.
        let queue = field(app, "Queue a message")
        XCTAssert(queue.waitForExistence(timeout: 10))
        XCTAssert(app.keyboards.firstMatch.waitForNonExistence(timeout: 5))
        let left = queue.value as? String ?? ""
        XCTAssert(left.isEmpty || left == "Queue a message", "the field holds \(left)")
        keep("chat-streaming")

        app.buttons["Back"].firstMatch.tap()
        XCTAssert(new.waitForExistence(timeout: 10))
        XCTAssert(labelled(app, "Coder").exists)
        keep("home")
    }

    /// The text field or (one that grows) text view showing `hint`.
    func field(_ app: XCUIApplication, _ hint: String) -> XCUIElement {
        app.descendants(matching: .any).matching(NSPredicate(format: "(elementType == %d OR elementType == %d) AND (label == %@ OR placeholderValue == %@)",
                                                             XCUIElement.ElementType.textField.rawValue,
                                                             XCUIElement.ElementType.textView.rawValue, hint, hint)).firstMatch
    }

    /// Any element whose label holds `text`: a button's texts read as one label ("Coder, slow 60, …").
    func labelled(_ app: XCUIApplication, _ text: String) -> XCUIElement {
        app.descendants(matching: .any).matching(NSPredicate(format: "label CONTAINS %@", text)).firstMatch
    }

    /// The screen now, named for the simulator's appearance.
    @MainActor func keep(_ name: String) {
        let a = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        a.name = "\(name)-\(ProcessInfo.processInfo.environment["COOEE_LOOK"] ?? "look")"
        a.lifetime = .keepAlways
        add(a)
    }
}
