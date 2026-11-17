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
            XCTAssertEqual(paired(app, timeout: 600), "Paired with 127.0.0.1")
            keep("pair-approved")
        }

        @MainActor
        internal func testStaysPairedAfterTheMacUpdates() throws {
            guard ProcessInfo.processInfo.environment["COOEE_AFTER_UPDATE"] == "1" else {
                throw XCTSkip("CI's mac-setup job sets COOEE_AFTER_UPDATE, once the Mac has updated itself")
            }
            let app = XCUIApplication()
            app.launch()
            XCTAssert(newChat(app).waitForExistence(timeout: 30), "still paired: home, not the pairing screen")
            newChat(app).tap()
            XCTAssert(app.buttons["Coder"].firstMatch.waitForExistence(timeout: 30), "the updated server answers")
            keep("after-mac-update")
        }
    }
#endif
