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
        internal func personaPanel(_ app: XCUIApplication) {
            keep("conductor")
            app.buttons["More"].firstMatch.tap()
            XCTAssert(app.buttons["Edit Conductor"].waitForExistence(timeout: 10), "the … opened no Edit Conductor")
            XCTAssert(app.buttons["New topic with Conductor"].exists, "the … opened no New topic with Conductor")
            XCTAssertFalse(labelled(app, "Attach ").exists, "Conductor still has a plugin to attach")
            XCTAssert(labelled(app, "WORKFLOW").exists, "the panel has no Workflow for Conductor")
            XCTAssert(app.buttons["Add routine"].exists, "the panel has no Routines + Add for Conductor")
            keep("persona-panel")
            app.buttons["Close"].firstMatch.tap()
            XCTAssert(app.buttons["Edit Conductor"].waitForNonExistence(timeout: 10))
        }

        @MainActor
        internal func droverChat(_ app: XCUIApplication, _ new: XCUIElement) -> XCUIElement {
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
            return queue
        }

        @MainActor
        internal func opensDroversScreens(_ app: XCUIApplication, _ new: XCUIElement) {
            let queue = droverChat(app, new)
            personaPanel(app)
            app.buttons["Events"].firstMatch.tap()
            XCTAssert(labelled(app, "merged").waitForExistence(timeout: 20))
            keep("events")
            app.buttons["Done"].firstMatch.tap()
            queue.tap()
            XCTAssert(labelled(app, "#722").waitForExistence(timeout: 30))
            keep("queue")
            followsTheQueueAndFoldsARowOpen(app)
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
            let line = app.buttons["4 need you"].firstMatch
            XCTAssert(line.waitForExistence(timeout: 30), "4 open asks show no one chat line")
            XCTAssert(app.buttons["Asks 4"].exists, "4 open asks show no Asks 4 chip")
            XCTAssertFalse(app.buttons["Approve"].exists, "an ask card renders inline in the thread")
            keep("chat-cards")
            line.tap()
            answersInTheSheet(app)
            app.buttons["Done"].firstMatch.tap()
            XCTAssert(app.buttons["1 need you"].waitForExistence(timeout: 10), "the chat line did not update in place")
            XCTAssert(app.buttons["Asks 1"].exists, "the Asks chip did not count down to 1")
            XCTAssertFalse(line.exists, "the chat line still says 4")
            for answered in ["Allow once ✓", "apply ✓", "ran at 3b7e0c55a1d9: exit 0"] {
                XCTAssert(labelled(app, answered).waitForExistence(timeout: 10), "the thread lacks \(answered)")
            }
            keep("chat-cards-answered")
        }

        @MainActor
        internal func answersInTheSheet(_ app: XCUIApplication) {
            XCTAssert(app.buttons["Allow once · recommended"].waitForExistence(timeout: 10), "the sheet holds no ask")
            keep("asks-sheet")
            app.buttons["Allow once · recommended"].firstMatch.tap()
            XCTAssert(labelled(app, "Asks 3").waitForExistence(timeout: 10), "answering one left Asks at 4")
            app.buttons["apply"].firstMatch.tap()
            XCTAssert(labelled(app, "Asks 2").waitForExistence(timeout: 10), "answering one left Asks at 3")
            app.buttons["Approve"].firstMatch.tap()
            app.buttons["Run"].firstMatch.tap()
            XCTAssert(labelled(app, "Asks 1").waitForExistence(timeout: 10), "answering one left Asks at 2")
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
            for row in ["Sign-ins", "Devices", "Machines", "Plugins"] {
                let button = app.buttons.matching(NSPredicate(format: "label CONTAINS %@", row)).firstMatch
                XCTAssert(button.waitForExistence(timeout: 10), "the menu lists \(row)")
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
            app.navigationBars.buttons.firstMatch.tap()
            labelled(app, "Terminal").tap()
            XCTAssert(labelled(app, "Terminal ·").waitForExistence(timeout: 5))
            keep("avatar-menu-terminal")
        }
    }
#endif
