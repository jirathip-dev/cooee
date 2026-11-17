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
        internal func testAsksAndTheCLIApprovesIt() throws {
            guard let server = ProcessInfo.processInfo.environment["COOEE_ASK_ADDRESS"] else {
                throw XCTSkip("apple/e2e/steps/20-app.sh sets COOEE_ASK_ADDRESS and approves with `cooee pair approve`")
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
            XCTAssert(app.staticTexts["Paired with 127.0.0.1"].waitForExistence(timeout: 60))
            keep("pair-approved")
        }
    }
#endif
