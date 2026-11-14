#if os(iOS)
    import XCTest

    internal final class LiveChatUITests: LiveUITestCase {
        @MainActor
        internal func testAnswersEachCard() throws {
            let app = try launchHome()
            answersEachCard(app, newChat(app))
            swipeBack(app, newChat(app))
        }

        @MainActor
        internal func testCopiesAReply() throws {
            let app = try launchHome()
            copiesAReply(app, newChat(app))
        }
    }

    internal final class LiveDroverUITests: LiveUITestCase {
        @MainActor
        internal func testOpensDroversScreens() throws {
            let app = try launchHome()
            opensDroversScreens(app, newChat(app))
        }
    }

    extension LiveUITestCase {
        @MainActor
        internal func opensDroversScreens(_ app: XCUIApplication, _ new: XCUIElement) {
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
        internal func answersEachCard(_ app: XCUIApplication, _ new: XCUIElement) {
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
        internal func swipeBack(_ app: XCUIApplication, _ home: XCUIElement) {
            let edge = app.coordinate(withNormalizedOffset: CGVector(dx: 0.01, dy: 0.5))
            edge.press(forDuration: 0.1, thenDragTo: app.coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5)))
            XCTAssert(field(app, "Message Coder").waitForNonExistence(timeout: 10), "the edge swipe left the chat")
            XCTAssert(home.waitForExistence(timeout: 10), "the edge swipe went back to Home")
            keep("swipe-back")
        }

        @MainActor
        internal func copiesAReply(_ app: XCUIApplication, _ new: XCUIElement) {
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
        internal func avatarMenu(_ app: XCUIApplication) {
            app.buttons["Account"].firstMatch.tap()
            XCTAssert(labelled(app, "Notifications").waitForExistence(timeout: 10))
            for missing in ["Sign-ins", "Devices", "Machines", "Plugins"] {
                XCTAssertFalse(
                    app.buttons.matching(NSPredicate(format: "label CONTAINS %@", missing)).firstMatch.exists)
            }
            keep("avatar-menu")
            labelled(app, "Look").tap()
            pickStyle(app, "Terminal")
            pickStyle(app, "Modern")
            keep("avatar-menu-look")
            app.navigationBars.buttons.firstMatch.tap()
            labelled(app, "Notifications").tap()
            XCTAssert(app.switches["Push notifications"].waitForExistence(timeout: 5))
            keep("avatar-menu-notifications")
        }
    }
#endif
