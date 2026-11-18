@testable import CooeeAPI
import CooeeCore
import Foundation
import Testing

internal struct FrameClientTests {
    @Test internal func aReplayFrameIsItsBytesWithTheCookie() async throws {
        let server = CannedServer(["/threads/a1b2/frames/f1": [Answer(chunks: ["pixels"])]])
        #expect(try await server.client().frame("a1b2", "f1") == Data("pixels".utf8))
        #expect(server.requests("/threads/a1b2/frames/f1").first?.value(forHTTPHeaderField: "Cookie") == "cooee=tok")
    }
}

internal struct ScreenClientTests {
    @Test internal func theLeaseIsTheOwnersUntilItsEndAndTheAgentsOtherwise() async throws {
        let owner = "{\"control\":{\"by\":\"owner\",\"until\":1700000600000},\"waiting\":1}"
        let server = CannedServer([
            "/personas/ann/screen": [
                Answer(headers: json, chunks: ["{\"control\":{\"by\":\"back\",\"note\":\"x\"},\"waiting\":0}"])
            ],
            "/personas/ann/screen/control": [Answer(headers: json, chunks: [owner])],
            "/personas/ann/screen/back": [
                Answer(headers: json, chunks: ["{\"control\":{\"by\":\"agent\"},\"waiting\":0}"])
            ],
        ])
        let client = server.client()
        #expect(try await client.screen("ann") == BrowserScreen())
        let held = try await client.takeControl("ann")
        #expect(held == BrowserScreen(until: Date(timeIntervalSince1970: 1_700_000_600), waiting: 1))
        #expect(held.left(at: Date(timeIntervalSince1970: 1_700_000_000.5)) == 600)
        #expect(held.left(at: Date(timeIntervalSince1970: 1_700_000_600)) == nil)
        #expect(try await client.handBack("ann", note: "signed in") == BrowserScreen())
        #expect(server.body("/personas/ann/screen/back") == "{\"note\":\"signed in\"}")
    }

    @Test internal func aTapOnTheFitPageIsItsCSSPoint() {
        let page = CGSize(width: 1280, height: 800)
        let point = LiveFrame.page(CGPoint(x: 195, y: 61), view: CGSize(width: 390, height: 244), page: page)
        #expect(point.x == 640 && point.y == 200)
        let none = LiveFrame.page(CGPoint(x: 5, y: 5), view: CGSize(width: 0, height: 0), page: page)
        #expect(none.x == 0 && none.y == 0)
    }

    @Test internal func aTapAndTextGoAsTheServersInput() async throws {
        let server = CannedServer(["/personas/ann/screen/input": [Answer(chunks: [])]])
        try await server.client().input("ann", .click(left: 10, top: 20.5))
        let click = try server.decoded("/personas/ann/screen/input", as: [String: [String: Double]].self)
        #expect(click == ["click": ["x": 10, "y": 20.5]])
        let other = CannedServer(["/personas/ann/screen/input": [Answer(chunks: [])]])
        try await other.client().input("ann", .text("hunter2"))
        #expect(other.body("/personas/ann/screen/input") == "{\"text\":\"hunter2\"}")
    }

    @Test internal func handingBackAThreadPostsTheNoteOnce() async throws {
        let server = CannedServer(["/threads/a1/handback": [Answer(headers: json, chunks: ["{}"])]])
        try await server.client().resumeHandoff("a1", note: "signed in", randomID: "r1")
        let body = try server.decoded("/threads/a1/handback", as: [String: String].self)
        #expect(body == ["text": "signed in", "random_id": "r1"])
    }

    @Test internal func theLiveViewStreamsFramesAndGoneUntilRefused() async throws {
        let stream =
            "event: frame\ndata: {\"url\":\"https://a.com/\",\"title\":\"A\",\"jpeg\":\"anBlZw==\"}\n\n"
            + ": keep-alive\n\nevent: gone\ndata: {\"why\":\"ann's browser isn't running\"}\n\n"
        let server = CannedServer([
            "/personas/ann/screen/live": [Answer(headers: sse, chunks: [stream]), Answer(status: 403, chunks: [])]
        ])
        var got: [LiveEvent] = []
        await #expect(throws: APIError(status: 403, text: "")) {
            for try await event in server.client().live("ann") { got.append(event) }
        }
        #expect(
            got == [
                .frame(LiveFrame(url: "https://a.com/", title: "A", jpeg: Data("jpeg".utf8))),
                .gone("ann's browser isn't running"),
            ])
    }
}
