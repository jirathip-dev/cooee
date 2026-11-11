@testable import CooeeAPI
import CooeeCore
import Foundation
import Testing

internal struct ScreensTests {
    private static let row = """
        {"installed": [{"name": "hello", "version": "0.1.0", "commit": "abc", "url": "/r", "listed": true,
        "state": "running", "places": [{"type": "menu-row", "label": "hello"}],
        "permissions": {"places": ["menu-row"], "push": false, "actions": ["wave"], "destructive": [], "network": []}}],
        "index": [], "jobs": {}}
        """

    @Test internal func placesPluginsAndAScreenComeThroughCooee() async throws {
        let server = CannedServer([
            "/personas/cond/places": [
                Answer(headers: json, chunks: [#"[{"plugin":"d","id":"q","label":"Queue","screen":"/screens/queue"}]"#])
            ],
            "/plugins": [Answer(headers: json, chunks: [Self.row])],
            "/p/d/screens/queue": [Answer(headers: json, chunks: [#"{"type":"screen","title":"Queue"}"#])],
        ])
        let client = server.client()
        #expect(
            try await client.places("cond") == [Place(plugin: "d", key: "q", label: "Queue", screen: "/screens/queue")])
        #expect(try await client.plugins().map(\.permissions.actions) == [["wave"]])
        #expect(try await client.screen("d", "/screens/queue").string("title") == "Queue")
    }

    @Test internal func anActionPostsTheScreenTheActionAndItsValuesAndGetsOps() async throws {
        let ops = #"{"ops":[{"op":"merge","id":"waves","fields":{"value":"1"}}]}"#
        let server = CannedServer(["/p/hello/action": [Answer(headers: json, chunks: [ops])]])
        let action = BlockAction(action: "wave", values: ["n": .number(2)])
        let answer = try await server.client().act("hello", screen: "hello", action)
        #expect(answer["ops"]?.items.count == 1)
        let sent = try server.decoded("/p/hello/action", as: JSONValue.self)
        #expect(
            sent.string("screen") == "hello" && sent.string("action") == "wave" && sent["values"]?["n"] == .number(2))
    }

    @Test internal func aRefusedActionThrowsItsStatus() async throws {
        let server = CannedServer(["/p/hello/action": [Answer(status: 403, chunks: ["no"])]])
        await #expect(throws: APIError(status: 403, text: "")) {
            try await server.client().act("hello", screen: "", BlockAction(action: "drop", values: [:]))
        }
    }

    @Test internal func attachingAddsThePluginToThePersonaOnce() async throws {
        let server = CannedServer([
            "/personas/cond": [
                Answer(headers: json, chunks: [#"{"id":"cond","name":"Conductor","plugins":["a"]}"#]),
                Answer(headers: json, chunks: [#"{"id":"cond","name":"Conductor","plugins":["a","d"]}"#]),
            ]
        ])
        let client = server.client()
        try await client.attach("d", to: "cond")
        try await client.attach("d", to: "cond")
        let methods = server.requests("/personas/cond").map(\.httpMethod)
        #expect(methods == ["GET", "PUT", "GET"])
        let put = try #require(server.requests("/personas/cond")[1].httpBody)
        let sent = try JSONDecoder().decode(JSONValue.self, from: put)
        #expect(sent["plugins"] == .array([.string("a"), .string("d")]) && sent.string("name") == "Conductor")
    }

    @Test internal func aScreenStreamYieldsEachEventsOps() async throws {
        let server = CannedServer([
            "/p/d/screens/queue/stream": [
                Answer(
                    headers: sse,
                    chunks: ["id: 1\ndata: {\"seq\":1,\"ops\":[]}\n\n: keep-alive\n\nid: 2\n", "data: {\"seq\":2}\n\n"]
                )
            ],
            "/p/d/screens/gone": [Answer(status: 404, chunks: [])],
        ])
        var seqs: [Double] = []
        for try await event in server.client().screenEvents("d", "/screens/queue/stream") {
            seqs.append(event["seq"]?.number ?? 0)
        }
        #expect(seqs == [1, 2])
        await #expect(throws: APIError(status: 404, text: "")) {
            for try await _ in server.client().screenEvents("d", "/screens/gone") {}
        }
    }

    @Test internal func aKeptScreenIsReadBackByItsKey() throws {
        let folder = FileManager.default.temporaryDirectory.appending(path: "kept-\(UUID().uuidString)")
        let kept = KeptScreen(screen: .object(["title": .string("Queue")]), kept: Date(timeIntervalSince1970: 60))
        let screens = KeptScreens(folder: folder)
        #expect(screens.load("https://mini|d|/screens/queue") == nil)
        screens.save("https://mini|d|/screens/queue", kept)
        #expect(screens.load("https://mini|d|/screens/queue") == kept)
        #expect(screens.load("https://mini|d|/screens/events") == nil)
        try FileManager.default.removeItem(at: folder)
    }
}
